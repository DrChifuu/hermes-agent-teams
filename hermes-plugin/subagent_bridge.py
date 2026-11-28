"""Bridge between Rust hermes-teamsd EngineDirectives and native Hermes subagents."""

from __future__ import annotations

import contextlib
import json
import logging
import os
import queue
import threading
import time
import urllib.request
from typing import Any

from .client import TeamsDaemonClient
from .prompts import MEMBER_INSTRUCTIONS

_log = logging.getLogger(__name__)


@contextlib.contextmanager
def _maybe_bind_parent(parent: Any):
    """Unconditionally bind the root Captain `parent` for the duration of a subagent lifecycle call.

    Must NOT skip binding when called from inside a delegated `leaf` child turn — otherwise Hermes
    would treat the leaf child as the parent and refuse to spawn the next teammate.
    """
    if parent is None:
        yield
        return
    try:
        from agent.subagent_lifecycle import bind_subagent_parent
    except Exception:  # noqa: BLE001
        yield
        return

    with bind_subagent_parent(parent):
        yield


@contextlib.contextmanager
def _override_parent_reasoning(parent: Any, reasoning_effort: str | None):
    """Temporarily override `parent.reasoning_config` during synchronous `lifecycle.launch(req)`.

    Hermes's `SubagentLaunchRequest` does not expose a `reasoning_effort` field; `_child_reasoning`
    reads `parent.reasoning_config` while building the child `AIAgent` synchronously on the calling
    thread before submitting to the executor. Restoring immediately in `finally` keeps the Captain's
    own reasoning effort untouched while giving each teammate its chosen effort level.
    """
    effort = (reasoning_effort or "").strip() if isinstance(reasoning_effort, str) else ""
    if parent is None or not effort:
        yield
        return
    try:
        from hermes_constants import parse_reasoning_effort

        parsed = parse_reasoning_effort(effort)
    except Exception:  # noqa: BLE001
        parsed = None
    if parsed is None:
        yield
        return

    sentinel = object()
    prev = getattr(parent, "reasoning_config", sentinel)
    try:
        setattr(parent, "reasoning_config", parsed)
        yield
    finally:
        try:
            if prev is sentinel:
                delattr(parent, "reasoning_config")
            else:
                setattr(parent, "reasoning_config", prev)
        except Exception:  # noqa: BLE001
            pass


_FALLBACK_PARENT_TOOLSETS: tuple[str, ...] = (
    "terminal",
    "file",
    "web",
    "skills",
    "vision",
    "code_execution",
    "agent_teams",
)

_MEMBER_REQUIRED_TOOLS: tuple[str, ...] = (
    "agent_teams_claim_task",
    "agent_teams_update_task",
    "agent_teams_send_message",
    "agent_teams_status",
)


def _ensure_parent_has_agent_teams(parent: Any) -> None:
    """Guarantee that Hermes's `_resolve_child_toolsets(parent, ...)` inherits `agent_teams`.

    When `AIAgent` is created with default `enabled_toolsets=None` (e.g. fallback parent after
    a server restart), its `valid_tool_names` omits plugin toolsets unless explicitly present,
    causing child subagents to fail with `'agent_teams_update_task' is not available`.
    """
    if parent is None:
        return
    try:
        enabled = getattr(parent, "enabled_toolsets", None)
        if enabled is not None:
            if isinstance(enabled, list):
                if "agent_teams" not in enabled:
                    enabled.append("agent_teams")
            elif isinstance(enabled, (tuple, set)):
                if "agent_teams" not in enabled:
                    setattr(parent, "enabled_toolsets", list(enabled) + ["agent_teams"])
        else:
            valid = getattr(parent, "valid_tool_names", None)
            if isinstance(valid, list):
                for tname in _MEMBER_REQUIRED_TOOLS:
                    if tname not in valid:
                        valid.append(tname)
            elif isinstance(valid, set):
                valid.update(_MEMBER_REQUIRED_TOOLS)
    except Exception as exc:  # noqa: BLE001
        _log.debug("Could not augment parent agent toolsets: %s", exc)


def _summarize_tool_args(tool_name: str, args: Any) -> str:
    if not isinstance(args, dict) or not args:
        return ""
    for key in ("command", "path", "file_path", "target", "query", "url", "taskId", "task_id", "to", "skill"):
        val = args.get(key)
        if isinstance(val, str) and val.strip():
            clean = " ".join(val.strip().split())
            return clean[:160]
    parts = []
    for k, v in list(args.items())[:3]:
        if isinstance(v, (str, int, float, bool)):
            parts.append(f"{k}={str(v)[:60]}")
    return ", ".join(parts)[:160]


