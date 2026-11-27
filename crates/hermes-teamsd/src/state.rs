use crate::auth::AuthMode;
use hermes_teams_core::{EngineDirective, TeamEngine, TeamStore, TeamsConfig};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tokio::sync::broadcast;

/// Live event broadcast to connected `WebUI` SSE clients whenever team state mutates.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LiveEvent {
    pub workspace: String,
    pub team_id: Option<String>,
    pub kind: String,
    pub ts: u64,
}

/// One live background activity entry (tool invocation, lifecycle transition, error, or mailbox).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActivityEvent {
    pub id: u64,
    pub ts: u64,
    pub team_id: String,
    pub member: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subagent_id: Option<String>,
    /// `"tool_start" | "tool_end" | "spawned" | "completed" | "failed" | "cancelled" | "blocked" | "message" | "lifecycle"`
    pub kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool: Option<String>,
    pub summary: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
}

/// Live runtime telemetry snapshot for one background team member.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemberRuntimeState {
    pub team_id: String,
    pub member: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subagent_id: Option<String>,
    /// `"running" | "idle" | "succeeded" | "failed" | "cancelled" | "interrupted"`
    pub state: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_tool: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_tool_detail: Option<String>,
    pub tool_count: u64,
    pub started_at: u64,
    pub updated_at: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_summary: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Default)]
struct ActivityStore {
    seq: u64,
    /// `team_id -> ring buffer of ActivityEvent (newest at back)`
    events: HashMap<String, VecDeque<ActivityEvent>>,
    /// `(team_id, member_name) -> live runtime state`
    runtime: HashMap<(String, String), MemberRuntimeState>,
}

/// Queued directive waiting to be consumed by the Hermes Python plugin bridge
/// (used when mutations originate from the `WebUI`: Approve & Run, Feedback, Discard, Halt).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QueuedDirective {
    pub id: u64,
    pub workspace: String,
    pub directive: EngineDirective,
}

#[derive(Debug)]
struct InnerState {
    started_at: Instant,
    auth_mode: AuthMode,
    default_workspace: PathBuf,
    config: TeamsConfig,
    /// Per-workspace mutex serializing disk mutations cleanly without global contention.
    workspace_locks: Mutex<HashMap<PathBuf, Arc<Mutex<()>>>>,
    /// Pending directives triggered by `WebUI` clicks, drained by the Hermes plugin bridge.
    directive_queue: Mutex<(u64, VecDeque<QueuedDirective>)>,
    /// Broadcast channel for real-time `WebUI` SSE subscribers.
    events_tx: broadcast::Sender<LiveEvent>,
    /// Track known workspaces so `/api/v1/state` without explicit workspace still finds active teams.
    known_workspaces: Mutex<Vec<PathBuf>>,
    /// In-memory ring buffer of live background agent activity + per-member runtime status.
    activity: Mutex<ActivityStore>,
}

/// Thread-safe shared daemon state.
#[derive(Debug, Clone)]
pub struct AppState(Arc<InnerState>);

impl AppState {
    pub fn new(auth_mode: AuthMode, default_workspace: PathBuf, config: TeamsConfig) -> Self {
        let (events_tx, _) = broadcast::channel(256);
        let mut known = vec![default_workspace.clone()];
        for ws in Self::load_seed_workspaces() {
            if ws.is_dir() && !known.iter().any(|p| p == &ws) {
                known.push(ws);
            }
        }
        Self(Arc::new(InnerState {
            started_at: Instant::now(),
            auth_mode,
            default_workspace,
            config,
            workspace_locks: Mutex::new(HashMap::new()),
            directive_queue: Mutex::new((0, VecDeque::new())),
            events_tx,
            known_workspaces: Mutex::new(known),
            activity: Mutex::new(ActivityStore::default()),
        }))
    }

    fn workspaces_registry_path() -> Option<PathBuf> {
        let home = std::env::var("HERMES_HOME")
            .ok()
            .filter(|s| !s.trim().is_empty())
            .map(PathBuf::from)
            .or_else(|| {
                std::env::var("HOME")
                    .ok()
                    .filter(|s| !s.trim().is_empty())
                    .map(|h| PathBuf::from(h).join(".hermes"))
            })?;
        Some(home.join("agent-teams-workspaces.json"))
    }

    fn load_seed_workspaces() -> Vec<PathBuf> {
        let mut out = Vec::new();
        if let Some(reg_path) = Self::workspaces_registry_path() {
            if let Ok(raw) = std::fs::read_to_string(&reg_path) {
                if let Ok(list) = serde_json::from_str::<Vec<String>>(&raw) {
                    for s in list {
                        let p = PathBuf::from(s.trim());
                        if !s.trim().is_empty() && p.is_dir() && !out.iter().any(|x| x == &p) {
                            out.push(p);
                        }
                    }
                }
            }
            // Also seed Hermes's configured terminal.cwd from config.yaml if present.
            if let Some(hermes_home) = reg_path.parent() {
                let cfg_path = hermes_home.join("config.yaml");
                if let Ok(raw) = std::fs::read_to_string(&cfg_path) {
                    for line in raw.lines() {
                        let trimmed = line.trim();
                        if let Some(rest) = trimmed.strip_prefix("cwd:") {
                            let val = rest.trim().trim_matches('"').trim_matches('\'');
                            if !val.is_empty() {
                                let p = PathBuf::from(val);
                                if p.is_dir() && !out.iter().any(|x| x == &p) {
                                    out.push(p);
                                }
                            }
                        }
                    }
                }
            }
        }
        out
    }

