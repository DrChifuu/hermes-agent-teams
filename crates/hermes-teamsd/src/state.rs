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
        let known = vec![default_workspace.clone()];
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

    pub fn remember_workspace(&self, ws: &Path) {
        if let Ok(mut list) = self.0.known_workspaces.lock() {
            if !list.iter().any(|p| p == ws) {
                list.push(ws.to_path_buf());
                if list.len() > 64 {
                    list.remove(0);
                }
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
