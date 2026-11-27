use crate::error::{Result, TeamsError};
use crate::types::{now_ms, TeamMessage, TeamState};
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

/// Default state directory name inside each workspace.
pub const DEFAULT_STATE_DIR: &str = ".agent-teams";

/// Disk-backed atomic persistence for teams, mailboxes, and archives.
#[derive(Debug, Clone)]
pub struct TeamStore {
    root: PathBuf,
}

impl TeamStore {
    /// Open or initialize a store at `<workspace>/<state_dir>`.
    pub fn for_workspace(workspace: impl AsRef<Path>, state_dir: Option<&str>) -> Self {
        let dir_name = state_dir.unwrap_or(DEFAULT_STATE_DIR);
        Self {
            root: workspace.as_ref().join(dir_name),
        }
    }

    /// Open a store directly at an explicit state root path.
    pub fn at_root(root: impl AsRef<Path>) -> Self {
        Self {
            root: root.as_ref().to_path_buf(),
        }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn team_dir(&self, team_id: &str) -> PathBuf {
        self.root.join(team_id)
    }

    pub fn archive_root(&self) -> PathBuf {
        self.root.join("archive")
    }

    pub fn archive_dir(&self, team_id: &str) -> PathBuf {
        self.archive_root().join(team_id)
    }

    pub fn team_file(&self, team_id: &str) -> PathBuf {
        self.team_dir(team_id).join("team.json")
    }

    pub fn inbox_dir(&self, team_id: &str) -> PathBuf {
        self.team_dir(team_id).join("inbox")
    }

    pub fn inbox_file(&self, team_id: &str, agent: &str) -> PathBuf {
        self.inbox_dir(team_id).join(format!("{agent}.jsonl"))
    }

    /// Persist `TeamState` atomically (write temp file -> sync -> rename).
    pub fn save_team(&self, team: &TeamState) -> Result<()> {
        let dir = self.team_dir(&team.id);
        fs::create_dir_all(dir.join("inbox"))?;
        let target = dir.join("team.json");
        let tmp = dir.join(format!(
            ".team.json.{}.{}.tmp",
            std::process::id(),
            now_ms()
        ));
        let bytes = serde_json::to_vec_pretty(team)?;
        {
            let mut file = File::create(&tmp)?;
            file.write_all(&bytes)?;
            file.sync_all()?;
        }
        fs::rename(&tmp, &target)?;
        Ok(())
    }

    /// Load an active team from disk.
    pub fn load_team(&self, team_id: &str) -> Result<TeamState> {
        let path = self.team_file(team_id);
        if !path.is_file() {
            return Err(TeamsError::TeamNotFound(team_id.to_string()));
        }
        let raw = fs::read(&path)?;
        let team: TeamState = serde_json::from_slice(&raw)?;
        Ok(team)
    }

    /// List all active teams under the state root.
    pub fn list_active_teams(&self) -> Result<Vec<TeamState>> {
        Self::scan_teams_in(&self.root, true)
    }

    /// List all archived teams under `<state_root>/archive/`.
    pub fn list_archived_teams(&self) -> Result<Vec<TeamState>> {
        Self::scan_teams_in(&self.archive_root(), false)
    }

    fn scan_teams_in(dir: &Path, skip_archive: bool) -> Result<Vec<TeamState>> {
        if !dir.is_dir() {
            return Ok(Vec::new());
        }
        let mut out = Vec::new();
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let name = entry.file_name();
            if skip_archive && name == "archive" {
                continue;
            }
            let path = entry.path().join("team.json");
            if path.is_file() {
                if let Ok(raw) = fs::read(&path) {
                    if let Ok(team) = serde_json::from_slice::<TeamState>(&raw) {
                        out.push(team);
                    }
                }
            }
        }
        out.sort_by_key(|t| std::cmp::Reverse(t.created_at));
        Ok(out)
    }

    /// Find the active team currently led by `captain_session_id`.
    pub fn find_by_captain(&self, captain_session_id: &str) -> Result<Option<TeamState>> {
        let teams = self.list_active_teams()?;
        Ok(teams
            .into_iter()
            .find(|t| t.captain_session_id == captain_session_id))
    }