    fn persist_workspaces(list: &[PathBuf]) {
        let Some(reg_path) = Self::workspaces_registry_path() else {
            return;
        };
        let strings: Vec<String> = list
            .iter()
            .filter(|p| p.is_dir())
            .map(|p| p.to_string_lossy().to_string())
            .collect();
        if let Ok(bytes) = serde_json::to_vec_pretty(&strings) {
            if let Some(parent) = reg_path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            let tmp = reg_path.with_extension("json.tmp");
            if std::fs::write(&tmp, bytes).is_ok() {
                let _ = std::fs::rename(&tmp, &reg_path);
            }
        }
    }

    pub fn uptime_secs(&self) -> u64 {
        self.0.started_at.elapsed().as_secs()
    }

    pub fn auth_mode(&self) -> AuthMode {
        self.0.auth_mode
    }

    pub fn available_policy_ids(&self) -> Vec<String> {
        self.0.config.available_policy_ids()
    }

    pub fn policies_summary(&self) -> serde_json::Value {
        serde_json::to_value(&self.0.config.tool_policies).unwrap_or_else(|_| serde_json::json!({}))
    }

    pub fn resolve_workspace(&self, raw: Option<&str>) -> PathBuf {
        let path = raw
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map_or_else(|| self.0.default_workspace.clone(), PathBuf::from);
        self.remember_workspace(&path);
        path
    }

    /// Resolve workspace for a specific `team_id`: honors explicit `raw` workspace if provided,
    /// otherwise locates the known workspace on disk that actually owns `.agent-teams/<team_id>`.
    pub fn resolve_workspace_for_team(&self, raw: Option<&str>, team_id: &str) -> PathBuf {
        if let Some(ws) = raw.map(str::trim).filter(|s| !s.is_empty()) {
            return self.resolve_workspace(Some(ws));
        }
        for candidate in self.known_workspaces() {
            let store = TeamStore::for_workspace(&candidate, Some(&self.0.config.state_dir));
            if store.team_file(team_id).is_file()
                || store.archive_dir(team_id).join("team.json").is_file()
            {
                return candidate;
            }
        }
        self.0.default_workspace.clone()
    }

    pub fn remember_workspace(&self, ws: &Path) {
        if let Ok(mut list) = self.0.known_workspaces.lock() {
            if !list.iter().any(|p| p == ws) {
                list.push(ws.to_path_buf());
                if list.len() > 64 {
                    list.remove(0);
                }
                Self::persist_workspaces(&list);
            }
        }
    }

    pub fn known_workspaces(&self) -> Vec<PathBuf> {
        self.0
            .known_workspaces
            .lock()
            .map_or_else(|_| vec![self.0.default_workspace.clone()], |g| g.clone())
    }

    /// Acquire the per-workspace lock and execute a synchronous core operation atomically.
    pub fn with_engine<F, R>(&self, workspace: &Path, f: F) -> R
    where
        F: FnOnce(&TeamEngine) -> R,
    {
        self.remember_workspace(workspace);
        let lock = {
            let mut map = self
                .0
                .workspace_locks
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            map.entry(workspace.to_path_buf())
                .or_insert_with(|| Arc::new(Mutex::new(())))
                .clone()
        };
        let _guard = lock
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let store = TeamStore::for_workspace(workspace, Some(&self.0.config.state_dir));
        let engine = TeamEngine::new(store, self.0.config.clone());
        f(&engine)
    }

    pub fn publish_event(&self, workspace: &Path, team_id: Option<String>, kind: &str) {
        let _ = self.0.events_tx.send(LiveEvent {
            workspace: workspace.to_string_lossy().to_string(),
            team_id,
            kind: kind.to_string(),
            ts: hermes_teams_core::types::now_ms(),
        });
    }

    pub fn subscribe_events(&self) -> broadcast::Receiver<LiveEvent> {
        self.0.events_tx.subscribe()
    }

    pub fn enqueue_directives(&self, workspace: &Path, directives: Vec<EngineDirective>) {
        if directives.is_empty() {
            return;
        }
        if let Ok(mut guard) = self.0.directive_queue.lock() {
            let ws_str = workspace.to_string_lossy().to_string();
            for dir in directives {
                guard.0 += 1;
                let id = guard.0;
                guard.1.push_back(QueuedDirective {
                    id,
                    workspace: ws_str.clone(),
                    directive: dir,
                });
            }
            while guard.1.len() > 256 {
                guard.1.pop_front();
            }
        }
    }

