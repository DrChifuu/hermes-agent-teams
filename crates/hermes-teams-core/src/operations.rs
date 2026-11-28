use crate::dag::{direct_unlocks, unsatisfied_dependencies, validate_dependencies};
use crate::error::{Result, TeamsError};
use crate::mailbox::{discard_for_attempt, send_message, SendMessageOptions};
use crate::policies::TeamsConfig;
use crate::quality_gates::{
    maybe_spawn_repair_loop, snapshot_artifacts, validate_quality_transition,
    validate_task_creation_contract, QualityUpdateCheck,
};
use crate::scheduler::{claim_task, schedule_ready_tasks};
use crate::store::TeamStore;
use crate::types::{
    gen_token, now_ms, sanitize_id, AcceptanceResult, CommandResult, EngineDirective, MemberStatus,
    PlanReviewState, ReviewFinding, ReviewPolicy, ReviewVerdict, TaskKind, TaskPlanningMode,
    TaskRevision, TaskStatus, TeamMember, TeamMessage, TeamPhase, TeamProfileSnapshot, TeamState,
    TeamTask,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

/// Standard result envelope returned by all 14 Agent Teams operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OperationOutput {
    pub team: TeamState,
    pub summary: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub directives: Vec<EngineDirective>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub extra: BTreeMap<String, serde_json::Value>,
}

/// Inline member row when creating or editing a staged plan.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanMemberInput {
    pub name: String,
    #[serde(default)]
    pub role: Option<String>,
    #[serde(default)]
    pub provider: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub reasoning_effort: Option<String>,
    #[serde(default)]
    pub execution_prompt: Option<String>,
    #[serde(default)]
    pub tool_policy: Option<String>,
}

/// Inline task row when creating or editing a staged plan.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanTaskInput {
    #[serde(default)]
    pub id: Option<String>,
    pub subject: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub assignee: Option<String>,
    #[serde(default)]
    pub dependencies: Vec<String>,
    #[serde(default)]
    pub kind: Option<TaskKind>,
    #[serde(default)]
    pub objective: Option<String>,
    #[serde(default)]
    pub in_scope: Option<Vec<String>>,
    #[serde(default)]
    pub out_of_scope: Option<Vec<String>>,
    #[serde(default)]
    pub acceptance: Option<Vec<String>>,
    #[serde(default)]
    pub verify: Option<Vec<String>>,
    #[serde(default)]
    pub reviewed_task_id: Option<String>,
    #[serde(default)]
    pub coverage_of: Option<Vec<String>>,
}

/// Input for `agent_teams_create`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateTeamInput {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    pub captain_session_id: String,
    #[serde(default)]
    pub profile: Option<String>,
    /// `"manual"` (default, staged plan for human review) or `"automatic"`.
    #[serde(default)]
    pub approval: Option<String>,
    #[serde(default)]
    pub members: Vec<PlanMemberInput>,
    #[serde(default)]
    pub tasks: Vec<PlanTaskInput>,
    #[serde(default)]
    pub review_policy: Option<ReviewPolicy>,
    #[serde(default)]
    pub captain_provider: Option<String>,
    #[serde(default)]
    pub captain_model: Option<String>,
    #[serde(default)]
    pub captain_reasoning_effort: Option<String>,
}

/// Input for `agent_teams_edit_plan`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EditPlanInput {
    pub team_id: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub members: Option<Vec<PlanMemberInput>>,
    #[serde(default)]
    pub tasks: Option<Vec<PlanTaskInput>>,
    #[serde(default)]
    pub captain_provider: Option<String>,
    #[serde(default)]
    pub captain_model: Option<String>,
    #[serde(default)]
    pub captain_reasoning_effort: Option<String>,
}

/// Input for `agent_teams_add_member`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddMemberInput {
    pub team_id: String,
    pub name: String,
    #[serde(default)]
    pub role: Option<String>,
    #[serde(default)]
    pub provider: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub reasoning_effort: Option<String>,
    #[serde(default)]
    pub execution_prompt: Option<String>,
    #[serde(default)]
    pub tool_policy: Option<String>,
    #[serde(default)]
    pub captain_provider: Option<String>,
    #[serde(default)]
    pub captain_model: Option<String>,
    #[serde(default)]
    pub captain_reasoning_effort: Option<String>,
}

/// Input for updating an existing team member in-place (model, provider, reasoning effort, role, policy).
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateMemberInput {
    pub team_id: String,
    pub name: String,
    #[serde(default)]
    pub role: Option<String>,
    #[serde(default)]
    pub provider: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub reasoning_effort: Option<String>,
    #[serde(default)]
    pub tool_policy: Option<String>,
}

/// Input for `agent_teams_create_task`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateTaskInput {
    pub team_id: String,
    pub subject: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub assignee: Option<String>,
    #[serde(default)]
    pub dependencies: Vec<String>,
    #[serde(default)]
    pub kind: Option<TaskKind>,
    #[serde(default)]
    pub objective: Option<String>,
    #[serde(default)]
    pub in_scope: Option<Vec<String>>,
    #[serde(default)]
    pub out_of_scope: Option<Vec<String>>,
    #[serde(default)]
    pub acceptance: Option<Vec<String>>,
    #[serde(default)]
    pub verify: Option<Vec<String>>,
    #[serde(default)]
    pub deliverables: Option<Vec<String>>,
    #[serde(default)]
    pub non_goals: Option<Vec<String>>,
    #[serde(default)]
    pub reviewed_task_id: Option<String>,
    #[serde(default)]
    pub coverage_of: Option<Vec<String>>,
    #[serde(default)]
    pub resume: Option<bool>,
    #[serde(default)]
    pub resume_reason: Option<String>,
}

/// Input for `agent_teams_update_task`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateTaskInput {
    pub team_id: String,
    pub task_id: String,
    pub actor: String,
    #[serde(default)]
    pub attempt_id: Option<String>,
    pub status: TaskStatus,
    #[serde(default)]
    pub output: Option<String>,
    #[serde(default)]
    pub verdict: Option<ReviewVerdict>,
    #[serde(default)]
    pub findings: Option<Vec<ReviewFinding>>,
    #[serde(default)]
    pub changed_paths: Option<Vec<String>>,
    #[serde(default)]
    pub acceptance_results: Option<Vec<AcceptanceResult>>,
    #[serde(default)]
    pub commands_run: Option<Vec<CommandResult>>,
}

/// Input for `agent_teams_amend_task` (captain-only quality contract amendment).
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AmendTaskInput {
    pub team_id: String,
    pub task_id: String,
    pub reason: String,
    #[serde(default)]
    pub objective: Option<String>,
    #[serde(default)]
    pub acceptance: Option<Vec<String>>,
    #[serde(default)]
    pub verify: Option<Vec<String>>,
    #[serde(default)]
    pub in_scope: Option<Vec<String>>,
    #[serde(default)]
    pub out_of_scope: Option<Vec<String>>,
}

/// Input for `agent_teams_reassign_task`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReassignTaskInput {
    pub team_id: String,
    pub task_id: String,
    pub assignee: String,
    #[serde(default)]
    pub reason: Option<String>,
}

/// Input for `agent_teams_send_message`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SendMessageInput {
    pub team_id: String,
    pub from: String,
    pub to: String,
    pub content: String,
    #[serde(default)]
    pub task_id: Option<String>,
    #[serde(default)]
    pub attempt_id: Option<String>,
}

/// Stateless operations facade backed by `TeamStore` + `TeamsConfig`.
#[derive(Debug, Clone)]
pub struct TeamEngine {
    store: TeamStore,
    config: TeamsConfig,
}

impl TeamEngine {
    pub fn new(store: TeamStore, config: TeamsConfig) -> Self {
        Self { store, config }
    }

