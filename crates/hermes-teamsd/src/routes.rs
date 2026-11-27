use crate::auth::verify_request_auth;
use crate::state::AppState;
use axum::extract::{Path as AxumPath, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use futures_core::Stream;
use hermes_teams_core::operations::{
    AddMemberInput, AmendTaskInput, CreateTaskInput, CreateTeamInput, EditPlanInput,
    ReassignTaskInput, SendMessageInput, UpdateTaskInput,
};
use hermes_teams_core::{collect_workspace_activity, TeamsError, WorkspaceActivityPayload};
use serde::Deserialize;
use serde_json::{json, Value};
use std::convert::Infallible;
use std::path::PathBuf;
use std::time::Duration;
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::StreamExt;

pub fn build_router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health_handler))
        .route("/api/v1/state", get(state_handler))
        .route("/api/v1/events", get(events_sse_handler))
        .route("/api/v1/inbox", get(inbox_handler))
        .route("/api/v1/directives/poll", get(poll_directives_handler))
        .route("/api/v1/members/bind", post(bind_member_handler))
        .route("/api/v1/tools/{tool_name}", post(dispatch_tool_handler))
        .route("/api/v1/plan/approve", post(plan_approve_handler))
        .route("/api/v1/plan/edit", post(plan_edit_handler))
        .route("/api/v1/plan/feedback", post(plan_feedback_handler))
        .route("/api/v1/plan/discard", post(plan_discard_handler))
        .route("/api/v1/team/halt", post(team_halt_handler))
        .route("/api/v1/team/resume", post(team_resume_handler))
        .with_state(state)
}

async fn health_handler(State(state): State<AppState>) -> Json<Value> {
    Json(json!({
        "ok": true,
        "service": "hermes-teamsd",
        "version": env!("CARGO_PKG_VERSION"),
        "uptimeSecs": state.uptime_secs(),
        "authMode": format!("{:?}", state.auth_mode()),
    }))
}

#[derive(Debug, Deserialize)]
struct StateQuery {
    #[serde(default)]
    workspace: Option<String>,
    #[serde(default)]
    session_id: Option<String>,
}

async fn state_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(q): Query<StateQuery>,
) -> Result<Json<WorkspaceActivityPayload>, ApiError> {
    verify_request_auth(state.auth_mode(), &headers)?;

    let workspaces: Vec<PathBuf> =
        if let Some(ws) = q.workspace.as_deref().filter(|s| !s.trim().is_empty()) {
            vec![state.resolve_workspace(Some(ws))]
        } else {
            state.known_workspaces()
        };

    let mut combined = WorkspaceActivityPayload {
        teams: Vec::new(),
        archived: Vec::new(),
    };
    for ws in workspaces {
        let snap = state.with_engine(&ws, |engine| {
            collect_workspace_activity(engine.store(), q.session_id.as_deref())
        });
        combined.teams.extend(snap.teams);
        combined.archived.extend(snap.archived);
    }

    combined
        .teams
        .sort_by_key(|t| std::cmp::Reverse(t.created_at));
    combined
        .archived
        .sort_by_key(|t| std::cmp::Reverse(t.created_at));
    Ok(Json(combined))
}

async fn events_sse_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(q): Query<StateQuery>,
) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, ApiError> {
    verify_request_auth(state.auth_mode(), &headers)?;
    let ws_filter = q
        .workspace
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
    let rx = state.subscribe_events();
    let stream = BroadcastStream::new(rx).filter_map(move |item| match item {
        Ok(ev) => {
            if let Some(ref filter) = ws_filter {
                if &ev.workspace != filter {
                    return None;
                }
            }
            let data = serde_json::to_string(&ev).unwrap_or_else(|_| "{}".to_string());
            Some(Ok(Event::default().event("team_update").data(data)))
        }
        Err(_) => None,
    });

    Ok(Sse::new(stream).keep_alive(
        KeepAlive::new()
            .interval(Duration::from_secs(15))
            .text("keep-alive"),
    ))
}

#[derive(Debug, Deserialize)]
struct InboxQuery {
    #[serde(default)]
    workspace: Option<String>,
    team_id: String,
    agent: String,
}

async fn inbox_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(q): Query<InboxQuery>,
) -> Result<Json<Value>, ApiError> {
    verify_request_auth(state.auth_mode(), &headers)?;
    let ws = state.resolve_workspace(q.workspace.as_deref());
    let msgs = state.with_engine(&ws, |engine| {
        engine.store().read_inbox(&q.team_id, &q.agent)
    })?;
    Ok(Json(json!({ "ok": true, "messages": msgs })))
}