    /// Move an active team directory into `<state_root>/archive/<teamId>/` (retaining full history).
    pub fn archive_team(&self, team: &TeamState) -> Result<PathBuf> {
        self.save_team(team)?;
        fs::create_dir_all(self.archive_root())?;
        let src = self.team_dir(&team.id);
        let mut dst = self.archive_dir(&team.id);
        if dst.exists() {
            dst = self
                .archive_root()
                .join(format!("{}-{}", team.id, now_ms()));
        }
        fs::rename(&src, &dst)?;
        Ok(dst)
    }

    /// Append one message to `<team>/inbox/<to>.jsonl`.
    pub fn append_message(&self, team_id: &str, msg: &TeamMessage) -> Result<()> {
        let inbox = self.inbox_dir(team_id);
        fs::create_dir_all(&inbox)?;
        let path = self.inbox_file(team_id, &msg.to);
        let mut line = serde_json::to_string(msg)?;
        line.push('\n');
        let mut file = OpenOptions::new().create(true).append(true).open(path)?;
        file.write_all(line.as_bytes())?;
        file.sync_all()?;
        Ok(())
    }

    /// Read all messages in `<team>/inbox/<agent>.jsonl`.
    pub fn read_inbox(&self, team_id: &str, agent: &str) -> Result<Vec<TeamMessage>> {
        let path = self.inbox_file(team_id, agent);
        read_jsonl(&path)
    }

    /// Rewrite `<team>/inbox/<agent>.jsonl` atomically (used for marking delivered/read/discarded).
    pub fn write_inbox(&self, team_id: &str, agent: &str, messages: &[TeamMessage]) -> Result<()> {
        let inbox = self.inbox_dir(team_id);
        fs::create_dir_all(&inbox)?;
        let target = self.inbox_file(team_id, agent);
        let tmp = inbox.join(format!(
            ".{agent}.jsonl.{}.{}.tmp",
            std::process::id(),
            now_ms()
        ));
        {
            let mut file = File::create(&tmp)?;
            for msg in messages {
                let mut line = serde_json::to_string(msg)?;
                line.push('\n');
                file.write_all(line.as_bytes())?;
            }
            file.sync_all()?;
        }
        fs::rename(&tmp, &target)?;
        Ok(())
    }

    /// Count all messages across all inbox files of a team (active or archived).
    pub fn count_team_messages(&self, team_dir: &Path) -> usize {
        let inbox = team_dir.join("inbox");
        let Ok(entries) = fs::read_dir(inbox) else {
            return 0;
        };
        let mut total = 0usize;
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("jsonl") {
                if let Ok(msgs) = read_jsonl(&path) {
                    total += msgs.len();
                }
            }
        }
        total
    }

    /// Read all messages across all inbox files of a team (active or archived), sorted newest-first.
    pub fn read_all_team_messages(&self, team_dir: &Path) -> Vec<TeamMessage> {
        let inbox = team_dir.join("inbox");
        let Ok(entries) = fs::read_dir(inbox) else {
            return Vec::new();
        };
        let mut all = Vec::new();
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("jsonl") {
                if let Ok(msgs) = read_jsonl(&path) {
                    all.extend(msgs);
                }
            }
        }
        all.sort_by_key(|m| std::cmp::Reverse(m.ts));
        all
    }

    /// Count unread (undiscarded & unread) messages for one member.
    pub fn count_unread(&self, team_dir: &Path, agent: &str) -> usize {
        let path = team_dir.join("inbox").join(format!("{agent}.jsonl"));
        read_jsonl(&path).map_or(0, |msgs| {
            msgs.into_iter()
                .filter(|m| m.read_at.is_none() && m.discarded_at.is_none())
                .count()
        })
    }
}

fn read_jsonl(path: &Path) -> Result<Vec<TeamMessage>> {
    if !path.is_file() {
        return Ok(Vec::new());
    }
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let mut out = Vec::new();
    for line in reader.lines() {
        let line = line?;
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if let Ok(msg) = serde_json::from_str::<TeamMessage>(trimmed) {
            out.push(msg);
        }
    }
    Ok(out)
}