def _build_fallback_parent_agent(captain_session_id: str, workspace: str) -> Any | None:
    """Best-effort fallback parent AIAgent when WebUI Approve & Run runs after a server restart."""
    try:
        from run_agent import AIAgent

        agent = AIAgent(
            session_id=captain_session_id or "hermes-agent-teams-captain",
            quiet_mode=True,
            enabled_toolsets=list(_FALLBACK_PARENT_TOOLSETS),
        )
        _ensure_parent_has_agent_teams(agent)
        if workspace:
            setattr(agent, "_working_directory", workspace)
        return agent
    except Exception as exc:  # noqa: BLE001
        _log.debug("Fallback AIAgent creation unavailable: %s", exc)
        return None


class SubagentBridge:
    """Executes Rust core directives against native Hermes `ctx.subagent_lifecycle` and `delegate_task`."""

    def __init__(self, ctx: Any, client: TeamsDaemonClient) -> None:
        self.ctx = ctx
        self.client = client
        self._handles: dict[str, Any] = {}  # subagent_id -> SubagentHandle
        # subagent_id -> (team_id, member_name, workspace, tool_filter, captain_session_id)
        self._member_lookup: dict[str, tuple[str, str, str, dict[str, Any] | None, str]] = {}
        # child_session_id -> subagent_id (populated by subagent_start hook)
        self._session_to_subagent: dict[str, str] = {}
        # Track terminal subagents already reported so we reconcile each handle once.
        self._terminal_reported: set[str] = set()
        # (team_id, member_name) -> latest bound subagent_id (prevents old exiting turns from triggering recovery)
        self._latest_subagent_for_member: dict[tuple[str, str], str] = {}
        # subagent_id -> (wake_directive, workspace) for P2P steers in case the child exits before reading them
        self._pending_steers: dict[str, tuple[dict[str, Any], str]] = {}
        # Bounded recovery guard: at most 1 automatic recovery per (team_id, member_name, task_id)
        self._auto_recovered_tasks: set[tuple[str, str, str]] = set()
        self._captain_agents: dict[str, Any] = {}  # captain_session_id -> AIAgent
        self._last_captain_agent: Any = None
        self._lock = threading.Lock()
        self._launch_lock = threading.Lock()
        self._stop_event = threading.Event()
        self._poller: threading.Thread | None = None
        self._status_tick: int = 0
        self._activity_queue: queue.Queue[dict[str, Any]] = queue.Queue(maxsize=256)
        self._activity_worker: threading.Thread | None = None

    def register_child_session(self, child_session_id: str, child_subagent_id: str) -> None:
        csid = (child_session_id or "").strip()
        sub_id = (child_subagent_id or "").strip()
        if not csid or not sub_id:
            return
        with self._lock:
            self._session_to_subagent[csid] = sub_id
            if len(self._session_to_subagent) > 256:
                oldest = next(iter(self._session_to_subagent))
                self._session_to_subagent.pop(oldest, None)

    def lookup_member_by_ids(
        self, *, subagent_id: str = "", session_id: str = "", task_id: str = ""
    ) -> tuple[str, str, str, str, dict[str, Any] | None] | None:
        """Return `(team_id, member_name, workspace, subagent_id, tool_filter)` if tracked."""
        with self._lock:
            candidates = [subagent_id.strip(), task_id.strip()]
            csid = session_id.strip()
            if csid and csid in self._session_to_subagent:
                candidates.append(self._session_to_subagent[csid])
            for cand in candidates:
                if cand and cand in self._member_lookup:
                    team_id, mname, ws, tfilter, _cs = self._member_lookup[cand]
                    return (team_id, mname, ws, cand, tfilter)
        return None

    def _ensure_activity_worker(self) -> None:
        worker = self._activity_worker
        if worker is not None and worker.is_alive():
            return
        with self._lock:
            if self._activity_worker is not None and self._activity_worker.is_alive():
                return
            self._activity_worker = threading.Thread(
                target=self._activity_loop,
                name="hat-activity-worker",
                daemon=True,
            )
            self._activity_worker.start()

    def _activity_loop(self) -> None:
        while not self._stop_event.is_set():
            try:
                payload = self._activity_queue.get(timeout=0.5)
            except queue.Empty:
                continue
            try:
                self.client.report_activity(**payload)
            except Exception:  # noqa: BLE001
                pass
            finally:
                self._activity_queue.task_done()

    def emit_activity_async(self, **kwargs: Any) -> None:
        """Non-blocking FIFO telemetry enqueue to a single persistent worker thread."""
        self._ensure_activity_worker()
        try:
            self._activity_queue.put_nowait(kwargs)
        except queue.Full:
            try:
                self._activity_queue.get_nowait()
                self._activity_queue.task_done()
            except queue.Empty:
                pass
            try:
                self._activity_queue.put_nowait(kwargs)
            except queue.Full:
                pass

    @staticmethod
    def _extract_effective_tool_names(tool_name: str, args: Any) -> list[str]:
        """Return `[tool_name]` plus any inner tool names when invoked via Hermes's `tool_call` bridge."""
        names = [tool_name]
        if tool_name == "tool_call" and isinstance(args, dict):
            inner = args.get("name")
            if isinstance(inner, str) and inner.strip():
                names.append(inner.strip())
            calls = args.get("calls")
            if isinstance(calls, list):
                for entry in calls:
                    if isinstance(entry, dict) and isinstance(entry.get("name"), str) and entry["name"].strip():
                        names.append(entry["name"].strip())
        return names

    def handle_pre_tool_call(
        self,
        *,
        tool_name: str,
        args: Any,
        session_id: str = "",
        task_id: str = "",
    ) -> dict[str, Any] | None:
        info = self.lookup_member_by_ids(session_id=session_id, task_id=task_id)
        if info is None:
            return None
        team_id, member_name, workspace, subagent_id, tool_filter = info
        arg_preview = _summarize_tool_args(tool_name, args)

        # Enforce owner-declared tool policy (e.g. readonly reviewer/architect cannot write_file/patch),
        # checking both direct tool calls and inner tools routed through Hermes's `tool_call` bridge.
        if isinstance(tool_filter, dict):
            deny = set(tool_filter.get("deny") or [])
            allow = tool_filter.get("allow")
            allow_set = set(allow) if isinstance(allow, list) and allow else None
            effective_names = self._extract_effective_tool_names(tool_name, args)
            for candidate in effective_names:
                if candidate in deny or (allow_set is not None and candidate != "tool_call" and candidate not in allow_set):
                    msg = f"Tool '{candidate}' bloqueada por política de solo lectura del rol '{member_name}'."
                    self.emit_activity_async(
                        team_id=team_id,
                        member=member_name,
                        workspace=workspace,
                        subagent_id=subagent_id,
                        kind="blocked",
                        tool=candidate,
                        summary=msg,
                        detail=arg_preview or None,
                        status="blocked",
                    )
                    return {"action": "block", "message": msg}

        self.emit_activity_async(
            team_id=team_id,
            member=member_name,
            workspace=workspace,
            subagent_id=subagent_id,
            kind="tool_start",
            tool=tool_name,
            summary=f"Ejecutando {tool_name}" + (f": {arg_preview}" if arg_preview else ""),
            detail=arg_preview or None,
            status="running",
            runtime_state="running",
        )
        return None

    def handle_post_tool_call(
        self,
        *,
        tool_name: str,
        args: Any,
        result: Any,
        session_id: str = "",
        task_id: str = "",
        duration_ms: int = 0,
        status: str | None = None,
        error_message: str | None = None,
    ) -> None:
        info = self.lookup_member_by_ids(session_id=session_id, task_id=task_id)
        if info is None:
            return
        team_id, member_name, workspace, subagent_id, _tfilter = info
        arg_preview = _summarize_tool_args(tool_name, args)
        st = (status or "ok").lower()
        is_err = st in {"error", "failed", "cancelled"} or bool(error_message)
        summary = f"{tool_name}" + (f" ({arg_preview})" if arg_preview else "")
        if is_err and error_message:
            detail = str(error_message)[:400]
        else:
            raw_res = result if isinstance(result, str) else str(result or "")
            detail = " ".join(raw_res.split())[:240] if raw_res else None
        self.emit_activity_async(
            team_id=team_id,
            member=member_name,
            workspace=workspace,
            subagent_id=subagent_id,
            kind="tool_end",
            tool=tool_name,
            summary=summary,
            detail=detail,
            status="error" if is_err else "ok",
            duration_ms=duration_ms,
            error=str(error_message)[:300] if is_err and error_message else None,
        )

    def remember_captain_agent(self, session_id: str, parent_agent: Any) -> None:
        if parent_agent is None:
            return
        with self._lock:
            if session_id:
                self._captain_agents[session_id] = parent_agent
                if len(self._captain_agents) > 16:
                    oldest = next(iter(self._captain_agents))
                    if oldest != session_id:
                        self._captain_agents.pop(oldest, None)
            self._last_captain_agent = parent_agent

    def get_captain_agent(self, session_id: str) -> Any:
        with self._lock:
            if session_id and session_id in self._captain_agents:
                return self._captain_agents[session_id]
            return self._last_captain_agent

    def _get_parent_agent(self, captain_session_id: str, workspace: str) -> Any:
        # Prefer the stored root Captain agent (depth == 0), never a delegated leaf child!
        with self._lock:
            if captain_session_id and captain_session_id in self._captain_agents:
                return self._captain_agents[captain_session_id]
            if self._last_captain_agent is not None:
                return self._last_captain_agent

        try:
            from agent.subagent_lifecycle import get_active_subagent_parent

            active = get_active_subagent_parent()
            if active is not None and int(getattr(active, "_delegate_depth", 0) or 0) == 0:
                self.remember_captain_agent(captain_session_id, active)
                return active
        except Exception:  # noqa: BLE001
            pass

        fallback = _build_fallback_parent_agent(captain_session_id, workspace)
        if fallback is not None:
            self.remember_captain_agent(captain_session_id, fallback)
        return fallback

    def resolve_member_context(self, subagent_id: str, fallback_workspace: str = "") -> tuple[str | None, str | None]:
        """Resolve `(member_name, workspace)` from a child's `_subagent_id`."""
        sid = (subagent_id or "").strip()
        if not sid:
            return (None, None)
        with self._lock:
            entry = self._member_lookup.get(sid)
            if entry is not None:
                return (entry[1], entry[2] or fallback_workspace or None)
        # Fallback: query live teams from hermes-teamsd (survives process restarts).
        try:
            qs = f"?workspace={fallback_workspace}" if fallback_workspace else ""
            state = self.client._request("GET", f"/api/v1/state{qs}")
            for team in state.get("teams") or []:
                for member in team.get("members") or []:
                    if str(member.get("id") or "") == sid:
                        name = str(member.get("name") or "")
                        if name:
                            with self._lock:
                                self._member_lookup[sid] = (
                                    str(team.get("id") or ""),
                                    name,
                                    fallback_workspace,
                                    None,
                                    str(team.get("captainSessionId") or ""),
                                )
                            return (name, fallback_workspace or None)
        except Exception:  # noqa: BLE001
            pass
        return (None, None)

    def start_poller(self) -> None:
        if self._poller is not None and self._poller.is_alive():
            return
        self._stop_event.clear()
        self._poller = threading.Thread(
            target=self._poll_loop,
            name="hermes-agent-teams-directive-poller",
            daemon=True,
        )
        self._poller.start()

    def stop(self) -> None:
        self._stop_event.set()

    def _poll_loop(self) -> None:
        while not self._stop_event.wait(0.5):
            try:
                queued = self.client.poll_directives()
                for item in queued:
                    ws = str(item.get("workspace") or "")
                    directive = item.get("directive")
                    if isinstance(directive, dict):
                        self.execute_directives([directive], workspace=ws)
            except Exception as exc:  # noqa: BLE001
                _log.debug("hermes-agent-teams directive poll tick error: %s", exc)

            # Reconcile tracked subagent handles every ~1.5s to detect completions & crashes.
            self._status_tick += 1
            if self._status_tick % 3 == 0:
                try:
                    self._reconcile_handles()
                except Exception as exc:  # noqa: BLE001
                    _log.debug("hermes-agent-teams handle reconcile tick error: %s", exc)

    def _reconcile_handles(self) -> None:
        lifecycle = getattr(self.ctx, "subagent_lifecycle", None)
        if lifecycle is None:
            return
        with self._lock:
            tracked = [
                (sid, handle, self._member_lookup.get(sid))
                for sid, handle in list(self._handles.items())
                if sid not in self._terminal_reported
            ]
        for sid, handle, meta in tracked:
            if not meta:
                continue
            team_id, member_name, workspace, _tfilter, captain_sid = meta
            parent = self._get_parent_agent(captain_sid, workspace)
            try:
                with _maybe_bind_parent(parent):
                    st = lifecycle.status(handle)
                state_name = str(getattr(st.state, "value", st.state) or "").upper()
                if state_name in {"SUCCEEDED", "FAILED", "CANCELLED", "INTERRUPTED"}:
                    with _maybe_bind_parent(parent):
                        res = lifecycle.result(handle)
                    with self._lock:
                        self._terminal_reported.add(sid)
                    summary = str(getattr(res, "summary", None) or "").strip()
                    err_msg = str(getattr(res, "error_message", None) or "").strip()
                    api_calls = (getattr(res, "usage_metadata", None) or {}).get("api_calls")
                    dur_s = (getattr(res, "tool_execution_summary", None) or {}).get("duration_seconds")
                    dur_ms = int(float(dur_s) * 1000) if dur_s else None

                    if state_name == "SUCCEEDED":
                        kind = "completed"
                        r_state = "succeeded"
                        msg = f"Turno finalizado ({api_calls or '?'} llamadas API)"
                    elif state_name == "FAILED":
                        kind = "failed"
                        r_state = "failed"
                        msg = f"Subagente falló: {err_msg or 'error desconocido'}"
                        self.client.bind_member(
                            workspace=workspace,
                            team_id=team_id,
                            member_name=member_name,
                            subagent_id=sid,
                            spawn_error=err_msg or "Subagent execution failed",
                        )
                    else:
                        kind = "cancelled"
                        r_state = state_name.lower()
                        msg = f"Subagente {state_name.lower()}"

                    self.client.report_activity(
                        team_id=team_id,
                        member=member_name,
                        workspace=workspace,
                        subagent_id=sid,
                        kind=kind,
                        summary=msg,
                        detail=summary[:500] if summary else (err_msg[:400] if err_msg else None),
                        status="error" if state_name == "FAILED" else "ok",
                        duration_ms=dur_ms,
                        runtime_state=r_state,
                        error=err_msg[:350] if err_msg else None,
                    )

                    if state_name == "SUCCEEDED":
                        with self._lock:
                            pending_steer = self._pending_steers.pop(sid, None)
                            is_latest = self._latest_subagent_for_member.get((team_id, member_name)) == sid
                        if is_latest:
                            self._maybe_recover_stalled_member(
                                team_id=team_id,
                                member_name=member_name,
                                workspace=workspace,
                                captain_sid=captain_sid,
                                pending_steer=pending_steer,
                            )
                    else:
                        with self._lock:
                            self._pending_steers.pop(sid, None)
            except Exception as exc:  # noqa: BLE001
                _log.debug("Reconcile check failed for %s: %s", sid, exc)

    def _maybe_recover_stalled_member(
        self,
        *,
        team_id: str,
        member_name: str,
        workspace: str,
        captain_sid: str,
        pending_steer: tuple[dict[str, Any], str] | None,
    ) -> None:
        """If a subagent exited SUCCEEDED while still owning a claimed/in_progress task, wake it cleanly once."""
        try:
            q = f"/api/v1/state?include_archived=0"
            if workspace:
                import urllib.parse

                q += f"&workspace={urllib.parse.quote(workspace, safe='')}"
            state = self.client._request("GET", q)
            teams = state.get("teams") if isinstance(state, dict) else None
            if not isinstance(teams, list):
                return
            team = next((t for t in teams if isinstance(t, dict) and t.get("id") == team_id), None)
            if not team or team.get("phase") != "running" or team.get("halted") or team.get("archived"):
                return
            open_task = next(
                (
                    tk
                    for tk in (team.get("tasks") or [])
                    if isinstance(tk, dict)
                    and tk.get("assignee") == member_name
                    and tk.get("status") in {"claimed", "in_progress"}
                ),
                None,
            )
            if not open_task:
                return
            tid = str(open_task.get("id") or "")
            if not tid:
                return
            guard_key = (team_id, member_name, tid)
            with self._lock:
                if guard_key in self._auto_recovered_tasks:
                    return
                self._auto_recovered_tasks.add(guard_key)

            member_obj = next(
                (m for m in (team.get("members") or []) if isinstance(m, dict) and m.get("name") == member_name),
                {},
            )
            if pending_steer is not None:
                steer_d, steer_ws = pending_steer
                goal_msg = str(steer_d.get("message") or "")
                role = str(steer_d.get("role") or member_obj.get("role") or "teammate")
                provider = steer_d.get("provider") or member_obj.get("provider")
                model = steer_d.get("model") or member_obj.get("model")
                effort = steer_d.get("reasoning_effort") or steer_d.get("reasoningEffort") or member_obj.get("reasoningEffort")
                exec_prompt = steer_d.get("execution_prompt") or steer_d.get("executionPrompt")
                tool_filter = steer_d.get("tool_filter") or steer_d.get("toolFilter")
                ws = steer_ws or workspace
            else:
                subj = str(open_task.get("subject") or tid)
                goal_msg = (
                    f"[Auto-Recovery] Your previous turn exited while task '{tid}' ({subj}) is still "
                    f"status='{open_task.get('status')}' assigned to you in team '{team_id}'. "
                    f"Call `agent_teams_status(teamId=\"{team_id}\")` to confirm your current `attemptId`, "
                    f"finish any remaining work for '{tid}', and report completion with `agent_teams_update_task`."
                )
                role = str(member_obj.get("role") or "teammate")
                provider = member_obj.get("provider")
                model = member_obj.get("model")
                effort = member_obj.get("reasoningEffort")
                exec_prompt = None
                tool_filter = None
                ws = workspace

            _log.warning(
                "Auto-recovering stalled member %s/%s on open task %s after prior turn exited SUCCEEDED",
                team_id,
                member_name,
                tid,
            )
            new_sid, err = self._launch_member_turn(
                team_id=team_id,
                captain_session_id=captain_sid,
                member_name=member_name,
                role=role,
                provider=provider,
                model=model,
                reasoning_effort=effort,
                execution_prompt=exec_prompt,
                tool_filter=tool_filter,
                goal_prompt=goal_msg,
                workspace=ws,
            )
            if new_sid or err:
                self.client.bind_member(
                    workspace=ws,
                    team_id=team_id,
                    member_name=member_name,
                    subagent_id=new_sid,
                    spawn_error=err,
                )
        except Exception as exc:  # noqa: BLE001
            _log.debug("Auto-recovery check failed for %s/%s: %s", team_id, member_name, exc)

    def execute_directives(self, directives: list[dict[str, Any]], *, workspace: str) -> None:
        for directive in directives:
            kind = directive.get("kind")
            try:
                if kind == "spawn_member":
                    self._handle_spawn(directive, workspace=workspace)
                elif kind == "wake_member":
                    self._handle_wake_member(directive, workspace=workspace)
                elif kind == "cancel_member":
                    self._handle_cancel_member(directive, workspace=workspace)
                elif kind == "wake_captain":
                    self._handle_wake_captain(directive, workspace=workspace)
            except Exception as exc:  # noqa: BLE001
                _log.warning("Failed executing directive %s: %s", kind, exc)

    def _launch_member_turn(
        self,
        *,
        team_id: str,
        captain_session_id: str,
        member_name: str,
        role: str,
        provider: str | None,
        model: str | None,
        reasoning_effort: str | None = None,
        execution_prompt: str | None,
        tool_filter: dict[str, Any] | None,
        goal_prompt: str,
        workspace: str,
    ) -> tuple[str, str | None]:
        context_parts = [
            MEMBER_INSTRUCTIONS,
            f"Team ID: {team_id}",
            f"Your member name: {member_name}",
            f"Role: {role}",
        ]
        if workspace:
            context_parts.append(f"Workspace directory: {workspace}")
        if execution_prompt:
            context_parts.append(f"Execution instructions:\n{execution_prompt}")
        full_context = "\n\n".join(context_parts)

        parent = self._get_parent_agent(captain_session_id, workspace)
        _ensure_parent_has_agent_teams(parent)
        subagent_id = ""
        spawn_error: str | None = None

        clean_effort = reasoning_effort.strip() if isinstance(reasoning_effort, str) and reasoning_effort.strip() else None

        try:
            lifecycle = getattr(self.ctx, "subagent_lifecycle", None)
            if lifecycle is not None:
                # Comply strictly with Hermes SubagentLaunchRequest validation:
                # - role must be 'leaf' or 'orchestrator'
                # - working_directory and blocked_tools must remain omitted/empty
                meta: dict[str, Any] = {
                    "hermes_agent_teams": True,
                    "team_id": team_id,
                    "member_name": member_name,
                    "team_role": role,
                    "workspace": workspace,
                }
                if clean_effort:
                    meta["reasoning_effort"] = clean_effort
                req: dict[str, Any] = {
                    "goal": goal_prompt,
                    "context": full_context,
                    "role": "leaf",
                    "metadata": meta,
                }
                if model:
                    req["model"] = model
                if provider:
                    req["provider"] = provider

                with self._launch_lock, _maybe_bind_parent(parent), _override_parent_reasoning(parent, clean_effort):
                    handle = lifecycle.launch(req)

                subagent_id = str(getattr(handle, "subagent_id", "") or "")
                if subagent_id:
                    with self._lock:
                        self._handles[subagent_id] = handle
                        self._terminal_reported.discard(subagent_id)
                        self._latest_subagent_for_member[(team_id, member_name)] = subagent_id
                        self._member_lookup[subagent_id] = (
                            team_id,
                            member_name,
                            workspace,
                            tool_filter if isinstance(tool_filter, dict) else None,
                            captain_session_id,
                        )
                    _log.info(
                        "Spawned Hermes subagent %s for team=%s member=%s (%s/%s effort=%s)",
                        subagent_id,
                        team_id,
                        member_name,
                        provider or "inherit",
                        model or "inherit",
                        clean_effort or "inherit",
                    )
            else:
                raw = self.ctx.dispatch_tool(
                    "delegate_task",
                    {
                        "goal": goal_prompt,
                        "context": full_context,
                        "role": role,
                        "background": True,
                    },
                )
                subagent_id = f"delegate-{team_id}-{member_name}-{int(time.time())}"
                _log.debug("delegate_task fallback result for %s: %s", member_name, str(raw)[:200])
        except Exception as exc:  # noqa: BLE001
            spawn_error = str(exc)
            _log.warning("Member spawn failed for %s/%s: %s", team_id, member_name, exc)

        return subagent_id, spawn_error

    def _handle_spawn(self, d: dict[str, Any], *, workspace: str) -> None:
        team_id = str(d.get("team_id") or d.get("teamId") or "")
        captain_sid = str(d.get("captain_session_id") or d.get("captainSessionId") or "")
        member_name = str(d.get("member_name") or d.get("memberName") or "")
        role = str(d.get("role") or "teammate")
        provider = d.get("provider")
        model = d.get("model")
        reasoning_effort = d.get("reasoning_effort") or d.get("reasoningEffort")
        execution_prompt = d.get("execution_prompt") or d.get("executionPrompt")
        initial_prompt = str(d.get("initial_prompt") or d.get("initialPrompt") or "")
        tool_filter = d.get("tool_filter") or d.get("toolFilter")

        subagent_id, spawn_error = self._launch_member_turn(
            team_id=team_id,
            captain_session_id=captain_sid,
            member_name=member_name,
            role=role,
            provider=provider,
            model=model,
            reasoning_effort=reasoning_effort,
            execution_prompt=execution_prompt,
            tool_filter=tool_filter,
            goal_prompt=initial_prompt,
            workspace=workspace,
        )

        bind_res = self.client.bind_member(
            workspace=workspace,
            team_id=team_id,
            member_name=member_name,
            subagent_id=subagent_id,
            spawn_error=spawn_error,
        )
        followups = bind_res.get("directives")
        if isinstance(followups, list) and followups:
            self.execute_directives(followups, workspace=workspace)

    def _handle_wake_member(self, d: dict[str, Any], *, workspace: str) -> None:
        team_id = str(d.get("team_id") or d.get("teamId") or "")
        captain_sid = str(d.get("captain_session_id") or d.get("captainSessionId") or "")
        member_name = str(d.get("member_name") or d.get("memberName") or "")
        subagent_id = str(d.get("subagent_id") or d.get("subagentId") or "")
        message = str(d.get("message") or "")
        if not message:
            return

        parent = self._get_parent_agent(captain_sid, workspace)
        lifecycle = getattr(self.ctx, "subagent_lifecycle", None)
        with self._lock:
            handle = self._handles.get(subagent_id)

        # A WakeMember carrying a new `task_id` + `attempt_id` is emitted right after the worker
        # completed its previous task while its old turn is still exiting. Never `steer()` a new
        # task assignment into a dying turn (Hermes drops the steer when the old turn finishes its
        # final reply); always launch a clean fresh turn with history=0 for the new task.
        is_new_task_assignment = bool(
            (d.get("task_id") or d.get("taskId"))
            and (d.get("attempt_id") or d.get("attemptId"))
        )

        # 1. For live P2P messages (not a new task assignment), steer an actively running child.
        if not is_new_task_assignment and lifecycle is not None and handle is not None:
            try:
                with _maybe_bind_parent(parent):
                    if lifecycle.steer(handle, message):
                        with self._lock:
                            self._pending_steers[subagent_id] = (dict(d), workspace)
                        return
            except Exception:  # noqa: BLE001
                pass

        # 2. Launch a fresh turn for this member (new task assignment or previously finished turn)
        #    on the same team/role/model/effort and re-bind its new subagent_id.
        new_sid, err = self._launch_member_turn(
            team_id=team_id,
            captain_session_id=captain_sid,
            member_name=member_name,
            role=str(d.get("role") or "teammate"),
            provider=d.get("provider"),
            model=d.get("model"),
            reasoning_effort=d.get("reasoning_effort") or d.get("reasoningEffort"),
            execution_prompt=d.get("execution_prompt") or d.get("executionPrompt"),
            tool_filter=d.get("tool_filter") or d.get("toolFilter"),
            goal_prompt=message,
            workspace=workspace,
        )
        if new_sid or err:
            self.client.bind_member(
                workspace=workspace,
                team_id=team_id,
                member_name=member_name,
                subagent_id=new_sid,
                spawn_error=err,
            )

    def _handle_cancel_member(self, d: dict[str, Any], *, workspace: str) -> None:
        captain_sid = str(d.get("captain_session_id") or d.get("captainSessionId") or "")
        subagent_id = str(d.get("subagent_id") or d.get("subagentId") or "")
        reason = str(d.get("reason") or "cancelled by team orchestrator")
        if not subagent_id:
            return

        parent = self._get_parent_agent(captain_sid, workspace)
        lifecycle = getattr(self.ctx, "subagent_lifecycle", None)
        with self._lock:
            handle = self._handles.get(subagent_id)

        if lifecycle is not None and handle is not None:
            try:
                with _maybe_bind_parent(parent):
                    lifecycle.cancel(handle, reason=reason)
                return
            except Exception:  # noqa: BLE001
                pass

        try:
            self.ctx.dispatch_tool(
                "delegate_task",
                {"action": "stop", "subagent_id": subagent_id},
            )
        except Exception as exc:  # noqa: BLE001
            _log.debug("Cancel fallback for %s: %s", subagent_id, exc)

    @staticmethod
    def _should_auto_wake_webui(message: str) -> bool:
        """Only auto-start a WebUI Captain turn for terminal/milestone events (avoid intermediate mail spam)."""
        low = message.lower()
        return (
            "have finished" in low
            or "revise staged team plan" in low
            or "escalat" in low
        )

    def _wake_captain_via_webui(self, captain_sid: str, message: str, workspace: str = "") -> bool:
        """Trigger a real Captain turn in Hermes WebUI when no gateway runner is active."""
        base = (os.environ.get("HERMES_WEBUI_URL") or "http://127.0.0.1:8787").rstrip("/")
        payload: dict[str, Any] = {
            "session_id": captain_sid,
            "message": message,
            "goal_continuation_id": "",
        }
        if workspace:
            payload["workspace"] = workspace
        req = urllib.request.Request(
            f"{base}/api/chat/start",
            data=json.dumps(payload).encode("utf-8"),
            headers={
                "Content-Type": "application/json",
                "Origin": base,
            },
            method="POST",
        )
        try:
            with urllib.request.urlopen(req, timeout=10) as resp:
                if 200 <= resp.status < 300:
                    _log.info("Auto-woke Captain session %s via WebUI /api/chat/start", captain_sid)
                    return True
        except Exception as exc:  # noqa: BLE001
            _log.debug("WebUI Captain auto-wake failed for %s: %s", captain_sid, exc)
        return False

    def _handle_wake_captain(self, d: dict[str, Any], *, workspace: str = "") -> None:
        message = str(d.get("message") or "").strip()
        if not message:
            return
        captain_sid = str(d.get("captain_session_id") or d.get("captainSessionId") or "").strip()
        delivered = False
        try:
            inject = getattr(self.ctx, "inject_message", None)
            if callable(inject):
                delivered = bool(
                    inject(message, session_key=captain_sid) if captain_sid else inject(message)
                )
            elif callable(getattr(self.ctx, "post_message", None)):
                self.ctx.post_message(message)
                delivered = True
        except Exception as exc:  # noqa: BLE001
            _log.debug("Captain wake notification deferred: %s", exc)

        if not delivered and captain_sid and self._should_auto_wake_webui(message):
            self._wake_captain_via_webui(captain_sid, message, workspace=workspace)