#[derive(Debug, Deserialize)]
struct PollQuery {
    #[serde(default)]
    workspace: Option<String>,
}

async fn poll_directives_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(q): Query<PollQuery>,
) -> Result<Json<Value>, ApiError> {
    verify_request_auth(state.auth_mode(), &headers)?;
    let items = state.drain_directives(q.workspace.as_deref());
    Ok(Json(json!({ "ok": true, "directives": items })))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BindMemberBody {
    #[serde(default)]
    workspace: Option<String>,
    team_id: String,
    member_name: String,
    subagent_id: String,
    #[serde(default)]
    spawn_error: Option<String>,
}

async fn bind_member_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<BindMemberBody>,
) -> Result<Json<Value>, ApiError> {
    verify_request_auth(state.auth_mode(), &headers)?;
    let ws = state.resolve_workspace(body.workspace.as_deref());
    let out = state.with_engine(&ws, |engine| {
        engine.bind_member_session(
            &body.team_id,
            &body.member_name,
            &body.subagent_id,
            body.spawn_error,
        )
    })?;
    state.publish_event(&ws, Some(out.team.id.clone()), "member_bound");
    Ok(Json(json!({
        "ok": true,
        "summary": out.summary,
        "team": out.team,
        "directives": out.directives,
    })))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ToolEnvelope {
    #[serde(default)]
    workspace: Option<String>,
    #[serde(default)]
    actor: Option<String>,
    #[serde(default)]
    args: Value,
}

async fn dispatch_tool_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    AxumPath(tool_name): AxumPath<String>,
    Json(env): Json<ToolEnvelope>,
) -> Result<Json<Value>, ApiError> {
    verify_request_auth(state.auth_mode(), &headers)?;
    let ws = state.resolve_workspace(env.workspace.as_deref());
    let actor = env
        .actor
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or("captain")
        .to_string();

    match tool_name.as_str() {
        "agent_teams_create" => {
            let input: CreateTeamInput = serde_json::from_value(env.args)?;
            let out = state.with_engine(&ws, |engine| engine.create_team(input))?;
            state.publish_event(&ws, Some(out.team.id.clone()), "team_created");
            Ok(Json(serde_json::to_value(out)?))
        }
        "agent_teams_approve" => {
            let team_id = extract_str(&env.args, "teamId", "team_id")?;
            let out = state.with_engine(&ws, |engine| engine.approve_team(&team_id))?;
            state.publish_event(&ws, Some(out.team.id.clone()), "team_approved");
            Ok(Json(serde_json::to_value(out)?))
        }
        "agent_teams_edit_plan" => {
            let input: EditPlanInput = serde_json::from_value(env.args)?;
            let out = state.with_engine(&ws, |engine| engine.edit_plan(input))?;
            state.publish_event(&ws, Some(out.team.id.clone()), "plan_edited");
            Ok(Json(serde_json::to_value(out)?))
        }
        "agent_teams_add_member" => {
            let input: AddMemberInput = serde_json::from_value(env.args)?;
            let out = state.with_engine(&ws, |engine| engine.add_member(input))?;
            state.publish_event(&ws, Some(out.team.id.clone()), "member_added");
            Ok(Json(serde_json::to_value(out)?))
        }
        "agent_teams_remove_member" => {
            let team_id = extract_str(&env.args, "teamId", "team_id")?;
            let member = extract_str(&env.args, "name", "member_name")?;
            let out = state.with_engine(&ws, |engine| engine.remove_member(&team_id, &member))?;
            state.publish_event(&ws, Some(out.team.id.clone()), "member_removed");
            Ok(Json(serde_json::to_value(out)?))
        }
        "agent_teams_create_task" => {
            let input: CreateTaskInput = serde_json::from_value(env.args)?;
            let out = state.with_engine(&ws, |engine| engine.create_task(input))?;
            state.publish_event(&ws, Some(out.team.id.clone()), "task_created");
            Ok(Json(serde_json::to_value(out)?))
        }
        "agent_teams_reassign_task" => {
            let input: ReassignTaskInput = serde_json::from_value(env.args)?;
            let out = state.with_engine(&ws, |engine| engine.reassign_task(&input))?;
            state.publish_event(&ws, Some(out.team.id.clone()), "task_reassigned");
            Ok(Json(serde_json::to_value(out)?))
        }
        "agent_teams_claim_task" => {
            let team_id = extract_str(&env.args, "teamId", "team_id")?;
            let task_id = extract_str(&env.args, "taskId", "task_id")?;
            let claimer = env
                .args
                .get("assignee")
                .and_then(Value::as_str)
                .map_or(actor, ToString::to_string);
            let out = state.with_engine(&ws, |engine| {
                engine.claim_task_op(&team_id, &task_id, &claimer)
            })?;
            state.publish_event(&ws, Some(out.team.id.clone()), "task_claimed");
            Ok(Json(serde_json::to_value(out)?))
        }
        "agent_teams_update_task" => {
            let mut val = env.args;
            if let Some(obj) = val.as_object_mut() {
                obj.entry("actor").or_insert_with(|| Value::String(actor));
            }
            let input: UpdateTaskInput = serde_json::from_value(val)?;
            let out = state.with_engine(&ws, |engine| engine.update_task(input, Some(&ws)))?;
            state.publish_event(&ws, Some(out.team.id.clone()), "task_updated");
            Ok(Json(serde_json::to_value(out)?))
        }
        "agent_teams_amend_task" => {
            let input: AmendTaskInput = serde_json::from_value(env.args)?;
            let out = state.with_engine(&ws, |engine| engine.amend_task(input))?;
            state.publish_event(&ws, Some(out.team.id.clone()), "task_amended");
            Ok(Json(serde_json::to_value(out)?))
        }
        "agent_teams_send_message" => {
            let mut val = env.args;
            if let Some(obj) = val.as_object_mut() {
                obj.entry("from").or_insert_with(|| Value::String(actor));
            }
            let input: SendMessageInput = serde_json::from_value(val)?;
            let tid = input.team_id.clone();
            let (msg, directives) =
                state.with_engine(&ws, |engine| engine.send_message_op(input))?;
            state.publish_event(&ws, Some(tid), "message_sent");
            Ok(Json(json!({
                "ok": true,
                "message": msg,
                "directives": directives,
            })))
        }
        "agent_teams_status" => {
            let team_id = extract_str(&env.args, "teamId", "team_id")?;
            let status = state.with_engine(&ws, |engine| engine.team_status(&team_id))?;
            Ok(Json(status))
        }
        "agent_teams_resume" => {
            let team_id = extract_str(&env.args, "teamId", "team_id")?;
            let reason = extract_str(&env.args, "reason", "resumeReason")?;
            let out = state.with_engine(&ws, |engine| engine.resume_team(&team_id, &reason))?;
            state.publish_event(&ws, Some(out.team.id.clone()), "team_resumed");
            Ok(Json(serde_json::to_value(out)?))
        }
        "agent_teams_delete" => {
            let team_id = extract_str(&env.args, "teamId", "team_id")?;
            let reason = env
                .args
                .get("reason")
                .and_then(Value::as_str)
                .map(ToString::to_string);
            let out = state.with_engine(&ws, |engine| {
                engine.delete_and_archive(&team_id, reason.as_deref())
            })?;
            state.publish_event(&ws, Some(out.team.id.clone()), "team_archived");
            Ok(Json(serde_json::to_value(out)?))
        }
        other => Err(ApiError::bad_request(format!(
            "unknown agent_teams tool '{other}'"
        ))),
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TeamActionBody {
    #[serde(default)]
    workspace: Option<String>,
    team_id: String,
    #[serde(default)]
    reason: Option<String>,
}

async fn plan_approve_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<TeamActionBody>,
) -> Result<Json<Value>, ApiError> {
    verify_request_auth(state.auth_mode(), &headers)?;
    let ws = state.resolve_workspace(body.workspace.as_deref());
    let out = state.with_engine(&ws, |engine| engine.approve_team(&body.team_id))?;
    state.enqueue_directives(&ws, out.directives.clone());
    state.publish_event(&ws, Some(out.team.id.clone()), "team_approved");
    Ok(Json(serde_json::to_value(out)?))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WebEditPlanBody {
    #[serde(default)]
    workspace: Option<String>,
    #[serde(flatten)]
    input: EditPlanInput,
}

async fn plan_edit_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<WebEditPlanBody>,
) -> Result<Json<Value>, ApiError> {
    verify_request_auth(state.auth_mode(), &headers)?;
    let ws = state.resolve_workspace(body.workspace.as_deref());
    let out = state.with_engine(&ws, |engine| engine.edit_plan(body.input))?;
    state.publish_event(&ws, Some(out.team.id.clone()), "plan_edited");
    Ok(Json(serde_json::to_value(out)?))
}

async fn plan_feedback_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<TeamActionBody>,
) -> Result<Json<Value>, ApiError> {
    verify_request_auth(state.auth_mode(), &headers)?;
    let ws = state.resolve_workspace(body.workspace.as_deref());
    let out = state.with_engine(&ws, |engine| engine.request_plan_feedback(&body.team_id))?;
    state.enqueue_directives(&ws, out.directives.clone());
    state.publish_event(&ws, Some(out.team.id.clone()), "plan_feedback");
    Ok(Json(serde_json::to_value(out)?))
}

