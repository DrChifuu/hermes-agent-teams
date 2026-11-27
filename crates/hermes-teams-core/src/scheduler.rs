use crate::dag::{is_task_ready, unsatisfied_dependencies};
use crate::error::{Result, TeamsError};
use crate::types::{
    gen_token, now_ms, EngineDirective, MemberStatus, TaskStatus, TeamState, TeamTask,
};

/// Result of a scheduling pass: newly claimed `(member_name, task_id, attempt_id)` triples
/// plus runtime directives for the Hermes subagent bridge.
#[derive(Debug, Clone, Default)]
pub struct ScheduleOutcome {
    pub claimed: Vec<(String, String, String)>,
    pub directives: Vec<EngineDirective>,
}

/// Return the currently open (`claimed` or `in_progress`) task owned by `member_name`, if any.
pub fn active_task_of<'a>(team: &'a TeamState, member_name: &str) -> Option<&'a TeamTask> {
    team.tasks
        .iter()
        .find(|t| t.assignee.as_deref() == Some(member_name) && t.status.is_active())
}

/// Atomically claim a specific task for `claimer` (`captain` or a member name).
/// Enforces:
/// - Team must be running and not halted
/// - All dependencies must be `completed`
/// - A member may not own two active tasks simultaneously
/// - Increments monotonic `attempt` and issues a fresh unique `attempt_id`
pub fn claim_task(
    team: &mut TeamState,
    task_id: &str,
    claimer: &str,
) -> Result<(TeamTask, Vec<EngineDirective>)> {
    if team.is_staged() {
        return Err(TeamsError::TeamStaged(team.id.clone()));
    }
    if team.is_halted() {
        return Err(TeamsError::TeamHalted(team.id.clone()));
    }

    let task_snapshot = team
        .find_task(task_id)
        .ok_or_else(|| TeamsError::TaskNotFound(task_id.to_string()))?
        .clone();

    if task_snapshot.status.is_terminal() {
        return Err(TeamsError::TaskTerminal(
            task_id.to_string(),
            task_snapshot.status.to_string(),
        ));
    }
    if task_snapshot.status != TaskStatus::Pending {
        return Err(TeamsError::InvalidInput(format!(
            "task '{task_id}' is already {}",
            task_snapshot.status
        )));
    }
    if task_snapshot.reassigning.unwrap_or(false) {
        return Err(TeamsError::InvalidInput(format!(
            "task '{task_id}' is currently quiescing a previous owner"
        )));
    }

    if claimer != "captain" {
        let member = team
            .find_member(claimer)
            .ok_or_else(|| TeamsError::MemberNotFound(claimer.to_string()))?;
        if member.status == MemberStatus::Removed || member.stopping.unwrap_or(false) {
            return Err(TeamsError::PermissionDenied(format!(
                "member '{claimer}' is removed or stopping"
            )));
        }
        if let Some(assignee) = task_snapshot.assignee.as_deref() {
            if assignee != claimer {
                return Err(TeamsError::PermissionDenied(format!(
                    "task '{task_id}' is assigned to '{assignee}', not '{claimer}'"
                )));
            }
        }
        if let Some(existing) = active_task_of(team, claimer) {
            return Err(TeamsError::MemberBusy {
                member: claimer.to_string(),
                task_id: existing.id.clone(),
            });
        }
    }

    let missing = unsatisfied_dependencies(&team.tasks, &task_snapshot);
    if !missing.is_empty() {
        return Err(TeamsError::UnsatisfiedDependencies {
            task_id: task_id.to_string(),
            missing,
        });
    }

    let next_attempt = task_snapshot.attempt.unwrap_or(0) + 1;
    let attempt_id = gen_token("att", u64::from(next_attempt));
    let now = now_ms();

    let task = team
        .find_task_mut(task_id)
        .ok_or_else(|| TeamsError::TaskNotFound(task_id.to_string()))?;
    task.status = TaskStatus::Claimed;
    task.assignee = Some(claimer.to_string());
    task.attempt = Some(next_attempt);
    task.attempt_id = Some(attempt_id.clone());
    task.updated_at = now;
    let claimed_task = task.clone();

    let mut directives = Vec::new();
    if claimer != "captain" {
        let prompt = build_task_dispatch_prompt(team, &claimed_task, &attempt_id);
        let team_id = team.id.clone();
        let captain_session_id = team.captain_session_id.clone();
        if let Some(member) = team.find_member_mut(claimer) {
            member.status = MemberStatus::Working;
            let role = member
                .role
                .clone()
                .unwrap_or_else(|| "teammate".to_string());
            if member.id.is_empty() {
                // Lazy spawn: launch member directly with its first claimed task & attempt_id!
                directives.push(EngineDirective::SpawnMember {
                    team_id,
                    captain_session_id,
                    member_name: member.name.clone(),
                    role,
                    provider: member.provider.clone(),
                    model: member.model.clone(),
                    reasoning_effort: member.reasoning_effort.clone(),
                    execution_prompt: member.execution_prompt.clone(),
                    tool_filter: member.tool_filter.clone(),
                    task_id: Some(claimed_task.id.clone()),
                    attempt_id: Some(attempt_id),
                    initial_prompt: prompt,
                });
            } else {
                directives.push(EngineDirective::WakeMember {
                    team_id,
                    captain_session_id,
                    member_name: member.name.clone(),
                    role,
                    provider: member.provider.clone(),
                    model: member.model.clone(),
                    reasoning_effort: member.reasoning_effort.clone(),
                    execution_prompt: member.execution_prompt.clone(),
                    tool_filter: member.tool_filter.clone(),
                    subagent_id: member.id.clone(),
                    task_id: Some(claimed_task.id.clone()),
                    attempt_id: Some(attempt_id),
                    message: prompt,
                });
            }
        }
    }

    Ok((claimed_task, directives))
}

