use crate::dag::{compute_depths, direct_unlocks, unsatisfied_dependencies};
use crate::store::TeamStore;
use crate::types::{MemberStatus, PlanReviewState, TaskKind, TaskStatus, TeamPhase, TeamState};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

/// UI-ready snapshot of a single task node (including precomputed DAG depth and unlocks).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotTask {
    pub id: String,
    pub subject: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub status: TaskStatus,
    pub kind: TaskKind,
    pub assignee: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    pub dependencies: Vec<String>,
    pub waiting_on: Vec<String>,
    pub unlocks: Vec<String>,
    pub depth: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attempt: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verdict: Option<crate::types::ReviewVerdict>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub round: Option<u32>,
    pub updated_at: u64,
}

/// UI-ready snapshot of a single member row.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotMember {
    pub id: String,
    pub name: String,
    pub role: String,
    pub status: MemberStatus,
    /// `"working" | "idle" | "unknown"` used to select Ferris action icon & animations.
    pub activity: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning_effort: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_policy: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_task: Option<String>,
    pub assigned_task_ids: Vec<String>,
    pub completed_tasks: usize,
    pub total_tasks: usize,
    pub unread: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spawn_error: Option<String>,
}

/// Progress counters rendered by the segmented bar and status pills.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TeamStats {
    pub total: usize,
    pub completed: usize,
    pub in_progress: usize,
    pub waiting_deps: usize,
    pub pending_ready: usize,
    pub failed: usize,
    pub cancelled: usize,
    pub member_count: usize,
    pub message_count: usize,
}

/// UI-ready snapshot of one team (active or archived).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TeamSnapshot {
    pub id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workspace: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub captain_session_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub captain_provider: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub captain_model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub captain_reasoning_effort: Option<String>,
    pub phase: TeamPhase,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plan_review_state: Option<PlanReviewState>,
    pub archived: bool,
    pub halted: bool,
    pub escalated: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resolution: Option<String>,
    pub created_at: u64,
    pub stats: TeamStats,
    pub members: Vec<SnapshotMember>,
    pub tasks: Vec<SnapshotTask>,
}

/// Full payload served to the Hermes `WebUI` extension (`GET /api/v1/state`).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceActivityPayload {
    pub teams: Vec<TeamSnapshot>,
    pub archived: Vec<TeamSnapshot>,
    #[serde(default)]
    pub archived_count: usize,
}

/// Build a `TeamSnapshot` from persisted `TeamState` + mailbox counters.
pub fn build_team_snapshot(store: &TeamStore, team: &TeamState, archived: bool) -> TeamSnapshot {
    let dir = if archived {
        store.archive_dir(&team.id)
    } else {
        store.team_dir(&team.id)
    };
    build_team_snapshot_with_dir(store, team, &dir, archived)
}