async fn plan_discard_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<TeamActionBody>,
) -> Result<Json<Value>, ApiError> {
    verify_request_auth(state.auth_mode(), &headers)?;
    let ws = state.resolve_workspace(body.workspace.as_deref());
    let out = state.with_engine(&ws, |engine| engine.discard_staged_plan(&body.team_id))?;
    state.enqueue_directives(&ws, out.directives.clone());
    state.publish_event(&ws, Some(out.team.id.clone()), "plan_discarded");
    Ok(Json(serde_json::to_value(out)?))
}

async fn team_halt_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<TeamActionBody>,
) -> Result<Json<Value>, ApiError> {
    verify_request_auth(state.auth_mode(), &headers)?;
    let ws = state.resolve_workspace(body.workspace.as_deref());
    let out = state.with_engine(&ws, |engine| {
        engine.halt_team(&body.team_id, body.reason.as_deref())
    })?;
    state.enqueue_directives(&ws, out.directives.clone());
    state.publish_event(&ws, Some(out.team.id.clone()), "team_halted");
    Ok(Json(serde_json::to_value(out)?))
}

async fn team_resume_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<TeamActionBody>,
) -> Result<Json<Value>, ApiError> {
    verify_request_auth(state.auth_mode(), &headers)?;
    let ws = state.resolve_workspace(body.workspace.as_deref());
    let reason = body.reason.as_deref().unwrap_or("resumed from WebUI");
    let out = state.with_engine(&ws, |engine| engine.resume_team(&body.team_id, reason))?;
    state.enqueue_directives(&ws, out.directives.clone());
    state.publish_event(&ws, Some(out.team.id.clone()), "team_resumed");
    Ok(Json(serde_json::to_value(out)?))
}

