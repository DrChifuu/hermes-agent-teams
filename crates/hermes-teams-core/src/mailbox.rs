use crate::error::{Result, TeamsError};
use crate::store::TeamStore;
use crate::types::{gen_token, now_ms, TaskStatus, TeamMessage, TeamState};

/// Options attached to a durable mailbox message.
#[derive(Debug, Clone, Default)]
pub struct SendMessageOptions {
    pub task_id: Option<String>,
    pub attempt_id: Option<String>,
    pub source_task_id: Option<String>,
    pub source_attempt_id: Option<String>,
    pub source_task_status: Option<TaskStatus>,
}

/// Append a durable P2P message into `<team>/inbox/<to>.jsonl`.
/// Rejects spoofed senders or unknown recipients.
pub fn send_message(
    store: &TeamStore,
    team: &TeamState,
    from: &str,
    to: &str,
    content: &str,
    opts: SendMessageOptions,
) -> Result<TeamMessage> {
    let from = from.trim();
    let to = to.trim();
    let content = content.trim();

    if from.is_empty() || to.is_empty() || content.is_empty() {
        return Err(TeamsError::InvalidInput(
            "from, to, and content must be non-empty".to_string(),
        ));
    }

    if from != "captain" && team.find_member(from).is_none() {
        return Err(TeamsError::PermissionDenied(format!(
            "unknown sender '{from}'"
        )));
    }
    if to != "captain" && team.find_member(to).is_none() {
        return Err(TeamsError::MemberNotFound(to.to_string()));
    }

    let existing = store.read_inbox(&team.id, to)?;
    let seq = (existing.len() as u64) + 1;
    let msg = TeamMessage {
        id: gen_token("msg", seq),
        from: from.to_string(),
        to: to.to_string(),
        content: content.to_string(),
        ts: now_ms(),
        delivery_claimed_at: None,
        delivered_at: None,
        read_at: None,
        task_id: opts.task_id,
        attempt_id: opts.attempt_id,
        source_task_id: opts.source_task_id,
        source_attempt_id: opts.source_attempt_id,
        source_task_status: opts.source_task_status,
        discarded_at: None,
    };

    store.append_message(&team.id, &msg)?;
    Ok(msg)
}

/// Mark a specific message as delivered to the live recipient session.
pub fn mark_delivered(
    store: &TeamStore,
    team_id: &str,
    recipient: &str,
    msg_id: &str,
) -> Result<()> {
    let mut messages = store.read_inbox(team_id, recipient)?;
    let now = now_ms();
    let mut changed = false;
    for msg in &mut messages {
        if msg.id == msg_id && msg.delivered_at.is_none() {
            msg.delivered_at = Some(now);
            changed = true;
        }
    }
    if changed {
        store.write_inbox(team_id, recipient, &messages)?;
    }
    Ok(())
}

/// Mark all unread undiscarded messages in `agent`'s inbox as read and return them.
pub fn consume_unread(store: &TeamStore, team_id: &str, agent: &str) -> Result<Vec<TeamMessage>> {
    let mut messages = store.read_inbox(team_id, agent)?;
    let now = now_ms();
    let mut unread = Vec::new();
    for msg in &mut messages {
        if msg.read_at.is_none() && msg.discarded_at.is_none() {
            msg.read_at = Some(now);
            unread.push(msg.clone());
        }
    }
    if !unread.is_empty() {
        store.write_inbox(team_id, agent, &messages)?;
    }
    Ok(unread)
}

/// Discard pending undelivered messages scoped to a revoked attempt (or removed member).
pub fn discard_for_attempt(
    store: &TeamStore,
    team_id: &str,
    recipient: &str,
    attempt_id: Option<&str>,
) -> Result<usize> {
    let mut messages = store.read_inbox(team_id, recipient)?;
    let now = now_ms();
    let mut count = 0usize;
    for msg in &mut messages {
        if msg.discarded_at.is_some() || msg.read_at.is_some() {
            continue;
        }
        let matches = match attempt_id {
            Some(aid) => msg.attempt_id.as_deref() == Some(aid),
            None => true,
        };
        if matches {
            msg.discarded_at = Some(now);
            count += 1;
        }
    }
    if count > 0 {
        store.write_inbox(team_id, recipient, &messages)?;
    }
    Ok(count)
}
