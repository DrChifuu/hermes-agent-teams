use thiserror::Error;

/// Domain errors produced by the Agent Teams engine.
#[derive(Debug, Error)]
pub enum TeamsError {
    #[error("invalid input: {0}")]
    InvalidInput(String),

    #[error("captain session '{session_id}' already leads active team '{team_id}'")]
    CaptainAlreadyHasTeam { session_id: String, team_id: String },

    #[error("team '{0}' already exists")]
    TeamAlreadyExists(String),

    #[error("team '{0}' not found")]
    TeamNotFound(String),

    #[error("member '{0}' already exists in team")]
    MemberAlreadyExists(String),

    #[error("member '{0}' not found in team")]
    MemberNotFound(String),

    #[error("team reached maxMembers limit ({0})")]
    MaxMembersReached(usize),

    #[error("task '{0}' not found in team")]
    TaskNotFound(String),

    #[error(
        "task '{task_id}' cannot be claimed because dependencies are not completed: {missing:?}"
    )]
    UnsatisfiedDependencies {
        task_id: String,
        missing: Vec<String>,
    },

    #[error("dependency cycle detected involving task '{0}'")]
    DependencyCycle(String),

    #[error("stale or mismatched attempt_id for task '{task_id}': expected '{expected}', got '{actual}'")]
    StaleAttempt {
        task_id: String,
        expected: String,
        actual: String,
    },

    #[error("member '{member}' already owns active task '{task_id}'")]
    MemberBusy { member: String, task_id: String },

    #[error("team '{0}' is currently staged; approve the plan before executing tasks")]
    TeamStaged(String),

    #[error("team '{0}' is halted; call agent_teams_resume with a non-empty reason first")]
    TeamHalted(String),

    #[error("task '{0}' is in terminal status '{1}' and cannot be mutated")]
    TaskTerminal(String, String),

    #[error("permission denied: {0}")]
    PermissionDenied(String),

    #[error("quality gate violation on task '{task_id}': {reason}")]
    QualityGateViolation { task_id: String, reason: String },

    #[error("tool policy violation: {0}")]
    ToolPolicyViolation(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON serialization error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("YAML parsing error: {0}")]
    Yaml(#[from] serde_yaml::Error),
}

pub type Result<T> = std::result::Result<T, TeamsError>;
