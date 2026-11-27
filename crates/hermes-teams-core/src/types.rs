use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::time::{SystemTime, UNIX_EPOCH};

/// Stable business tool names — exact 1:1 parity with `dsh-agent-teams`.
pub const TEAM_TOOL_NAMES: &[&str] = &[
    "agent_teams_create",
    "agent_teams_approve",
    "agent_teams_edit_plan",
    "agent_teams_add_member",
    "agent_teams_remove_member",
    "agent_teams_create_task",
    "agent_teams_reassign_task",
    "agent_teams_claim_task",
    "agent_teams_update_task",
    "agent_teams_amend_task",
    "agent_teams_send_message",
    "agent_teams_status",
    "agent_teams_resume",
    "agent_teams_delete",
];

/// Four coordination tools always exposed to team members.
pub const MEMBER_TOOL_NAMES: &[&str] = &[
    "agent_teams_claim_task",
    "agent_teams_update_task",
    "agent_teams_send_message",
    "agent_teams_status",
];

/// Ten captain-only tools that members may never invoke.
pub const CAPTAIN_TOOL_NAMES: &[&str] = &[
    "agent_teams_create",
    "agent_teams_approve",
    "agent_teams_edit_plan",
    "agent_teams_add_member",
    "agent_teams_remove_member",
    "agent_teams_create_task",
    "agent_teams_reassign_task",
    "agent_teams_amend_task",
    "agent_teams_resume",
    "agent_teams_delete",
];

/// Current Unix timestamp in milliseconds.
pub fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_millis() as u64)
}

/// Sanitize a human team name into a safe filesystem directory identifier.
pub fn sanitize_id(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len().min(64));
    let mut prev_dash = false;
    for ch in raw.trim().chars() {
        let c = ch.to_ascii_lowercase();
        if c.is_ascii_alphanumeric() {
            out.push(c);
            prev_dash = false;
        } else if (c == '-' || c == '_' || c.is_whitespace() || c == '.')
            && !prev_dash
            && !out.is_empty()
        {
            out.push('-');
            prev_dash = true;
        }
        if out.len() >= 64 {
            break;
        }
    }
    let trimmed = out.trim_matches('-').to_string();
    if trimmed.is_empty() {
        format!("team-{}", now_ms())
    } else {
        trimmed
    }
}

/// Generate a monotonic-enough opaque unique token (hex timestamp + counter).
pub fn gen_token(prefix: &str, seq: u64) -> String {
    format!("{prefix}-{:x}-{seq}", now_ms())
}

/// Task lifecycle statuses in progression order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Pending,
    Claimed,
    InProgress,
    Completed,
    Failed,
    Cancelled,
}

impl TaskStatus {
    pub const fn is_terminal(self) -> bool {
        matches!(self, Self::Completed | Self::Failed | Self::Cancelled)
    }

    pub const fn is_active(self) -> bool {
        matches!(self, Self::Claimed | Self::InProgress)
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Claimed => "claimed",
            Self::InProgress => "in_progress",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
        }
    }
}

impl std::fmt::Display for TaskStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Structured quality-gate task kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskKind {
    Requirements,
    Implementation,
    Verification,
    Review,
    Repair,
    Integration,
    #[default]
    Work,
}

impl TaskKind {
    pub const fn is_quality(self) -> bool {
        !matches!(self, Self::Work)
    }

    pub const fn requires_verdict(self) -> bool {
        matches!(self, Self::Review | Self::Requirements)
    }

    pub const fn requires_scope_and_verify(self) -> bool {
        matches!(self, Self::Implementation | Self::Repair)
    }
}

/// Review / requirements conclusion. Only `Pass` may complete those kinds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReviewVerdict {
    Pass,
    NeedsRevision,
    Reject,
}

/// Severity level for review findings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FindingSeverity {
    Low,
    Medium,
    High,
    Blocker,
}

/// One structured review finding.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewFinding {
    pub id: String,
    pub severity: FindingSeverity,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<u32>,
    pub problem: String,
    pub required_fix: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resolved: Option<bool>,
}