fn build_team_snapshot_with_dir(
    store: &TeamStore,
    team: &TeamState,
    dir: &Path,
    archived: bool,
) -> TeamSnapshot {
    let depths = compute_depths(&team.tasks).unwrap_or_default();
    let member_routes: BTreeMap<&str, String> = team
        .members
        .iter()
        .filter_map(|m| {
            let route = match (&m.provider, &m.model) {
                (Some(p), Some(mo)) if !p.is_empty() && !mo.is_empty() => format!("{p}/{mo}"),
                (_, Some(mo)) if !mo.is_empty() => mo.clone(),
                _ => return None,
            };
            Some((m.name.as_str(), route))
        })
        .collect();

    let mut completed = 0usize;
    let mut in_progress = 0usize;
    let mut waiting_deps = 0usize;
    let mut pending_ready = 0usize;
    let mut failed = 0usize;
    let mut cancelled = 0usize;

    let tasks: Vec<SnapshotTask> = team
        .tasks
        .iter()
        .map(|t| {
            let waiting = unsatisfied_dependencies(&team.tasks, t);
            let unlocks = direct_unlocks(&team.tasks, &t.id);
            match t.status {
                TaskStatus::Completed => completed += 1,
                TaskStatus::Claimed | TaskStatus::InProgress => in_progress += 1,
                TaskStatus::Failed => failed += 1,
                TaskStatus::Cancelled => cancelled += 1,
                TaskStatus::Pending => {
                    if waiting.is_empty() {
                        pending_ready += 1;
                    } else {
                        waiting_deps += 1;
                    }
                }
            }
            let assignee = t.assignee.clone().unwrap_or_default();
            let model = member_routes.get(assignee.as_str()).cloned();
            SnapshotTask {
                id: t.id.clone(),
                subject: t.subject.clone(),
                description: t.description.clone(),
                status: t.status,
                kind: t.effective_kind(),
                assignee,
                model,
                dependencies: t.dependencies.clone(),
                waiting_on: waiting,
                unlocks,
                depth: depths.get(&t.id).copied().unwrap_or(0),
                output: t.output.clone(),
                attempt: t.attempt,
                verdict: t.verdict,
                round: t.round,
                updated_at: t.updated_at,
            }
        })
        .collect();

    let members: Vec<SnapshotMember> = team
        .members
        .iter()
        .map(|m| {
            let owned: Vec<&SnapshotTask> = tasks.iter().filter(|t| t.assignee == m.name).collect();
            let assigned_task_ids: Vec<String> = owned.iter().map(|t| t.id.clone()).collect();
            let completed_tasks = owned
                .iter()
                .filter(|t| t.status == TaskStatus::Completed)
                .count();
            let current_task = owned
                .iter()
                .find(|t| t.status.is_active())
                .map(|t| t.subject.clone());

            let activity = if archived || team.is_halted() || m.status == MemberStatus::Removed {
                "idle"
            } else if m.status == MemberStatus::Working || current_task.is_some() {
                "working"
            } else if team.is_staged() {
                "unknown"
            } else {
                "idle"
            };

            SnapshotMember {
                id: m.id.clone(),
                name: m.name.clone(),
                role: m.role.clone().unwrap_or_else(|| "teammate".to_string()),
                status: m.status,
                activity: activity.to_string(),
                provider: m.provider.clone(),
                model: m.model.clone(),
                reasoning_effort: m.reasoning_effort.clone(),
                tool_policy: m.tool_policy.clone(),
                current_task,
                assigned_task_ids,
                completed_tasks,
                total_tasks: owned.len(),
                unread: if archived {
                    0
                } else {
                    store.count_unread(dir, &m.name)
                },
                spawn_error: m.spawn_error.clone(),
            }
        })
        .collect();

    let message_count = store.count_team_messages(dir);
    let stats = TeamStats {
        total: tasks.len(),
        completed,
        in_progress,
        waiting_deps,
        pending_ready,
        failed,
        cancelled,
        member_count: members.len(),
        message_count,
    };

    let workspace = store
        .root()
        .parent()
        .map(|p| p.to_string_lossy().to_string())
        .filter(|s| !s.is_empty());

    TeamSnapshot {
        id: team.id.clone(),
        name: team.name.clone(),
        workspace,
        description: team.description.clone(),
        captain_session_id: team.captain_session_id.clone(),
        captain_provider: team.captain_provider.clone(),
        captain_model: team.captain_model.clone(),
        captain_reasoning_effort: team.captain_reasoning_effort.clone(),
        phase: team.effective_phase(),
        plan_review_state: team.plan_review_state,
        archived,
        halted: team.is_halted(),
        escalated: team.escalated.unwrap_or(false),
        resolution: team.resolution.clone(),
        created_at: team.created_at,
        stats,
        members,
        tasks,
    }
}

/// Collect active + archived snapshots for a workspace, optionally filtering active teams to one captain session.
/// Archived teams are workspace-scoped history and are always returned so the Archive tab stays populated.
pub fn collect_workspace_activity(
    store: &TeamStore,
    session_id: Option<&str>,
) -> WorkspaceActivityPayload {
    collect_workspace_activity_opts(store, session_id, true)
}

/// Collect active snapshots and optionally archived snapshots (or just a fast `archived_count` when `include_archived == false`).
pub fn collect_workspace_activity_opts(
    store: &TeamStore,
    session_id: Option<&str>,
    include_archived: bool,
) -> WorkspaceActivityPayload {
    let filter_sid = session_id.map(str::trim).filter(|s| !s.is_empty());

    let teams = store
        .list_active_teams()
        .unwrap_or_default()
        .into_iter()
        .filter(|t| filter_sid.is_none_or(|sid| t.captain_session_id == sid))
        .map(|t| build_team_snapshot(store, &t, false))
        .collect();

    if include_archived {
        let archived: Vec<TeamSnapshot> = store
            .list_archived_teams()
            .unwrap_or_default()
            .into_iter()
            .map(|t| build_team_snapshot(store, &t, true))
            .collect();
        let archived_count = archived.len();
        WorkspaceActivityPayload {
            teams,
            archived,
            archived_count,
        }
    } else {
        let archived_count = store.count_archived_teams();
        WorkspaceActivityPayload {
            teams,
            archived: Vec::new(),
            archived_count,
        }
    }
}