    pub fn store(&self) -> &TeamStore {
        &self.store
    }

    pub fn config(&self) -> &TeamsConfig {
        &self.config
    }

    /// 1. `agent_teams_create`: create a staged (default) or running team for a captain session.
    pub fn create_team(&self, input: CreateTeamInput) -> Result<OperationOutput> {
        let name = input.name.trim();
        let captain = input.captain_session_id.trim();
        if name.is_empty() || captain.is_empty() {
            return Err(TeamsError::InvalidInput(
                "name and captainSessionId are required".to_string(),
            ));
        }

        if let Some(existing) = self.store.find_by_captain(captain)? {
            return Err(TeamsError::CaptainAlreadyHasTeam {
                session_id: captain.to_string(),
                team_id: existing.id,
            });
        }

        let team_id = sanitize_id(name);
        if self.store.team_file(&team_id).is_file() {
            return Err(TeamsError::TeamAlreadyExists(team_id));
        }

        let automatic = input.approval.as_deref() == Some("automatic");
        let now = now_ms();

        let mut profile_snapshot = None;
        let mut seed_members = input.members;
        let mut seed_tasks = input.tasks;
        let mut review_policy = input.review_policy;

        if let Some(profile_name) = input
            .profile
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            let prof = self.config.profiles.get(profile_name).ok_or_else(|| {
                TeamsError::InvalidInput(format!("unknown team profile '{profile_name}'"))
            })?;
            profile_snapshot = Some(TeamProfileSnapshot {
                name: profile_name.to_string(),
                description: prof.description.clone(),
                protocol: prof.protocol.clone(),
                execution_prompt: prof.execution_prompt.clone(),
                fallback: prof.fallback.clone(),
                task_planning: prof.task_planning,
                review_policy: prof.review_policy.clone(),
            });
            if review_policy.is_none() {
                review_policy.clone_from(&prof.review_policy);
            }
            if seed_members.is_empty() {
                for m in &prof.members {
                    seed_members.push(PlanMemberInput {
                        name: m.name.clone(),
                        role: m.role.clone(),
                        provider: m.provider.clone(),
                        model: m.model.clone(),
                        reasoning_effort: m.reasoning_effort.clone(),
                        execution_prompt: m.execution_prompt.clone(),
                        tool_policy: m.tool_policy.clone(),
                    });
                }
            }
            let is_seed_mode =
                prof.task_planning.unwrap_or(TaskPlanningMode::Seed) == TaskPlanningMode::Seed;
            if is_seed_mode && seed_tasks.is_empty() {
                for t in &prof.tasks {
                    seed_tasks.push(PlanTaskInput {
                        id: Some(t.id.clone()),
                        subject: t.subject.clone(),
                        description: t.description.clone(),
                        assignee: t.assignee.clone(),
                        dependencies: t.dependencies.clone(),
                        kind: t.kind,
                        objective: t.objective.clone(),
                        in_scope: t.in_scope.clone(),
                        out_of_scope: t.out_of_scope.clone(),
                        acceptance: t.acceptance.clone(),
                        verify: t.verify.clone(),
                        reviewed_task_id: None,
                        coverage_of: None,
                    });
                }
            }
        }

        let mut team = TeamState {
            name: name.to_string(),
            id: team_id,
            description: input.description,
            profile: profile_snapshot,
            captain_session_id: captain.to_string(),
            captain_provider: input.captain_provider.clone(),
            captain_model: input.captain_model.clone(),
            captain_reasoning_effort: input.captain_reasoning_effort.clone(),
            created_at: now,
            members: Vec::new(),
            tasks: Vec::new(),
            task_seq: 0,
            phase: Some(if automatic {
                TeamPhase::Running
            } else {
                TeamPhase::Staged
            }),
            plan_review_state: if automatic {
                None
            } else {
                Some(PlanReviewState::AwaitingReview)
            },
            approved_at: if automatic { Some(now) } else { None },
            halted: None,
            halted_at: None,
            halt_reason: None,
            review_policy,
            escalated: None,
            resolution: None,
        };

        self.apply_plan_members(
            &mut team,
            seed_members,
            input.captain_provider.as_deref(),
            input.captain_model.as_deref(),
            input.captain_reasoning_effort.as_deref(),
        )?;
        Self::apply_plan_tasks(&mut team, seed_tasks)?;

        let mut directives = Vec::new();
        if automatic {
            let sched = schedule_ready_tasks(&mut team);
            directives.extend(sched.directives);
        }

        self.store.save_team(&team)?;

        let summary = if automatic {
            format!(
                "Team '{}' ({}) created and running with {} members and {} tasks.",
                team.name,
                team.id,
                team.members.len(),
                team.tasks.len()
            )
        } else {
            format!(
                "Staged team plan '{}' ({}) created with {} members and {} tasks. Awaiting Approve & Run.",
                team.name,
                team.id,
                team.members.len(),
                team.tasks.len()
            )
        };