    pub fn drain_directives(&self, workspace_filter: Option<&str>) -> Vec<QueuedDirective> {
        let Ok(mut guard) = self.0.directive_queue.lock() else {
            return Vec::new();
        };
        let filter = workspace_filter.map(str::trim).filter(|s| !s.is_empty());
        let mut drained = Vec::new();
        let mut remaining = VecDeque::new();
        while let Some(item) = guard.1.pop_front() {
            if filter.is_none_or(|ws| item.workspace == ws) {
                drained.push(item);
            } else {
                remaining.push_back(item);
            }
        }
        guard.1 = remaining;
        drained
    }

    #[allow(clippy::too_many_arguments)]
    pub fn record_activity(
        &self,
        team_id: &str,
        member: &str,
        subagent_id: Option<String>,
        kind: &str,
        tool: Option<String>,
        summary: String,
        detail: Option<String>,
        status: Option<String>,
        duration_ms: Option<u64>,
        runtime_state: Option<String>,
        error: Option<String>,
    ) -> ActivityEvent {
        let ts = hermes_teams_core::types::now_ms();
        let mut guard = self
            .0
            .activity
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        guard.seq += 1;
        let id = guard.seq;
        let is_err_status = status.as_deref() == Some("error");
        let ev = ActivityEvent {
            id,
            ts,
            team_id: team_id.to_string(),
            member: member.to_string(),
            subagent_id: subagent_id.clone(),
            kind: kind.to_string(),
            tool: tool.clone(),
            summary: summary.clone(),
            detail: detail.clone(),
            status,
            duration_ms,
        };
        let buf = guard.events.entry(team_id.to_string()).or_default();
        buf.push_back(ev.clone());
        while buf.len() > 240 {
            buf.pop_front();
        }

        // Update live per-member runtime snapshot when a specific member is referenced.
        if !member.is_empty() && member != "system" {
            let key = (team_id.to_string(), member.to_string());
            let entry = guard
                .runtime
                .entry(key)
                .or_insert_with(|| MemberRuntimeState {
                    team_id: team_id.to_string(),
                    member: member.to_string(),
                    subagent_id: subagent_id.clone(),
                    state: "idle".to_string(),
                    current_tool: None,
                    current_tool_detail: None,
                    tool_count: 0,
                    started_at: ts,
                    updated_at: ts,
                    last_summary: None,
                    error: None,
                });
            if let Some(sid) = subagent_id.filter(|s| !s.is_empty()) {
                entry.subagent_id = Some(sid);
            }
            entry.updated_at = ts;

            match kind {
                "spawned" => {
                    entry.state = "running".to_string();
                    entry.started_at = ts;
                    entry.current_tool = None;
                    entry.current_tool_detail = None;
                    entry.error = None;
                    entry.last_summary = Some(summary);
                }
                "tool_start" => {
                    entry.state = "running".to_string();
                    entry.current_tool = tool;
                    entry.current_tool_detail = detail.or(Some(summary));
                }
                "tool_end" => {
                    entry.tool_count += 1;
                    entry.current_tool = None;
                    entry.current_tool_detail = None;
                    entry.last_summary = Some(summary);
                    if is_err_status {
                        entry.error = error.or(detail);
                    }
                }
                "blocked" => {
                    entry.current_tool = None;
                    entry.current_tool_detail = None;
                    entry.last_summary = Some(summary);
                }
                "completed" | "succeeded" => {
                    entry.state = "succeeded".to_string();
                    entry.current_tool = None;
                    entry.current_tool_detail = None;
                    entry.last_summary = Some(summary);
                    entry.error = None;
                }
                "failed" => {
                    entry.state = "failed".to_string();
                    entry.current_tool = None;
                    entry.current_tool_detail = None;
                    entry.error = error.or(Some(summary.clone()));
                    entry.last_summary = Some(summary);
                }
                "cancelled" | "interrupted" => {
                    entry.state = kind.to_string();
                    entry.current_tool = None;
                    entry.current_tool_detail = None;
                    entry.last_summary = Some(summary);
                }
                _ => {
                    if let Some(ref st) = runtime_state {
                        entry.state.clone_from(st);
                    }
                    if let Some(err) = error {
                        entry.error = Some(err);
                    }
                    entry.last_summary = Some(summary);
                }
            }
            if let Some(st) = runtime_state {
                entry.state = st;
            }
        }
        ev
    }

    pub fn get_team_activity(
        &self,
        team_id: &str,
    ) -> (Vec<MemberRuntimeState>, Vec<ActivityEvent>) {
        let guard = self
            .0
            .activity
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let events: Vec<ActivityEvent> = guard
            .events
            .get(team_id)
            .map(|q| q.iter().rev().cloned().collect())
            .unwrap_or_default();
        let members: Vec<MemberRuntimeState> = guard
            .runtime
            .values()
            .filter(|r| r.team_id == team_id)
            .cloned()
            .collect();
        (members, events)
    }
}
