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
}
