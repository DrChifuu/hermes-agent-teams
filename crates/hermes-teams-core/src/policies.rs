use crate::error::{Result, TeamsError};
use crate::types::{
    MemberToolFilter, ReviewPolicy, TaskKind, TaskPlanningMode, TeamModelFallback,
    CAPTAIN_TOOL_NAMES, MEMBER_TOOL_NAMES,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

/// Raw tool policy declared by the deployment owner in config YAML.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RawToolPolicy {
    #[serde(default)]
    pub allow: Option<Vec<String>>,
    #[serde(default)]
    pub deny: Option<Vec<String>>,
}

/// Member seed row inside a named multi-role profile.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileMemberSeed {
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
    pub tool_policy: Option<String>,
}

/// Task seed row inside a named multi-role profile (`taskPlanning: seed`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileTaskSeed {
    pub id: String,
    pub subject: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assignee: Option<String>,
    #[serde(default)]
    pub dependencies: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<TaskKind>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub objective: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub acceptance: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub in_scope: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub out_of_scope: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verify: Option<Vec<String>>,
}

/// Named team profile declared in configuration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TeamProfileConfig {
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
    #[serde(default)]
    pub members: Vec<ProfileMemberSeed>,
    #[serde(default)]
    pub tasks: Vec<ProfileTaskSeed>,
}

/// Complete engine configuration (loaded from YAML or defaults).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TeamsConfig {
    #[serde(default = "default_state_dir")]
    pub state_dir: String,
    #[serde(default = "default_max_members")]
    pub max_members: usize,
    #[serde(default)]
    pub member_max_depth: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub member_model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub member_llm_provider: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub execution_prompt: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fallback: Option<TeamModelFallback>,
    #[serde(default)]
    pub tool_policies: BTreeMap<String, RawToolPolicy>,
    #[serde(default)]
    pub profiles: BTreeMap<String, TeamProfileConfig>,
}

fn default_state_dir() -> String {
    ".agent-teams".to_string()
}

const fn default_max_members() -> usize {
    8
}

impl Default for TeamsConfig {
    fn default() -> Self {
        Self {
            state_dir: default_state_dir(),
            max_members: default_max_members(),
            member_max_depth: 0,
            member_model: None,
            member_llm_provider: None,
            execution_prompt: None,
            fallback: None,
            tool_policies: BTreeMap::new(),
            profiles: BTreeMap::new(),
        }
    }
}

impl TeamsConfig {
    pub fn load_yaml(path: impl AsRef<Path>) -> Result<Self> {
        let raw = std::fs::read_to_string(path)?;
        let cfg: Self = serde_yaml::from_str(&raw)?;
        cfg.validate()?;
        Ok(cfg)
    }

    /// Validate all declared tool policies fail-closed at load time.
    pub fn validate(&self) -> Result<()> {
        for (id, raw) in &self.tool_policies {
            resolve_tool_policy(id, raw)?;
        }
        Ok(())
    }

    /// Look up and resolve an owner-declared tool policy id.
    pub fn resolve_policy_id(&self, id: &str) -> Result<MemberToolFilter> {
        let trimmed = id.trim();
        let raw = self.tool_policies.get(trimmed).ok_or_else(|| {
            TeamsError::ToolPolicyViolation(format!("unknown tool policy id '{trimmed}'"))
        })?;
        resolve_tool_policy(trimmed, raw)
    }
}

/// Resolve a single `RawToolPolicy` enforcing DSH invariants:
/// - Must specify non-empty `allow` or `deny` (cannot be empty policy or empty list).
/// - Cannot deny any of the 4 required member coordination tools.
/// - Captain-only tools are ALWAYS denied and cannot be reopened via `allow`.
/// - When `allow` is used, the 4 member coordination tools are automatically added.
pub fn resolve_tool_policy(id: &str, raw: &RawToolPolicy) -> Result<MemberToolFilter> {
    if raw.allow.is_none() && raw.deny.is_none() {
        return Err(TeamsError::ToolPolicyViolation(format!(
            "tool policy '{id}' must define 'allow' or 'deny'"
        )));
    }

    let captain_set: BTreeSet<&str> = CAPTAIN_TOOL_NAMES.iter().copied().collect();
    let required_member_set: BTreeSet<&str> = MEMBER_TOOL_NAMES.iter().copied().collect();

    let mut deny: BTreeSet<String> = CAPTAIN_TOOL_NAMES
        .iter()
        .map(|s| (*s).to_string())
        .collect();

    if let Some(raw_deny) = &raw.deny {
        if raw_deny.is_empty() {
            return Err(TeamsError::ToolPolicyViolation(format!(
                "tool policy '{id}' has an empty 'deny' list"
            )));
        }
        for name in raw_deny {
            let trimmed = name.trim();
            if trimmed.is_empty() {
                continue;
            }
            if required_member_set.contains(trimmed) {
                return Err(TeamsError::ToolPolicyViolation(format!(
                    "tool policy '{id}' cannot deny required coordination tool '{trimmed}'"
                )));
            }
            deny.insert(trimmed.to_string());
        }
    }

    let allow = if let Some(raw_allow) = &raw.allow {
        if raw_allow.is_empty() {
            return Err(TeamsError::ToolPolicyViolation(format!(
                "tool policy '{id}' has an empty 'allow' list"
            )));
        }
        let mut allowed: BTreeSet<String> = BTreeSet::new();
        for name in raw_allow {
            let trimmed = name.trim();
            if trimmed.is_empty() {
                continue;
            }
            if captain_set.contains(trimmed) {
                return Err(TeamsError::ToolPolicyViolation(format!(
                    "tool policy '{id}' cannot reopen captain-only tool '{trimmed}'"
                )));
            }
            allowed.insert(trimmed.to_string());
        }
        for coord in MEMBER_TOOL_NAMES {
            allowed.insert((*coord).to_string());
        }
        Some(allowed.into_iter().collect())
    } else {
        None
    };

    Ok(MemberToolFilter {
        allow,
        deny: deny.into_iter().collect(),
    })
}