        Ok(OperationOutput {
            team,
            summary,
            directives,
            extra: BTreeMap::new(),
        })
    }

    /// 2. `agent_teams_edit_plan`: atomically edit a staged plan without creating a new team.
    pub fn edit_plan(&self, input: EditPlanInput) -> Result<OperationOutput> {
        let mut team = self.store.load_team(&input.team_id)?;
        if !team.is_staged() {
            return Err(TeamsError::InvalidInput(format!(
                "team '{}' is already running; edit_plan is only valid while staged",
                team.id
            )));
        }

        if let Some(desc) = input.description {
            team.description = Some(desc);
        }
        if team.captain_provider.is_none() {
            if let Some(ref cp) = input.captain_provider {
                if !cp.trim().is_empty() {
                    team.captain_provider = Some(cp.clone());
                }
            }
        }
        if team.captain_model.is_none() {
            if let Some(ref cm) = input.captain_model {
                if !cm.trim().is_empty() {
                    team.captain_model = Some(cm.clone());
                }
            }
        }
        if team.captain_reasoning_effort.is_none() {
            if let Some(ref ce) = input.captain_reasoning_effort {
                if !ce.trim().is_empty() {
                    team.captain_reasoning_effort = Some(ce.clone());
                }
            }
        }
        if let Some(mut members) = input.members {
            let prev_prompts: BTreeMap<String, String> = team
                .members
                .iter()
                .filter_map(|m| {
                    m.execution_prompt
                        .as_ref()
                        .map(|ep| (m.name.clone(), ep.clone()))
                })
                .collect();
            for m in &mut members {
                if m.execution_prompt.is_none() {
                    if let Some(ep) = prev_prompts.get(m.name.trim()) {
                        m.execution_prompt = Some(ep.clone());
                    }
                }
            }
            team.members.clear();
            let eff_provider = input
                .captain_provider
                .as_deref()
                .or(team.captain_provider.as_deref());
            let eff_model = input
                .captain_model
                .as_deref()
                .or(team.captain_model.as_deref());
            let eff_effort = input
                .captain_reasoning_effort
                .as_deref()
                .or(team.captain_reasoning_effort.as_deref());
            let eff_provider_owned = eff_provider.map(ToString::to_string);
            let eff_model_owned = eff_model.map(ToString::to_string);
            let eff_effort_owned = eff_effort.map(ToString::to_string);
            self.apply_plan_members(
                &mut team,
                members,
                eff_provider_owned.as_deref(),
                eff_model_owned.as_deref(),
                eff_effort_owned.as_deref(),
            )?;
        }
        if let Some(mut tasks) = input.tasks {
            let prev_tasks: BTreeMap<String, TeamTask> = team
                .tasks
                .iter()
                .map(|t| (t.id.clone(), t.clone()))
                .collect();
            for row in &mut tasks {
                if let Some(prev) = row
                    .id
                    .as_deref()
                    .map(str::trim)
                    .and_then(|id| prev_tasks.get(id))
                {
                    if row.description.is_none() {
                        row.description = prev.description.clone();
                    }
                    if row.objective.is_none() {
                        row.objective = prev.objective.clone();
                    }
                    if row.in_scope.is_none() {
                        row.in_scope = prev.in_scope.clone();
                    }
                    if row.out_of_scope.is_none() {
                        row.out_of_scope = prev.out_of_scope.clone();
                    }
                    if row.acceptance.is_none() {
                        row.acceptance = prev.acceptance.clone();
                    }
                    if row.verify.is_none() {
                        row.verify = prev.verify.clone();
                    }
                    if row.reviewed_task_id.is_none() {
                        row.reviewed_task_id = prev.reviewed_task_id.clone();
                    }
                    if row.coverage_of.is_none() {
                        row.coverage_of = prev.coverage_of.clone();
                    }
                }
            }
            team.tasks.clear();
            team.task_seq = 0;
            Self::apply_plan_tasks(&mut team, tasks)?;
        }
        team.plan_review_state = Some(PlanReviewState::AwaitingReview);
        self.store.save_team(&team)?;

        Ok(OperationOutput {
            summary: format!(
                "Updated staged plan '{}' ({} members, {} tasks).",
                team.id,
                team.members.len(),
                team.tasks.len()
            ),
            team,
            directives: Vec::new(),
            extra: BTreeMap::new(),
        })
    }

    /// Mark a staged plan as waiting for user chat feedback ("Return to chat to revise").
    pub fn request_plan_feedback(&self, team_id: &str) -> Result<OperationOutput> {
        let mut team = self.store.load_team(team_id)?;
        if !team.is_staged() {
            return Err(TeamsError::InvalidInput(format!(
                "team '{team_id}' is not staged"
            )));
        }
        team.plan_review_state = Some(PlanReviewState::AwaitingFeedback);
        self.store.save_team(&team)?;

        let directives = vec![EngineDirective::WakeCaptain {
            team_id: team.id.clone(),
            captain_session_id: team.captain_session_id.clone(),
            message: format!(
                "User returned to chat to revise staged team plan '{}'. Ask concisely what they want to change, then update this same draft via agent_teams_edit_plan.",
                team.name
            ),
        }];

        Ok(OperationOutput {
            summary: format!("Plan '{}' marked awaiting_feedback.", team.id),
            team,
            directives,
            extra: BTreeMap::new(),
        })
    }

    /// 3. `agent_teams_approve`: Approve & Run a staged plan, spawning all members and waking captain.
    pub fn approve_team(&self, team_id: &str) -> Result<OperationOutput> {
        let mut team = self.store.load_team(team_id)?;
        if !team.is_staged() {
            return Err(TeamsError::InvalidInput(format!(
                "team '{team_id}' is already approved and running"
            )));
        }

        let now = now_ms();
        team.phase = Some(TeamPhase::Running);
        team.plan_review_state = None;
        team.approved_at = Some(now);

        // Claim ready tasks immediately so members spawn directly with their first task + attempt_id!
        let sched = schedule_ready_tasks(&mut team);
        let mut directives = sched.directives;
        if team.tasks.is_empty() {
            directives.extend(build_spawn_directives(&team));
        }
        directives.push(EngineDirective::WakeCaptain {
            team_id: team.id.clone(),
            captain_session_id: team.captain_session_id.clone(),
            message: format!(
                "Team '{}' approved and started with {} members and {} tasks ({} ready tasks dispatched). Monitor progress with agent_teams_status.",
                team.name,
                team.members.len(),
                team.tasks.len(),
                sched.claimed.len()
            ),
        });

        self.store.save_team(&team)?;

        Ok(OperationOutput {
            summary: format!(
                "Approved team '{}': dispatched {} ready tasks.",
                team.id,
                sched.claimed.len()
            ),
            team,
            directives,
            extra: BTreeMap::new(),
        })
    }

    /// Register the real Hermes subagent session ID after the Python bridge spawns a member.
    /// Preserves `Working` status if the member already owns an active claimed task.
    pub fn bind_member_session(
        &self,
        team_id: &str,
        member_name: &str,
        subagent_id: &str,
        spawn_error: Option<String>,
    ) -> Result<OperationOutput> {
        let mut team = self.store.load_team(team_id)?;
        let has_active_task = team
            .tasks
            .iter()
            .any(|t| t.assignee.as_deref() == Some(member_name) && t.status.is_active());
        let member = team
            .find_member_mut(member_name)
            .ok_or_else(|| TeamsError::MemberNotFound(member_name.to_string()))?;

        if let Some(err) = spawn_error {
            member.spawn_error = Some(err);
            member.status = MemberStatus::Idle;
            // Release any active task back to pending if initial spawn failed.
            for t in &mut team.tasks {
                if t.assignee.as_deref() == Some(member_name) && t.status.is_active() {
                    t.status = TaskStatus::Pending;
                    t.attempt_id = None;
                    t.updated_at = now_ms();
                }
            }
            self.store.save_team(&team)?;
            return Ok(OperationOutput {
                summary: format!("Recorded spawn error for member '{member_name}'."),
                team,
                directives: Vec::new(),
                extra: BTreeMap::new(),
            });
        }

        member.id = subagent_id.trim().to_string();
        member.spawn_error = None;
        member.status = if has_active_task {
            MemberStatus::Working
        } else {
            MemberStatus::Idle
        };

        let sched = schedule_ready_tasks(&mut team);
        self.store.save_team(&team)?;

        Ok(OperationOutput {
            summary: format!(
                "Bound member '{member_name}' to subagent '{subagent_id}'; dispatched {} additional tasks.",
                sched.claimed.len()
            ),
            team,
            directives: sched.directives,
            extra: BTreeMap::new(),
        })
    }

    /// 4. `agent_teams_add_member`: add a member (placeholder if staged, spawned immediately if running).
    pub fn add_member(&self, input: AddMemberInput) -> Result<OperationOutput> {
        let mut team = self.store.load_team(&input.team_id)?;
        let name = input.name.trim();
        if name.is_empty() || name.eq_ignore_ascii_case("captain") {
            return Err(TeamsError::InvalidInput(
                "member name must be non-empty and cannot be 'captain'".to_string(),
            ));
        }
        if team.find_member(name).is_some() {
            // If the team is paused/halted or the existing member is idle/errored, update it
            // in-place instead of forcing the caller to create duplicate mini-agents.
            let can_update_in_place = team.is_halted()
                || team.is_staged()
                || team
                    .find_member(name)
                    .map(|m| m.status != MemberStatus::Working || m.spawn_error.is_some())
                    .unwrap_or(false);
            if can_update_in_place {
                return self.update_member(UpdateMemberInput {
                    team_id: input.team_id,
                    name: name.to_string(),
                    role: input.role,
                    provider: input.provider,
                    model: input.model,
                    reasoning_effort: input.reasoning_effort,
                    tool_policy: input.tool_policy,
                });
            }
            return Err(TeamsError::MemberAlreadyExists(name.to_string()));
        }
        if team.active_members_count() >= self.config.max_members {
            return Err(TeamsError::MaxMembersReached(self.config.max_members));
        }

        let member = self.build_member_record(
            PlanMemberInput {
                name: name.to_string(),
                role: input.role,
                provider: input.provider,
                model: input.model,
                reasoning_effort: input.reasoning_effort,
                execution_prompt: input.execution_prompt,
                tool_policy: input.tool_policy,
            },
            input.captain_provider.as_deref(),
            input.captain_model.as_deref(),
            input.captain_reasoning_effort.as_deref(),
        )?;

        team.members.push(member.clone());

        let directives = if team.is_staged() {
            Vec::new()
        } else {
            schedule_ready_tasks(&mut team).directives
        };

        self.store.save_team(&team)?;

        Ok(OperationOutput {
            summary: format!("Added member '{}' to team '{}'.", member.name, team.id),
            team,
            directives,
            extra: BTreeMap::new(),
        })
    }

    /// 5. `agent_teams_remove_member`: safely remove a member, revoke active attempts, reclaim tasks, reschedule.
    pub fn remove_member(&self, team_id: &str, member_name: &str) -> Result<OperationOutput> {
        let mut team = self.store.load_team(team_id)?;
        let member = team
            .find_member_mut(member_name)
            .ok_or_else(|| TeamsError::MemberNotFound(member_name.to_string()))?;

        let subagent_id = member.id.clone();
        member.status = MemberStatus::Removed;
        member.stopping = Some(true);

        let now = now_ms();
        let mut reclaimed = Vec::new();
        for task in &mut team.tasks {
            if task.assignee.as_deref() == Some(member_name) && !task.status.is_terminal() {
                if let Some(old_aid) = task.attempt_id.as_deref() {
                    let _ = discard_for_attempt(&self.store, team_id, member_name, Some(old_aid));
                }
                task.status = TaskStatus::Pending;
                task.assignee = None;
                task.attempt_id = None;
                task.reassigning = None;
                task.updated_at = now;
                reclaimed.push(task.id.clone());
            }
        }
        let _ = discard_for_attempt(&self.store, team_id, member_name, None);

        let mut directives = Vec::new();
        if !subagent_id.is_empty() {
            directives.push(EngineDirective::CancelMember {
                team_id: team.id.clone(),
                captain_session_id: team.captain_session_id.clone(),
                member_name: member_name.to_string(),
                subagent_id,
                reason: "member removed from team".to_string(),
            });
        }

        let sched = schedule_ready_tasks(&mut team);
        directives.extend(sched.directives);
        self.store.save_team(&team)?;

        Ok(OperationOutput {
            summary: format!(
                "Removed member '{member_name}' and reclaimed {} tasks ({reclaimed:?}).",
                reclaimed.len()
            ),
            team,
            directives,
            extra: BTreeMap::new(),
        })
    }

    /// 6. `agent_teams_create_task`: add a dependency-aware task; rejects on halted team unless `resume=true`.
    pub fn create_task(&self, input: CreateTaskInput) -> Result<OperationOutput> {
        let mut team = self.store.load_team(&input.team_id)?;

        if team.is_halted() {
            if input.resume.unwrap_or(false) {
                let reason = input.resume_reason.as_deref().unwrap_or("").trim();
                if reason.is_empty() {
                    return Err(TeamsError::InvalidInput(
                        "resuming a halted team via create_task requires non-empty resumeReason"
                            .to_string(),
                    ));
                }
                team.halted = Some(false);
                team.halt_reason = None;
            } else {
                return Err(TeamsError::TeamHalted(team.id.clone()));
            }
        }

        let subject = input.subject.trim();
        if subject.is_empty() {
            return Err(TeamsError::InvalidInput(
                "task subject cannot be empty".to_string(),
            ));
        }

        if let Some(assignee) = input.assignee.as_deref() {
            if assignee != "captain" && team.find_member(assignee).is_none() {
                return Err(TeamsError::MemberNotFound(assignee.to_string()));
            }
        }

        team.task_seq += 1;
        let task_id = format!("t{}", team.task_seq);
        validate_dependencies(&team.tasks, &task_id, &input.dependencies)?;

        let now = now_ms();
        let task = TeamTask {
            profile_seed_id: None,
            id: task_id.clone(),
            subject: subject.to_string(),
            description: input.description,
            status: TaskStatus::Pending,
            assignee: input.assignee,
            dependencies: input.dependencies,
            output: None,
            attempt: None,
            attempt_id: None,
            handoff_id: None,
            handoff_from_member_id: None,
            reassigning: None,
            kind: input.kind,
            round: if input.kind.unwrap_or(TaskKind::Work).is_quality() {
                Some(1)
            } else {
                None
            },
            verdict: None,
            findings: None,
            objective: input.objective,
            in_scope: input.in_scope,
            out_of_scope: input.out_of_scope,
            acceptance: input.acceptance,
            verify: input.verify,
            deliverables: input.deliverables,
            non_goals: input.non_goals,
            changed_paths: None,
            acceptance_results: None,
            commands_run: None,
            supplemental_evidence: None,
            reviewed_task_id: input.reviewed_task_id,
            reviewed_attempt: None,
            reviewed_artifacts: None,
            reviewed_gaps: None,
            reviewed_scope: None,
            reviewed_during_drift: None,
            verdict_start_snapshot: None,
            contract_snapshot: None,
            source_task_id: None,
            finding_source_task_ids: None,
            source_finding_ids: None,
            coverage_of: input.coverage_of,
            revisions: None,
            created_at: now,
            updated_at: now,
        };

        validate_task_creation_contract(&task)?;
        team.tasks.push(task);

        let sched = schedule_ready_tasks(&mut team);
        self.store.save_team(&team)?;

        let mut extra = BTreeMap::new();
        extra.insert("taskId".to_string(), serde_json::json!(task_id));

        Ok(OperationOutput {
            summary: format!("Created task '{task_id}' in team '{}'.", team.id),
            team,
            directives: sched.directives,
            extra,
        })
    }

    /// 7. `agent_teams_reassign_task`: atomically revoke old attempt, cancel old worker, and reassign (or captain takeover).
    pub fn reassign_task(&self, input: &ReassignTaskInput) -> Result<OperationOutput> {
        let mut team = self.store.load_team(&input.team_id)?;
        let new_assignee = input.assignee.trim();
        if new_assignee != "captain" && team.find_member(new_assignee).is_none() {
            return Err(TeamsError::MemberNotFound(new_assignee.to_string()));
        }

        let old_task = team
            .find_task(&input.task_id)
            .ok_or_else(|| TeamsError::TaskNotFound(input.task_id.clone()))?
            .clone();

        if old_task.status == TaskStatus::Completed || old_task.status == TaskStatus::Cancelled {
            return Err(TeamsError::TaskTerminal(
                old_task.id,
                old_task.status.to_string(),
            ));
        }

        let team_id_for_cancel = team.id.clone();
        let captain_sid_for_cancel = team.captain_session_id.clone();
        let mut directives = Vec::new();
        if let Some(old_owner) = old_task.assignee.as_deref() {
            if old_owner != "captain" {
                if let Some(old_aid) = old_task.attempt_id.as_deref() {
                    let _ = discard_for_attempt(
                        &self.store,
                        &team_id_for_cancel,
                        old_owner,
                        Some(old_aid),
                    );
                }
                if let Some(m) = team.find_member_mut(old_owner) {
                    if !m.id.is_empty() && old_task.status.is_active() {
                        directives.push(EngineDirective::CancelMember {
                            team_id: team_id_for_cancel.clone(),
                            captain_session_id: captain_sid_for_cancel.clone(),
                            member_name: m.name.clone(),
                            subagent_id: m.id.clone(),
                            reason: input
                                .reason
                                .clone()
                                .unwrap_or_else(|| format!("task {} reassigned", input.task_id)),
                        });
                    }
                    if m.status == MemberStatus::Working {
                        m.status = MemberStatus::Idle;
                    }
                }
            }
        }

        let now = now_ms();
        let next_attempt = old_task.attempt.unwrap_or(0) + 1;
        let handoff_id = gen_token("handoff", u64::from(next_attempt));

        if let Some(task) = team.find_task_mut(&input.task_id) {
            task.status = TaskStatus::Pending;
            task.assignee = Some(new_assignee.to_string());
            task.attempt = Some(next_attempt);
            task.attempt_id = None;
            task.handoff_id = Some(handoff_id);
            task.reassigning = None;
            task.updated_at = now;
        }

        if new_assignee == "captain" {
            let (_claimed, claim_dirs) = claim_task(&mut team, &input.task_id, "captain")?;
            directives.extend(claim_dirs);
        } else {
            let sched = schedule_ready_tasks(&mut team);
            directives.extend(sched.directives);
        }

        self.store.save_team(&team)?;

        Ok(OperationOutput {
            summary: format!(
                "Reassigned task '{}' to '{new_assignee}' (revoked prior attempt).",
                input.task_id
            ),
            team,
            directives,
            extra: BTreeMap::new(),
        })
    }

    /// 8. `agent_teams_claim_task`: claim a ready task and issue a unique `attempt_id`.
    pub fn claim_task_op(
        &self,
        team_id: &str,
        task_id: &str,
        claimer: &str,
    ) -> Result<OperationOutput> {
        let mut team = self.store.load_team(team_id)?;
        let (claimed_task, directives) = claim_task(&mut team, task_id, claimer)?;
        self.store.save_team(&team)?;

        let mut extra = BTreeMap::new();
        extra.insert("taskId".to_string(), serde_json::json!(claimed_task.id));
        extra.insert(
            "attemptId".to_string(),
            serde_json::json!(claimed_task.attempt_id),
        );

        Ok(OperationOutput {
            summary: format!("Task '{task_id}' claimed by '{claimer}'."),
            team,
            directives,
            extra,
        })
    }

    /// 9. `agent_teams_update_task`: advance or finalize a task with `attempt_id` capability verification,
    ///    quality-gate enforcement, auto-repair spawning, and automatic downstream scheduling.
    pub fn update_task(
        &self,
        input: UpdateTaskInput,
        workspace: Option<&Path>,
    ) -> Result<OperationOutput> {
        let mut team = self.store.load_team(&input.team_id)?;
        let task_snapshot = team
            .find_task(&input.task_id)
            .ok_or_else(|| TeamsError::TaskNotFound(input.task_id.clone()))?
            .clone();

        if task_snapshot.status.is_terminal() {
            return Err(TeamsError::TaskTerminal(
                task_snapshot.id,
                task_snapshot.status.to_string(),
            ));
        }

        // Enforce attempt_id capability on member updates so stale/revoked attempts cannot overwrite results.
        if input.actor != "captain" {
            if task_snapshot.assignee.as_deref() != Some(input.actor.as_str()) {
                return Err(TeamsError::PermissionDenied(format!(
                    "member '{}' does not own task '{}'",
                    input.actor, input.task_id
                )));
            }
            if let Some(expected_aid) = task_snapshot.attempt_id.as_deref() {
                let actual_aid = input.attempt_id.as_deref().unwrap_or("");
                if actual_aid != expected_aid {
                    return Err(TeamsError::StaleAttempt {
                        task_id: input.task_id.clone(),
                        expected: expected_aid.to_string(),
                        actual: actual_aid.to_string(),
                    });
                }
            }
        }

        // Enforce quality-gate contracts before accepting terminal state.
        let qcheck = QualityUpdateCheck {
            verdict: input.verdict,
            findings: input.findings.as_deref(),
            changed_paths: input.changed_paths.as_deref(),
            acceptance_results: input.acceptance_results.as_deref(),
            commands_run: input.commands_run.as_deref(),
        };
        validate_quality_transition(&task_snapshot, input.status, &qcheck)?;

        // Optional SHA-256 artifact snapshot on review completion.
        let artifact_snapshot = if input.status.is_terminal() {
            if let (Some(ws), Some(paths)) = (workspace, input.changed_paths.as_deref()) {
                Some(snapshot_artifacts(ws, paths))
            } else {
                None
            }
        } else {
            None
        };

        let now = now_ms();
        let owner_name = task_snapshot.assignee.clone();
        if let Some(task) = team.find_task_mut(&input.task_id) {
            task.status = input.status;
            if let Some(out) = input.output {
                task.output = Some(out);
            }
            if let Some(v) = input.verdict {
                task.verdict = Some(v);
            }
            if let Some(f) = input.findings {
                task.findings = Some(f);
            }
            if let Some(cp) = input.changed_paths {
                task.changed_paths = Some(cp);
            }
            if let Some(ar) = input.acceptance_results {
                task.acceptance_results = Some(ar);
            }
            if let Some(cr) = input.commands_run {
                task.commands_run = Some(cr);
            }
            if let Some(snap) = artifact_snapshot {
                task.reviewed_artifacts = Some(snap.artifacts);
                task.reviewed_gaps = snap.gaps;
            }
            task.updated_at = now;
        }

        let mut directives = Vec::new();
        let mut spawned_loop = Vec::new();

        if input.status.is_terminal() {
            if let Some(owner) = owner_name.as_deref() {
                if owner != "captain" {
                    if let Some(m) = team.find_member_mut(owner) {
                        if m.status == MemberStatus::Working {
                            m.status = MemberStatus::Idle;
                        }
                    }
                }
            }

            if input.status == TaskStatus::Failed {
                spawned_loop = maybe_spawn_repair_loop(&mut team, &input.task_id);
            }

            let sched = schedule_ready_tasks(&mut team);
            directives.extend(sched.directives);

            // Notify captain when all tasks reach a terminal state.
            let all_done =
                !team.tasks.is_empty() && team.tasks.iter().all(|t| t.status.is_terminal());
            if all_done {
                directives.push(EngineDirective::WakeCaptain {
                    team_id: team.id.clone(),
                    captain_session_id: team.captain_session_id.clone(),
                    message: format!(
                        "All {} tasks in team '{}' have finished. Review results with agent_teams_status and deliver your consolidated final report. Do NOT call agent_teams_delete automatically — leave the team open so the user can inspect the Resolution tab and close/delete it manually.",
                        team.tasks.len(),
                        team.name
                    ),
                });
            }
        }

        let unlocked = if input.status == TaskStatus::Completed {
            direct_unlocks(&team.tasks, &input.task_id)
        } else {
            Vec::new()
        };

        self.store.save_team(&team)?;

        let mut extra = BTreeMap::new();
        if !unlocked.is_empty() {
            extra.insert("unlockedTasks".to_string(), serde_json::json!(unlocked));
        }
        if !spawned_loop.is_empty() {
            extra.insert(
                "spawnedRepairTasks".to_string(),
                serde_json::json!(spawned_loop),
            );
        }

        Ok(OperationOutput {
            summary: format!("Task '{}' updated to '{}'.", input.task_id, input.status),
            team,
            directives,
            extra,
        })
    }

    /// 10. `agent_teams_amend_task`: captain-only contract amendment recorded in revisions ledger.
    pub fn amend_task(&self, input: AmendTaskInput) -> Result<OperationOutput> {
        let reason = input.reason.trim();
        if reason.is_empty() {
            return Err(TeamsError::InvalidInput(
                "amend_task requires a non-empty reason".to_string(),
            ));
        }
        let mut team = self.store.load_team(&input.team_id)?;
        let task = team
            .find_task_mut(&input.task_id)
            .ok_or_else(|| TeamsError::TaskNotFound(input.task_id.clone()))?;

        if task.status.is_terminal() {
            return Err(TeamsError::TaskTerminal(
                task.id.clone(),
                task.status.to_string(),
            ));
        }

        let mut fields = Vec::new();
        let mut previous = BTreeMap::new();

        if let Some(obj) = input.objective {
            fields.push("objective".to_string());
            if let Some(old) = task.objective.replace(obj) {
                previous.insert("objective".to_string(), serde_json::json!(old));
            }
        }
        if let Some(acc) = input.acceptance {
            fields.push("acceptance".to_string());
            if let Some(old) = task.acceptance.replace(acc) {
                previous.insert("acceptance".to_string(), serde_json::json!(old));
            }
        }
        if let Some(ver) = input.verify {
            fields.push("verify".to_string());
            if let Some(old) = task.verify.replace(ver) {
                previous.insert("verify".to_string(), serde_json::json!(old));
            }
        }
        if let Some(in_s) = input.in_scope {
            fields.push("inScope".to_string());
            if let Some(old) = task.in_scope.replace(in_s) {
                previous.insert("inScope".to_string(), serde_json::json!(old));
            }
        }
        if let Some(out_s) = input.out_of_scope {
            fields.push("outOfScope".to_string());
            if let Some(old) = task.out_of_scope.replace(out_s) {
                previous.insert("outOfScope".to_string(), serde_json::json!(old));
            }
        }

        if fields.is_empty() {
            return Err(TeamsError::InvalidInput(
                "amend_task must modify at least one contract field".to_string(),
            ));
        }

        let now = now_ms();
        let rev = TaskRevision {
            at: now,
            by: "captain".to_string(),
            reason: reason.to_string(),
            fields: fields.clone(),
            previous,
        };
        task.revisions.get_or_insert_with(Vec::new).push(rev);
        task.updated_at = now;

        validate_task_creation_contract(task)?;
        self.store.save_team(&team)?;

        Ok(OperationOutput {
            summary: format!("Amended task '{}' fields {fields:?}.", input.task_id),
            team,
            directives: Vec::new(),
            extra: BTreeMap::new(),
        })
    }

    /// 11. `agent_teams_send_message`: P2P durable mailbox delivery + live wake directive.
    pub fn send_message_op(
        &self,
        input: SendMessageInput,
    ) -> Result<(TeamMessage, Vec<EngineDirective>)> {
        let team = self.store.load_team(&input.team_id)?;
        let msg = send_message(
            &self.store,
            &team,
            &input.from,
            &input.to,
            &input.content,
            SendMessageOptions {
                task_id: input.task_id.clone(),
                attempt_id: input.attempt_id.clone(),
                ..SendMessageOptions::default()
            },
        )?;

        let mut directives = Vec::new();
        if msg.to == "captain" {
            directives.push(EngineDirective::WakeCaptain {
                team_id: team.id.clone(),
                captain_session_id: team.captain_session_id.clone(),
                message: format!("[Team message from {}]: {}", msg.from, msg.content),
            });
        } else if let Some(member) = team.find_member(&msg.to) {
            if member.status != MemberStatus::Removed {
                let role = member
                    .role
                    .clone()
                    .unwrap_or_else(|| "teammate".to_string());
                let body = format!("[Direct message from {}]: {}", msg.from, msg.content);
                if member.id.is_empty() {
                    directives.push(EngineDirective::SpawnMember {
                        team_id: team.id.clone(),
                        captain_session_id: team.captain_session_id.clone(),
                        member_name: member.name.clone(),
                        role,
                        provider: member.provider.clone(),
                        model: member.model.clone(),
                        reasoning_effort: member.reasoning_effort.clone(),
                        execution_prompt: member.execution_prompt.clone(),
                        tool_filter: member.tool_filter.clone(),
                        task_id: input.task_id,
                        attempt_id: input.attempt_id,
                        initial_prompt: body,
                    });
                } else {
                    directives.push(EngineDirective::WakeMember {
                        team_id: team.id.clone(),
                        captain_session_id: team.captain_session_id.clone(),
                        member_name: member.name.clone(),
                        role,
                        provider: member.provider.clone(),
                        model: member.model.clone(),
                        reasoning_effort: member.reasoning_effort.clone(),
                        execution_prompt: member.execution_prompt.clone(),
                        tool_filter: member.tool_filter.clone(),
                        subagent_id: member.id.clone(),
                        task_id: input.task_id,
                        attempt_id: input.attempt_id,
                        message: body,
                    });
                }
            }
        }

        Ok((msg, directives))
    }

    /// 12. `agent_teams_status`: full team overview including tasks, members, pending deps, and unread counts.
    pub fn team_status(&self, team_id: &str) -> Result<serde_json::Value> {
        let team = self.store.load_team(team_id)?;
        let team_dir = self.store.team_dir(&team.id);
        let total_messages = self.store.count_team_messages(&team_dir);

        let task_summaries: Vec<serde_json::Value> = team
            .tasks
            .iter()
            .map(|t| {
                let waiting_on = unsatisfied_dependencies(&team.tasks, t);
                let unlocks = direct_unlocks(&team.tasks, &t.id);
                serde_json::json!({
                    "id": t.id,
                    "subject": t.subject,
                    "status": t.status,
                    "kind": t.effective_kind(),
                    "round": t.round,
                    "verdict": t.verdict,
                    "assignee": t.assignee,
                    "attempt": t.attempt,
                    "attemptId": t.attempt_id,
                    "dependencies": t.dependencies,
                    "waitingOn": waiting_on,
                    "unlocks": unlocks,
                    "output": t.output,
                })
            })
            .collect();

        let member_summaries: Vec<serde_json::Value> = team
            .members
            .iter()
            .map(|m| {
                serde_json::json!({
                    "id": m.id,
                    "name": m.name,
                    "role": m.role,
                    "status": m.status,
                    "provider": m.provider,
                    "model": m.model,
                    "reasoningEffort": m.reasoning_effort,
                    "toolPolicy": m.tool_policy,
                    "unread": self.store.count_unread(&team_dir, &m.name),
                    "spawnError": m.spawn_error,
                })
            })
            .collect();

        Ok(serde_json::json!({
            "id": team.id,
            "name": team.name,
            "description": team.description,
            "captainSessionId": team.captain_session_id,
            "phase": team.effective_phase(),
            "planReviewState": team.plan_review_state,
            "halted": team.is_halted(),
            "haltReason": team.halt_reason,
            "escalated": team.escalated.unwrap_or(false),
            "totalMessages": total_messages,
            "captainUnread": self.store.count_unread(&team_dir, "captain"),
            "members": member_summaries,
            "tasks": task_summaries,
        }))
    }

    /// Halt a running team (user clicked Stop Team): cancels all active members and unfinished tasks,
    /// preserves the team on disk so `agent_teams_resume` can continue later.
    pub fn halt_team(&self, team_id: &str, reason: Option<&str>) -> Result<OperationOutput> {
        let mut team = self.store.load_team(team_id)?;
        let now = now_ms();
        team.halted = Some(true);
        team.halted_at = Some(now);
        team.halt_reason = reason.map(ToString::to_string);

        let captain_sid = team.captain_session_id.clone();
        let mut directives = Vec::new();
        for member in &mut team.members {
            if member.status != MemberStatus::Removed && !member.id.is_empty() {
                directives.push(EngineDirective::CancelMember {
                    team_id: team.id.clone(),
                    captain_session_id: captain_sid.clone(),
                    member_name: member.name.clone(),
                    subagent_id: member.id.clone(),
                    reason: reason.unwrap_or("team halted by user").to_string(),
                });
            }
            if member.status == MemberStatus::Working {
                member.status = MemberStatus::Idle;
            }
        }

        for task in &mut team.tasks {
            if !task.status.is_terminal() {
                task.status = TaskStatus::Cancelled;
                task.attempt_id = None;
                task.updated_at = now;
            }
        }

        self.store.save_team(&team)?;

        Ok(OperationOutput {
            summary: format!("Team '{}' halted; active members interrupted.", team.id),
            team,
            directives,
            extra: BTreeMap::new(),
        })
    }

    /// 13. `agent_teams_resume`: explicitly resume a halted team with a non-empty reason.
    ///     Never recreates cancelled tasks.
    pub fn resume_team(&self, team_id: &str, reason: &str) -> Result<OperationOutput> {
        let reason = reason.trim();
        if reason.is_empty() {
            return Err(TeamsError::InvalidInput(
                "agent_teams_resume requires a non-empty reason".to_string(),
            ));
        }
        let mut team = self.store.load_team(team_id)?;
        if !team.is_halted() {
            return Err(TeamsError::InvalidInput(format!(
                "team '{team_id}' is not halted"
            )));
        }
        team.halted = Some(false);
        team.halt_reason = None;

        let sched = schedule_ready_tasks(&mut team);
        self.store.save_team(&team)?;

        Ok(OperationOutput {
            summary: format!("Resumed team '{}' (reason: {reason}).", team.id),
            team,
            directives: sched.directives,
            extra: BTreeMap::new(),
        })
    }

    /// Update an existing team member's model/provider/reasoning effort/role in-place
    /// without spawning duplicate mini-agents. When the team is paused/halted, preserves the
    /// paused state; when active and the member has ready work, dispatches immediately.
    pub fn update_member(&self, input: UpdateMemberInput) -> Result<OperationOutput> {
        let mut team = self.store.load_team(&input.team_id)?;
        let name = input.name.trim();
        let mut directives = Vec::new();

        let (resolved_policy_opt, resolved_filter_opt) = match input.tool_policy.as_deref().map(str::trim) {
            Some(pid) if !pid.is_empty() => {
                let filter = self.config.resolve_policy_id(pid)?;
                (Some(pid.to_string()), Some(filter))
            }
            _ => (None, None),
        };

        let captain_id = team.captain_session_id.clone();
        let team_id = team.id.clone();
        let is_halted = team.is_halted();
        let is_staged = team.is_staged();

        let member = team
            .find_member_mut(name)
            .ok_or_else(|| TeamsError::MemberNotFound(name.to_string()))?;

        // If a previous subagent handle existed and the member is being reconfigured, cancel it if
        // still working and clear `id` so the next launch binds cleanly to the new model/effort.
        let old_subagent_id = member.id.clone();
        let was_working = member.status == MemberStatus::Working;
        if let Some(role) = input.role.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
            member.role = Some(role.to_string());
        }
        if let Some(prov) = input.provider {
            let trimmed = prov.trim();
            member.provider = if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            };
        }
        if let Some(mod_id) = input.model {
            let trimmed = mod_id.trim();
            member.model = if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            };
        }
        if let Some(eff) = input.reasoning_effort {
            let trimmed = eff.trim();
            member.reasoning_effort = if trimmed.is_empty() || trimmed == "default" {
                None
            } else {
                Some(trimmed.to_string())
            };
        }
        if let Some(pid) = resolved_policy_opt {
            member.tool_policy = Some(pid);
            member.tool_filter = resolved_filter_opt;
        }

        member.active_provider = None;
        member.active_model = None;
        member.fallback_active = None;
        member.spawn_error = None;
        member.id = String::new();
        member.status = MemberStatus::Idle;

        if was_working && !old_subagent_id.is_empty() {
            directives.push(EngineDirective::CancelMember {
                team_id: team_id.clone(),
                captain_session_id: captain_id,
                member_name: name.to_string(),
                subagent_id: old_subagent_id,
                reason: format!("member '{name}' model/effort updated in-place"),
            });
            // Release any active task held by the cancelled turn so it re-dispatches cleanly on the new model.
            for t in &mut team.tasks {
                if t.assignee.as_deref() == Some(name) && t.status.is_active() {
                    t.status = TaskStatus::Pending;
                    t.attempt_id = None;
                    t.updated_at = now_ms();
                }
            }
        }

        if !is_staged && !is_halted {
            let sched = schedule_ready_tasks(&mut team);
            directives.extend(sched.directives);
        }

        self.store.save_team(&team)?;
        Ok(OperationOutput {
            summary: format!("Updated member '{name}' in-place in team '{}'.", team.id),
            team,
            directives,
            extra: BTreeMap::new(),
        })
    }

    /// Persist a consolidated final resolution summary for a team (shown in the WebUI Resolution tab).
    pub fn set_resolution(&self, team_id: &str, resolution: String) -> Result<OperationOutput> {
        let mut team = self.store.load_team(team_id)?;
        let trimmed = resolution.trim();
        team.resolution = if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        };
        self.store.save_team(&team)?;
        Ok(OperationOutput {
            summary: format!("Saved resolution summary for team '{}'.", team.id),
            team,
            directives: Vec::new(),
            extra: BTreeMap::new(),
        })
    }

    /// 14. `agent_teams_delete`: stop active members and archive the full team directory on disk.
    pub fn delete_and_archive(
        &self,
        team_id: &str,
        reason: Option<&str>,
    ) -> Result<OperationOutput> {
        let mut team = self.store.load_team(team_id)?;
        let now = now_ms();
        let captain_sid = team.captain_session_id.clone();
        let mut directives = Vec::new();

        for member in &mut team.members {
            if member.status != MemberStatus::Removed && !member.id.is_empty() {
                directives.push(EngineDirective::CancelMember {
                    team_id: team.id.clone(),
                    captain_session_id: captain_sid.clone(),
                    member_name: member.name.clone(),
                    subagent_id: member.id.clone(),
                    reason: reason.unwrap_or("team archived").to_string(),
                });
            }
            member.status = MemberStatus::Removed;
        }
        for task in &mut team.tasks {
            if !task.status.is_terminal() {
                task.status = TaskStatus::Cancelled;
                task.attempt_id = None;
                task.updated_at = now;
            }
        }

        let archive_path = self.store.archive_team(&team)?;
        let mut extra = BTreeMap::new();
        extra.insert(
            "archivePath".to_string(),
            serde_json::json!(archive_path.to_string_lossy()),
        );

        Ok(OperationOutput {
            summary: format!("Team '{}' archived to {}.", team.id, archive_path.display()),
            team,
            directives,
            extra,
        })
    }

    /// Discard a staged draft without ever spawning members; archives draft and notifies captain.
    pub fn discard_staged_plan(&self, team_id: &str) -> Result<OperationOutput> {
        let team = self.store.load_team(team_id)?;
        if !team.is_staged() {
            return Err(TeamsError::InvalidInput(format!(
                "team '{team_id}' is not staged; use halt or delete instead"
            )));
        }
        let mut out = self.delete_and_archive(team_id, Some("staged plan discarded by user"))?;
        out.directives.push(EngineDirective::WakeCaptain {
            team_id: out.team.id.clone(),
            captain_session_id: out.team.captain_session_id.clone(),
            message: format!(
                "The user discarded staged team plan '{}'. Do NOT automatically recreate the team unless they explicitly ask again.",
                out.team.name
            ),
        });
        Ok(out)
    }

    /// Permanently delete an active or archived team from disk (cancelling any active members first).
    pub fn purge_team(&self, team_id: &str, archived: bool) -> Result<Vec<EngineDirective>> {
        if archived {
            self.store.delete_archived_team(team_id)?;
            return Ok(Vec::new());
        }
        let team = self.store.load_team(team_id)?;
        let captain_sid = team.captain_session_id.clone();
        let mut directives = Vec::new();
        for member in &team.members {
            if member.status != MemberStatus::Removed && !member.id.is_empty() {
                directives.push(EngineDirective::CancelMember {
                    team_id: team.id.clone(),
                    captain_session_id: captain_sid.clone(),
                    member_name: member.name.clone(),
                    subagent_id: member.id.clone(),
                    reason: "team deleted permanently by user".to_string(),
                });
            }
        }
        self.store.delete_active_team(team_id)?;
        Ok(directives)
    }

    fn apply_plan_members(
        &self,
        team: &mut TeamState,
        inputs: Vec<PlanMemberInput>,
        captain_provider: Option<&str>,
        captain_model: Option<&str>,
        captain_reasoning_effort: Option<&str>,
    ) -> Result<()> {
        if inputs.len() > self.config.max_members {
            return Err(TeamsError::MaxMembersReached(self.config.max_members));
        }
        for m in inputs {
            if team.find_member(m.name.trim()).is_some() {
                return Err(TeamsError::MemberAlreadyExists(m.name));
            }
            let record = self.build_member_record(
                m,
                captain_provider,
                captain_model,
                captain_reasoning_effort,
            )?;
            team.members.push(record);
        }
        Ok(())
    }

    fn build_member_record(
        &self,
        input: PlanMemberInput,
        captain_provider: Option<&str>,
        captain_model: Option<&str>,
        captain_reasoning_effort: Option<&str>,
    ) -> Result<TeamMember> {
        let name = input.name.trim();
        if name.is_empty() || name.eq_ignore_ascii_case("captain") {
            return Err(TeamsError::InvalidInput(format!(
                "invalid member name '{name}'"
            )));
        }

        let explicit_provider = input.provider.filter(|s| !s.trim().is_empty());
        let explicit_model = input.model.filter(|s| !s.trim().is_empty());

        let changed_route = explicit_provider.is_some() || explicit_model.is_some();
        let resolved_provider = explicit_provider
            .or_else(|| self.config.member_llm_provider.clone())
            .or_else(|| captain_provider.map(ToString::to_string));
        let resolved_model = explicit_model
            .or_else(|| self.config.member_model.clone())
            .or_else(|| captain_model.map(ToString::to_string));

        // Inherit captain reasoning effort only when keeping the captain's route.
        let resolved_effort = match input.reasoning_effort.as_deref().map(str::trim) {
            Some("default" | "") => None,
            Some(explicit) => Some(explicit.to_string()),
            None => {
                if changed_route {
                    None
                } else {
                    captain_reasoning_effort.map(ToString::to_string)
                }
            }
        };

        let (tool_policy, tool_filter) = match input.tool_policy.as_deref().map(str::trim) {
            Some(pid) if !pid.is_empty() => {
                let filter = self.config.resolve_policy_id(pid)?;
                (Some(pid.to_string()), Some(filter))
            }
            _ => {
                if crate::policies::is_readonly_role(input.role.as_deref()) {
                    let filter = self.config.resolve_policy_id("readonly")?;
                    (Some("readonly".to_string()), Some(filter))
                } else {
                    (None, None)
                }
            }
        };

        Ok(TeamMember {
            id: String::new(),
            name: name.to_string(),
            role: input.role,
            provider: resolved_provider,
            model: resolved_model,
            reasoning_effort: resolved_effort,
            execution_prompt: input
                .execution_prompt
                .or_else(|| self.config.execution_prompt.clone()),
            fallback: self.config.fallback.clone(),
            active_provider: None,
            active_model: None,
            fallback_active: None,
            tool_policy,
            tool_filter,
            joined_at: now_ms(),
            status: MemberStatus::Idle,
            stopping: None,
            spawn_error: None,
        })
    }

    fn apply_plan_tasks(team: &mut TeamState, inputs: Vec<PlanTaskInput>) -> Result<()> {
        let mut seed_map: BTreeMap<String, String> = BTreeMap::new();
        let now = now_ms();

        // First pass: assign sequential `t1`, `t2`, ... ids and map seed aliases.
        let mut pending_records: Vec<(TeamTask, Vec<String>)> = Vec::with_capacity(inputs.len());
        for row in inputs {
            team.task_seq += 1;
            let assigned_id = format!("t{}", team.task_seq);
            if let Some(seed_id) = row.id.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
                seed_map.insert(seed_id.to_string(), assigned_id.clone());
            }
            seed_map.insert(assigned_id.clone(), assigned_id.clone());

            if let Some(assignee) = row.assignee.as_deref() {
                if assignee != "captain" && team.find_member(assignee).is_none() {
                    return Err(TeamsError::MemberNotFound(assignee.to_string()));
                }
            }

            let task = TeamTask {
                profile_seed_id: row.id,
                id: assigned_id,
                subject: row.subject,
                description: row.description,
                status: TaskStatus::Pending,
                assignee: row.assignee,
                dependencies: Vec::new(),
                output: None,
                attempt: None,
                attempt_id: None,
                handoff_id: None,
                handoff_from_member_id: None,
                reassigning: None,
                kind: row.kind,
                round: if row.kind.unwrap_or(TaskKind::Work).is_quality() {
                    Some(1)
                } else {
                    None
                },
                verdict: None,
                findings: None,
                objective: row.objective,
                in_scope: row.in_scope,
                out_of_scope: row.out_of_scope,
                acceptance: row.acceptance,
                verify: row.verify,
                deliverables: None,
                non_goals: None,
                changed_paths: None,
                acceptance_results: None,
                commands_run: None,
                supplemental_evidence: None,
                reviewed_task_id: row.reviewed_task_id,
                reviewed_attempt: None,
                reviewed_artifacts: None,
                reviewed_gaps: None,
                reviewed_scope: None,
                reviewed_during_drift: None,
                verdict_start_snapshot: None,
                contract_snapshot: None,
                source_task_id: None,
                finding_source_task_ids: None,
                source_finding_ids: None,
                coverage_of: row.coverage_of,
                revisions: None,
                created_at: now,
                updated_at: now,
            };
            validate_task_creation_contract(&task)?;
            pending_records.push((task, row.dependencies));
        }

        // Second pass: remap dependency seed IDs to `tN` and validate acyclicity.
        for (mut task, raw_deps) in pending_records {
            let deps: Vec<String> = raw_deps
                .into_iter()
                .map(|d| seed_map.get(&d).cloned().unwrap_or(d))
                .collect();
            task.dependencies = deps;
            if let Some(ref rev) = task.reviewed_task_id {
                if let Some(mapped) = seed_map.get(rev) {
                    task.reviewed_task_id = Some(mapped.clone());
                }
            }
            team.tasks.push(task);
        }

        // Validate all edges and acyclicity across the complete batch.
        for idx in 0..team.tasks.len() {
            let tid = team.tasks[idx].id.clone();
            let deps = team.tasks[idx].dependencies.clone();
            validate_dependencies(&team.tasks, &tid, &deps)?;
        }

        Ok(())
    }
}

