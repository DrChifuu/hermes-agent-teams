"""Bridge between Rust hermes-teamsd EngineDirectives and native Hermes subagents."""

from __future__ import annotations

import contextlib
import logging
import threading
import time
from typing import Any

from .client import TeamsDaemonClient
from .prompts import MEMBER_INSTRUCTIONS

_log = logging.getLogger(__name__)


@contextlib.contextmanager
def _maybe_bind_parent(parent: Any):
    """Bind `parent` to Hermes's active subagent contextvar when executing outside a live tool turn."""
    if parent is None:
        yield
        return
    try:
        from agent.subagent_lifecycle import bind_subagent_parent, get_active_subagent_parent

        if get_active_subagent_parent() is not None:
            yield
            return
        with bind_subagent_parent(parent):
            yield
        return
    except Exception:  # noqa: BLE001
        pass
    yield


def _build_fallback_parent_agent(captain_session_id: str, workspace: str) -> Any | None:
    """Best-effort fallback parent AIAgent when WebUI Approve & Run runs after a server restart."""
    try:
        from run_agent import AIAgent

        agent = AIAgent(
            session_id=captain_session_id or "hermes-agent-teams-captain",
            quiet_mode=True,
        )
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
        self._member_lookup: dict[str, tuple[str, str]] = {}  # subagent_id -> (team_id, member_name)
        self._captain_agents: dict[str, Any] = {}  # captain_session_id -> AIAgent
        self._last_captain_agent: Any = None
        self._lock = threading.Lock()
        self._stop_event = threading.Event()
        self._poller: threading.Thread | None = None

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

    def _get_parent_agent(self, captain_session_id: str, workspace: str) -> Any:
        try:
            from agent.subagent_lifecycle import get_active_subagent_parent

            active = get_active_subagent_parent()
            if active is not None:
                return active
        except Exception:  # noqa: BLE001
            pass

        with self._lock:
            if captain_session_id and captain_session_id in self._captain_agents:
                return self._captain_agents[captain_session_id]
            if self._last_captain_agent is not None:
                return self._last_captain_agent

        fallback = _build_fallback_parent_agent(captain_session_id, workspace)
        if fallback is not None:
            self.remember_captain_agent(captain_session_id, fallback)
        return fallback

    def resolve_member_name(self, subagent_id: str, workspace: str = "") -> str | None:
        """Resolve exact team member name (`ejecutor`, `arquitecto-revisor`, ...) from a child's `_subagent_id`."""
        sid = (subagent_id or "").strip()
        if not sid:
            return None
        with self._lock:
            entry = self._member_lookup.get(sid)
            if entry is not None:
                return entry[1]
        # Fallback: query live teams from hermes-teamsd (survives process restarts).
        try:
            qs = f"?workspace={workspace}" if workspace else ""
            state = self.client._request("GET", f"/api/v1/state{qs}")
            for team in state.get("teams") or []:
                for member in team.get("members") or []:
                    if str(member.get("id") or "") == sid:
                        name = str(member.get("name") or "")
                        if name:
                            with self._lock:
                                self._member_lookup[sid] = (str(team.get("id") or ""), name)
                            return name
        except Exception:  # noqa: BLE001
            pass
        return None

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
        while not self._stop_event.wait(1.0):
            try:
                queued = self.client.poll_directives()
                for item in queued:
                    ws = str(item.get("workspace") or "")
                    directive = item.get("directive")
                    if isinstance(directive, dict):
                        self.execute_directives([directive], workspace=ws)
            except Exception as exc:  # noqa: BLE001
                _log.debug("hermes-agent-teams directive poll tick error: %s", exc)

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
                    self._handle_wake_captain(directive)
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
        execution_prompt: str | None,
        tool_filter: dict[str, Any] | None,
        goal_prompt: str,
        workspace: str,
    ) -> tuple[str, str | None]:
        blocked_tools = tuple((tool_filter or {}).get("deny") or ())
        context_parts = [
            MEMBER_INSTRUCTIONS,
            f"Team ID: {team_id}",
            f"Your member name: {member_name}",
            f"Role: {role}",
        ]
        if execution_prompt:
            context_parts.append(f"Execution instructions:\n{execution_prompt}")
        full_context = "\n\n".join(context_parts)

        parent = self._get_parent_agent(captain_session_id, workspace)
        subagent_id = ""
        spawn_error: str | None = None

        try:
            lifecycle = getattr(self.ctx, "subagent_lifecycle", None)
            if lifecycle is not None:
                req: dict[str, Any] = {
                    "goal": goal_prompt,
                    "context": full_context,
                    "role": role,
                    "working_directory": workspace or None,
                    "blocked_tools": blocked_tools,
                    "metadata": {
                        "hermes_agent_teams": True,
                        "team_id": team_id,
                        "member_name": member_name,
                    },
                }
                if model:
                    req["model"] = model
                if provider:
                    req["provider"] = provider

                with _maybe_bind_parent(parent):
                    handle = lifecycle.launch(req)

                subagent_id = str(getattr(handle, "subagent_id", "") or "")
                if subagent_id:
                    with self._lock:
                        self._handles[subagent_id] = handle
                        self._member_lookup[subagent_id] = (team_id, member_name)
                    _log.info(
                        "Spawned Hermes subagent %s for team=%s member=%s (%s/%s)",
                        subagent_id,
                        team_id,
                        member_name,
                        provider or "inherit",
                        model or "inherit",
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

        # 1. If the child is actively running right now, steer it live.
        if lifecycle is not None and handle is not None:
            try:
                with _maybe_bind_parent(parent):
                    if lifecycle.steer(handle, message):
                        return
            except Exception:  # noqa: BLE001
                pass

        # 2. If the previous turn finished cleanly, launch a fresh turn for this member
        #    on the same team/role/model and re-bind its new subagent_id.
        new_sid, err = self._launch_member_turn(
            team_id=team_id,
            captain_session_id=captain_sid,
            member_name=member_name,
            role=str(d.get("role") or "teammate"),
            provider=d.get("provider"),
            model=d.get("model"),
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

    def _handle_wake_captain(self, d: dict[str, Any]) -> None:
        message = str(d.get("message") or "").strip()
        if not message:
            return
        try:
            if callable(getattr(self.ctx, "inject_message", None)):
                self.ctx.inject_message(message)
            elif callable(getattr(self.ctx, "post_message", None)):
                self.ctx.post_message(message)
        except Exception as exc:  # noqa: BLE001
            _log.debug("Captain wake notification deferred: %s", exc)