fn extract_str(val: &Value, camel: &str, snake: &str) -> Result<String, ApiError> {
    val.get(camel)
        .or_else(|| val.get(snake))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(ToString::to_string)
        .ok_or_else(|| ApiError::bad_request(format!("missing required string field '{camel}'")))
}

/// Structured HTTP error mapping domain errors cleanly to JSON responses.
struct ApiError {
    status: StatusCode,
    message: String,
}

impl ApiError {
    fn bad_request(message: String) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            message,
        }
    }
}

impl From<StatusCode> for ApiError {
    fn from(status: StatusCode) -> Self {
        let message = match status {
            StatusCode::UNAUTHORIZED => "unauthorized: missing or invalid sidecar token",
            StatusCode::SERVICE_UNAVAILABLE => "sidecar token file missing or unreadable",
            _ => "request failed",
        };
        Self {
            status,
            message: message.to_string(),
        }
    }
}

impl From<serde_json::Error> for ApiError {
    fn from(err: serde_json::Error) -> Self {
        Self::bad_request(format!("invalid JSON arguments: {err}"))
    }
}

impl From<TeamsError> for ApiError {
    fn from(err: TeamsError) -> Self {
        let status = match &err {
            TeamsError::TeamNotFound(_)
            | TeamsError::MemberNotFound(_)
            | TeamsError::TaskNotFound(_) => StatusCode::NOT_FOUND,
            TeamsError::PermissionDenied(_) => StatusCode::FORBIDDEN,
            TeamsError::CaptainAlreadyHasTeam { .. }
            | TeamsError::TeamAlreadyExists(_)
            | TeamsError::MemberAlreadyExists(_)
            | TeamsError::MemberBusy { .. }
            | TeamsError::StaleAttempt { .. }
            | TeamsError::TeamStaged(_)
            | TeamsError::TeamHalted(_)
            | TeamsError::TaskTerminal(_, _) => StatusCode::CONFLICT,
            TeamsError::Io(_) | TeamsError::Json(_) | TeamsError::Yaml(_) => {
                StatusCode::INTERNAL_SERVER_ERROR
            }
            _ => StatusCode::BAD_REQUEST,
        };
        Self {
            status,
            message: err.to_string(),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (
            self.status,
            Json(json!({
                "ok": false,
                "error": self.message,
            })),
        )
            .into_response()
    }
}