fn build_spawn_directives(team: &TeamState) -> Vec<EngineDirective> {
    team.members
        .iter()
        .filter(|m| m.status != MemberStatus::Removed && m.id.is_empty())
        .map(|m| spawn_directive_for(team, m))
        .collect()
}

fn spawn_directive_for(team: &TeamState, member: &TeamMember) -> EngineDirective {
    let role = member
        .role
        .clone()
        .unwrap_or_else(|| "teammate".to_string());
    let initial_prompt = format!(
        "You are team member '{}' (role: {}) in Agent Team '{}' ({}). Wait for assigned tasks or claim ready work with agent_teams_claim_task, update results with agent_teams_update_task (including attempt_id), and communicate directly with teammates or 'captain' via agent_teams_send_message.",
        member.name, role, team.name, team.id
    );
    EngineDirective::SpawnMember {
        team_id: team.id.clone(),
        captain_session_id: team.captain_session_id.clone(),
        member_name: member.name.clone(),
        role,
        provider: member.provider.clone(),
        model: member.model.clone(),
        reasoning_effort: member.reasoning_effort.clone(),
        execution_prompt: member.execution_prompt.clone(),
        tool_filter: member.tool_filter.clone(),
        task_id: None,
        attempt_id: None,
        initial_prompt,
    }
}
