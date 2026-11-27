"""Bridge between Rust hermes-teamsd EngineDirectives and native Hermes subagents."""

from __future__ import annotations

import logging
import threading
import time
from typing import Any

from .client import TeamsDaemonClient
from .prompts import MEMBER_INSTRUCTIONS

_log = logging.getLogger(__name__)


class SubagentBridge:
    """Executes Rust core directives against native Hermes `ctx.subagent_lifecycle` and `delegate_task`."""

    def __init__(self, ctx: Any, client: TeamsDaemonClient) -> None:
        self.ctx = ctx
        self.client = client
        self._handles: dict[str, Any] = {}  # subagent_id -> SubagentHandle
        self._member_lookup: dict[str, tuple[str, str]] = {}  # subagent_id -> (team_id, member_name)
        self._lock = threading.Lock()
        self._stop_event = threading.Event()
        self._poller: threading.Thread | None = None

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
        while not self._stop_event.wait(1.2):
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
                    self._handle_wake_member(directive)
                elif kind == "cancel_member":
                    self._handle_cancel_member(directive)
                elif kind == "wake_captain":
                    self._handle_wake_captain(directive)
            except Exception as exc:  # noqa: BLE001
                _log.warning("Failed executing directive %s: %s", kind, exc)

    def _handle_spawn(self, d: dict[str, Any], *, workspace: str) -> None:
        team_id = str(d.get("team_id") or d.get("teamId") or "")
        member_name = str(d.get("member_name") or d.get("memberName") or "")
        role = str(d.get("role") or "teammate")
        provider = d.get("provider")
        model = d.get("model")
        execution_prompt = d.get("execution_prompt") or d.get("executionPrompt")
        initial_prompt = str(d.get("initial_prompt") or d.get("initialPrompt") or "")
        tool_filter = d.get("tool_filter") or d.get("toolFilter") or {}
        blocked_tools = tuple(tool_filter.get("deny") or ())

        context_parts = [
            MEMBER_INSTRUCTIONS,
            f"Team ID: {team_id}",
            f"Your member name: {member_name}",
            f"Role: {role}",
        ]
        if execution_prompt:
            context_parts.append(f"Execution instructions:\n{execution_prompt}")
        full_context = "\n\n".join(context_parts)

        subagent_id = ""
        spawn_error: str | None = None

        try:
            lifecycle = getattr(self.ctx, "subagent_lifecycle", None)
            if lifecycle is not None:
                req: dict[str, Any] = {
                    "goal": initial_prompt,
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
                handle = lifecycle.launch(req)
                subagent_id = str(getattr(handle, "subagent_id", "") or "")
                if subagent_id:
                    with self._lock:
                        self._handles[subagent_id] = handle
                        self._member_lookup[subagent_id] = (team_id, member_name)
            else:
                # Fallback to native delegate_task dispatch
                raw = self.ctx.dispatch_tool(
                    "delegate_task",
                    {
                        "goal": initial_prompt,
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

    def _handle_wake_member(self, d: dict[str, Any]) -> None:
        subagent_id = str(d.get("subagent_id") or d.get("subagentId") or "")
        message = str(d.get("message") or "")
        if not subagent_id or not message:
            return

        lifecycle = getattr(self.ctx, "subagent_lifecycle", None)
        with self._lock:
            handle = self._handles.get(subagent_id)

        if lifecycle is not None and handle is not None:
            try:
                if lifecycle.steer(handle, message):
                    return
            except Exception:  # noqa: BLE001
                pass

        # Continuable follow-up turn through native Hermes delegate_task(action='continue')
        try:
            self.ctx.dispatch_tool(
                "delegate_task",
                {
                    "action": "continue",
                    "subagent_id": subagent_id,
                    "message": message,
                    "background": True,
                },
            )
        except Exception as exc:  # noqa: BLE001
            _log.warning("Could not wake member %s via delegate_task continue: %s", subagent_id, exc)

    def _handle_cancel_member(self, d: dict[str, Any]) -> None:
        subagent_id = str(d.get("subagent_id") or d.get("subagentId") or "")
        reason = str(d.get("reason") or "cancelled by team orchestrator")
        if not subagent_id:
            return

        lifecycle = getattr(self.ctx, "subagent_lifecycle", None)
        with self._lock:
            handle = self._handles.get(subagent_id)

        if lifecycle is not None and handle is not None:
            try:
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