/// Automatically claim the next ready task for every idle, unstopped member with no open task.
/// Unspawned members in a running team are spawned lazily carrying their first ready task.
pub fn schedule_ready_tasks(team: &mut TeamState) -> ScheduleOutcome {
    let mut outcome = ScheduleOutcome::default();
    if team.is_staged() || team.is_halted() {
        return outcome;
    }

    let idle_members: Vec<String> = team
        .members
        .iter()
        .filter(|m| {
            m.status == MemberStatus::Idle
                && !m.stopping.unwrap_or(false)
                && active_task_of(team, &m.name).is_none()
        })
        .map(|m| m.name.clone())
        .collect();

    for member_name in idle_members {
        // Prefer explicitly assigned ready tasks, then unassigned ready tasks.
        let next_task_id = team
            .tasks
            .iter()
            .find(|t| {
                t.assignee.as_deref() == Some(member_name.as_str()) && is_task_ready(&team.tasks, t)
            })
            .or_else(|| {
                team.tasks
                    .iter()
                    .find(|t| t.assignee.is_none() && is_task_ready(&team.tasks, t))
            })
            .map(|t| t.id.clone());

        if let Some(tid) = next_task_id {
            if let Ok((task, dirs)) = claim_task(team, &tid, &member_name) {
                let aid = task.attempt_id.unwrap_or_default();
                outcome.claimed.push((member_name, tid, aid));
                outcome.directives.extend(dirs);
            }
        }
    }

    outcome
}