/// Result of one acceptance criterion checked at completion.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AcceptanceResult {
    pub criterion: String,
    pub status: CheckOutcome,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub evidence: Option<String>,
}

/// Result of one verification command executed at completion.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandResult {
    pub command: String,
    pub status: CheckOutcome,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub evidence: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CheckOutcome {
    Passed,
    Failed,
}

/// Digest of one workspace path at a single instant.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArtifactDigest {
    pub path: String,
    pub present: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sha256: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mtime_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unreadable: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArtifactScopeGap {
    pub pattern: String,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArtifactSnapshot {
    pub at: u64,
    pub artifacts: Vec<ArtifactDigest>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gaps: Option<Vec<ArtifactScopeGap>>,
}

/// Profile / team review-loop limits.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewPolicy {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub requirements_min_rounds: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub requirements_max_rounds: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code_max_rounds: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_repair_attempts: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required_reviewers: Option<Vec<String>>,
}

/// One captain-only contract amendment recorded on a quality task.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskRevision {
    pub at: u64,
    pub by: String,
    pub reason: String,
    pub fields: Vec<String>,
    pub previous: BTreeMap<String, serde_json::Value>,
}

/// Append-only supplemental observations on a task.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskEvidence {
    pub at: u64,
    pub by: String,
    pub attempt: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attempt_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub acceptance_results: Option<Vec<AcceptanceResult>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub commands_run: Option<Vec<CommandResult>>,
}

/// One task inside a team's DAG.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TeamTask {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profile_seed_id: Option<String>,
    pub id: String,
    pub subject: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub status: TaskStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assignee: Option<String>,
    #[serde(default)]
    pub dependencies: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attempt: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attempt_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub handoff_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub handoff_from_member_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reassigning: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<TaskKind>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub round: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verdict: Option<ReviewVerdict>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub findings: Option<Vec<ReviewFinding>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub objective: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub in_scope: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub out_of_scope: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub acceptance: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verify: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deliverables: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub non_goals: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub changed_paths: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub acceptance_results: Option<Vec<AcceptanceResult>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub commands_run: Option<Vec<CommandResult>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supplemental_evidence: Option<Vec<TaskEvidence>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reviewed_task_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reviewed_attempt: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reviewed_artifacts: Option<Vec<ArtifactDigest>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reviewed_gaps: Option<Vec<ArtifactScopeGap>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reviewed_scope: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reviewed_during_drift: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verdict_start_snapshot: Option<ArtifactSnapshot>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contract_snapshot: Option<ArtifactSnapshot>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_task_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finding_source_task_ids: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_finding_ids: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub coverage_of: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub revisions: Option<Vec<TaskRevision>>,
    pub created_at: u64,
    pub updated_at: u64,
}

impl TeamTask {
    pub fn effective_kind(&self) -> TaskKind {
        self.kind.unwrap_or(TaskKind::Work)
    }
}

/// Member lifecycle status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemberStatus {
    Idle,
    Working,
    Removed,
}

/// Owner-declared tool policy filter resolved for a member.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemberToolFilter {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allow: Option<Vec<String>>,
    #[serde(default)]
    pub deny: Vec<String>,
}

/// Fallback LLM route for unavailable member models.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TeamModelFallback {
    pub provider: String,
    pub model: String,
}

/// One team member backed by a continuable Hermes subagent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TeamMember {
    /// Durable Hermes subagent session ID (empty while staged / unspawned).
    pub id: String,
    /// Unique display name inside the team.
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning_effort: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub execution_prompt: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fallback: Option<TeamModelFallback>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active_provider: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active_model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fallback_active: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_policy: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_filter: Option<MemberToolFilter>,
    pub joined_at: u64,
    pub status: MemberStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stopping: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spawn_error: Option<String>,
}

/// One durable JSONL mailbox message.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TeamMessage {
    pub id: String,
    pub from: String,
    pub to: String,
    pub content: String,
    pub ts: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delivery_claimed_at: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delivered_at: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub read_at: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attempt_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_task_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_attempt_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_task_status: Option<TaskStatus>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub discarded_at: Option<u64>,
}