/// Cold-start recovery: find stranded open (`claimed`/`in_progress`) tasks after a daemon restart,
/// issue fresh attempts, and emit wake directives so they resume cleanly.
pub fn recover_stranded_tasks(team: &mut TeamState) -> Vec<EngineDirective> {
    let mut directives = Vec::new();
    if team.is_staged() || team.is_halted() {
        return directives;
    }

    let open_ids: Vec<String> = team
        .tasks
        .iter()
        .filter(|t| t.status.is_active() && !t.reassigning.unwrap_or(false))
        .map(|t| t.id.clone())
        .collect();

    let now = now_ms();
    for tid in open_ids {
        let assignee = team
            .find_task(&tid)
            .and_then(|t| t.assignee.clone())
            .unwrap_or_default();
        if assignee.is_empty() || assignee == "captain" {
            continue;
        }
        let member_info = team
            .find_member(&assignee)
            .filter(|m| m.status != MemberStatus::Removed && !m.id.is_empty())
            .cloned();
        let Some(member) = member_info else {
            continue;
        };

        let next_attempt = team.find_task(&tid).and_then(|t| t.attempt).unwrap_or(0) + 1;
        let new_attempt_id = gen_token("att-rec", u64::from(next_attempt));

        let team_id = team.id.clone();
        let captain_session_id = team.captain_session_id.clone();
        if let Some(task) = team.find_task_mut(&tid) {
            task.attempt = Some(next_attempt);
            task.attempt_id = Some(new_attempt_id.clone());
            task.updated_at = now;
            let snapshot = task.clone();
            let prompt = build_task_dispatch_prompt(team, &snapshot, &new_attempt_id);
            directives.push(EngineDirective::WakeMember {
                team_id,
                captain_session_id,
                member_name: assignee,
                role: member.role.unwrap_or_else(|| "teammate".to_string()),
                provider: member.provider,
                model: member.model,
                reasoning_effort: member.reasoning_effort,
                execution_prompt: member.execution_prompt,
                tool_filter: member.tool_filter,
                subagent_id: member.id,
                task_id: Some(tid),
                attempt_id: Some(new_attempt_id),
                message: format!("[Cold recovery retry]\n{prompt}"),
            });
        }
    }

    directives
}

/// Build the structured task dispatch prompt injected into a member's turn, including
/// completed upstream dependency outputs and quality-gate contract fields.
pub fn build_task_dispatch_prompt(team: &TeamState, task: &TeamTask, attempt_id: &str) -> String {
    let mut lines = vec![
        format!("Assigned Team Task: {} — {}", task.id, task.subject),
        format!("Team: {} ({})", team.name, team.id),
        format!("Attempt ID: {attempt_id} (required on agent_teams_update_task)"),
    ];
    if let Some(desc) = &task.description {
        if !desc.trim().is_empty() {
            lines.push(format!("Description:\n{desc}"));
        }
    }
    if let Some(obj) = &task.objective {
        lines.push(format!("Objective: {obj}"));
    }
    if let Some(scope) = &task.in_scope {
        lines.push(format!("In Scope: {}", scope.join(", ")));
    }
    if let Some(out_scope) = &task.out_of_scope {
        lines.push(format!("Out of Scope: {}", out_scope.join(", ")));
    }
    if let Some(acc) = &task.acceptance {
        lines.push(format!("Acceptance Criteria:\n- {}", acc.join("\n- ")));
    }
    if let Some(ver) = &task.verify {
        lines.push(format!("Verify Commands:\n- {}", ver.join("\n- ")));
    }

    // Pass outputs from completed upstream dependencies.
    let mut dep_blocks = Vec::new();
    for dep_id in &task.dependencies {
        if let Some(dep) = team.find_task(dep_id) {
            if let Some(out) = &dep.output {
                dep_blocks.push(format!("- {} ({}): {}", dep.id, dep.subject, out));
            }
        }
    }
    if !dep_blocks.is_empty() {
        lines.push(format!(
            "Upstream Dependency Outputs:\n{}",
            dep_blocks.join("\n")
        ));
    }

    lines.push(
        "Update progress with `agent_teams_update_task` passing this `attempt_id`, and notify teammates or `captain` via `agent_teams_send_message` when done."
            .to_string(),
    );

    lines.join("\n\n")
}