/// Planning mode for named profiles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskPlanningMode {
    Captain,
    Seed,
}

/// Immutable named profile snapshot stored on the team.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TeamProfileSnapshot {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub protocol: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub execution_prompt: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fallback: Option<TeamModelFallback>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task_planning: Option<TaskPlanningMode>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub review_policy: Option<ReviewPolicy>,
}

/// Two-phase execution lifecycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TeamPhase {
    Staged,
    #[default]
    Running,
}

/// Human-facing review sub-state while staged.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlanReviewState {
    #[default]
    AwaitingReview,
    AwaitingFeedback,
}

/// Full durable team record persisted to `<workspace>/.agent-teams/<teamId>/team.json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TeamState {
    pub name: String,
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profile: Option<TeamProfileSnapshot>,
    pub captain_session_id: String,
    pub created_at: u64,
    #[serde(default)]
    pub members: Vec<TeamMember>,
    #[serde(default)]
    pub tasks: Vec<TeamTask>,
    #[serde(default)]
    pub task_seq: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phase: Option<TeamPhase>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plan_review_state: Option<PlanReviewState>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub approved_at: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub halted: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub halted_at: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub halt_reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub review_policy: Option<ReviewPolicy>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub escalated: Option<bool>,
}

impl TeamState {
    pub fn effective_phase(&self) -> TeamPhase {
        self.phase.unwrap_or(TeamPhase::Running)
    }

    pub fn is_halted(&self) -> bool {
        self.halted.unwrap_or(false)
    }

    pub fn is_staged(&self) -> bool {
        self.effective_phase() == TeamPhase::Staged
    }

    pub fn find_member(&self, name: &str) -> Option<&TeamMember> {
        self.members.iter().find(|m| m.name == name)
    }

    pub fn find_member_mut(&mut self, name: &str) -> Option<&mut TeamMember> {
        self.members.iter_mut().find(|m| m.name == name)
    }

    pub fn find_task(&self, id: &str) -> Option<&TeamTask> {
        self.tasks.iter().find(|t| t.id == id)
    }

    pub fn find_task_mut(&mut self, id: &str) -> Option<&mut TeamTask> {
        self.tasks.iter_mut().find(|t| t.id == id)
    }

    pub fn active_members_count(&self) -> usize {
        self.members
            .iter()
            .filter(|m| m.status != MemberStatus::Removed)
            .count()
    }
}

/// Runtime directive emitted by the Rust core for the Hermes Python plugin shim
/// to execute against native `ctx.subagent_lifecycle` / `delegate_task` / `inject_message`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum EngineDirective {
    /// Spawn a new durable Hermes subagent for a team member (carrying its first claimed task if ready).
    SpawnMember {
        team_id: String,
        captain_session_id: String,
        member_name: String,
        role: String,
        provider: Option<String>,
        model: Option<String>,
        reasoning_effort: Option<String>,
        execution_prompt: Option<String>,
        tool_filter: Option<MemberToolFilter>,
        task_id: Option<String>,
        attempt_id: Option<String>,
        initial_prompt: String,
    },
    /// Send a follow-up turn to an existing member subagent (new task or mailbox message).
    WakeMember {
        team_id: String,
        captain_session_id: String,
        member_name: String,
        role: String,
        provider: Option<String>,
        model: Option<String>,
        reasoning_effort: Option<String>,
        execution_prompt: Option<String>,
        tool_filter: Option<MemberToolFilter>,
        subagent_id: String,
        task_id: Option<String>,
        attempt_id: Option<String>,
        message: String,
    },
    /// Cancel/interrupt a running member subagent (reassignment, removal, halt, or archive).
    CancelMember {
        team_id: String,
        captain_session_id: String,
        member_name: String,
        subagent_id: String,
        reason: String,
    },
    /// Wake the captain session with a control message (approval, report, halt, feedback).
    WakeCaptain {
        team_id: String,
        captain_session_id: String,
        message: String,
    },
}
