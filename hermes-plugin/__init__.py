"""Hermes Agent native plugin for DSH Agent Teams (backed by Rust hermes-teamsd)."""

from __future__ import annotations

import json
import logging
import os
from pathlib import Path
from typing import Any

from .client import TeamsDaemonClient
from .prompts import CAPTAIN_PROTOCOL_SECTION, MAX_PROMPT_CHARS, slash_activation_directive
from .schemas import MEMBER_TOOL_SET, TOOL_SCHEMAS
from .subagent_bridge import SubagentBridge

_log = logging.getLogger(__name__)
PLUGIN_NAME = "hermes-agent-teams"


def _resolve_workspace(ctx: Any) -> str:
    try:
        cwd = ctx.session_cwd()
        if isinstance(cwd, str) and cwd.strip():
            return cwd.strip()
    except Exception:  # noqa: BLE001
        pass
    return os.getcwd()


def _resolve_session_id(ctx: Any, kwargs: dict[str, Any]) -> str:
    for key in ("session_id", "sessionId"):
        val = kwargs.get(key)
        if isinstance(val, str) and val.strip():
            return val.strip()
    parent = kwargs.get("parent_agent")
    if parent is not None:
        for attr in ("session_id", "id"):
            val = getattr(parent, attr, None)
            if isinstance(val, str) and val.strip():
                return val.strip()
    try:
        origin = ctx.turn_origin()
        if isinstance(origin, str) and origin.strip():
            return origin.strip()
    except Exception:  # noqa: BLE001
        pass
    return "hermes-default-session"


def _resolve_captain_route(kwargs: dict[str, Any]) -> tuple[str | None, str | None, str | None]:
    parent = kwargs.get("parent_agent")
    if parent is None:
        return (None, None, None)
    provider = getattr(parent, "provider", None) or getattr(parent, "api_provider", None)
    model = getattr(parent, "model", None)
    effort = getattr(parent, "reasoning_effort", None)
    return (
        str(provider) if provider else None,
        str(model) if model else None,
        str(effort) if effort else None,
    )


def register(ctx: Any) -> None:
    """Entry point invoked automatically by Hermes Agent's plugin loader."""
    sidecar_url = str(
        ctx.get_config("sidecar_url", os.environ.get("HERMES_TEAMS_URL", "http://127.0.0.1:17890"))
        or "http://127.0.0.1:17890"
    )
    auto_poll = bool(ctx.get_config("auto_poll_directives", True))

    client = TeamsDaemonClient(base_url=sidecar_url)
    bridge = SubagentBridge(ctx, client)
    if auto_poll:
        bridge.start_poller()

    def make_handler(tool_name: str):
        def _handler(args: dict[str, Any], **kwargs: Any) -> str:
            is_child = False
            try:
                is_child = bool(ctx.in_delegated_child())
            except Exception:  # noqa: BLE001
                is_child = False

            if is_child and tool_name not in MEMBER_TOOL_SET:
                return json.dumps(
                    {
                        "ok": False,
                        "error": f"Tool '{tool_name}' is captain-only and cannot be invoked by a team member.",
                    }
                )

            payload = dict(args or {})
            workspace = _resolve_workspace(ctx)
            session_id = _resolve_session_id(ctx, kwargs)
            parent_agent = kwargs.get("parent_agent")

            if not is_child and parent_agent is not None:
                bridge.remember_captain_agent(session_id, parent_agent)

            resolved_member: str | None = None
            task_id_kw = str(kwargs.get("task_id") or "")
            child_sid = str(getattr(parent_agent, "_subagent_id", "") or "") if parent_agent is not None else ""

            # Deferred `tool_call` invocations pass `task_id` / `session_id` without `parent_agent`.
            # Resolve member identity and workspace from SubagentBridge across all available IDs.
            tracked = bridge.lookup_member_by_ids(
                subagent_id=child_sid,
                session_id=session_id,
                task_id=task_id_kw,
            )
            if tracked is not None:
                _tid, resolved_member, member_ws, _sub_id, _tfilter = tracked
                if member_ws:
                    workspace = member_ws
                is_child = True
            elif is_child and (child_sid or task_id_kw):
                resolved_member, member_ws = bridge.resolve_member_context(
                    child_sid or task_id_kw, fallback_workspace=workspace
                )
                if member_ws:
                    workspace = member_ws

            if is_child and tool_name not in MEMBER_TOOL_SET:
                return json.dumps(
                    {
                        "ok": False,
                        "error": f"Tool '{tool_name}' is captain-only and cannot be invoked by a team member.",
                    }
                )

            actor = str(
                payload.pop("_actor", None)
                or resolved_member
                or ("member" if is_child else "captain")
            )

            if tool_name == "agent_teams_create":
                payload.setdefault("captainSessionId", session_id)
                cap_provider, cap_model, cap_effort = _resolve_captain_route(kwargs)
                if cap_provider:
                    payload.setdefault("captainProvider", cap_provider)
                if cap_model:
                    payload.setdefault("captainModel", cap_model)
                if cap_effort:
                    payload.setdefault("captainReasoningEffort", cap_effort)
            elif tool_name in {"agent_teams_add_member", "agent_teams_edit_plan"}:
                cap_provider, cap_model, cap_effort = _resolve_captain_route(kwargs)
                if cap_provider:
                    payload.setdefault("captainProvider", cap_provider)
                if cap_model:
                    payload.setdefault("captainModel", cap_model)
                if cap_effort:
                    payload.setdefault("captainReasoningEffort", cap_effort)

            res = client.dispatch_tool(tool_name, payload, workspace=workspace, actor=actor)
            directives = res.pop("directives", None)
            # When the background poller is running, hermes-teamsd already enqueued all directives
            # for single-threaded root-Captain execution. Only execute inline when auto_poll is off
            # (e.g. unit tests or single-shot CLI runs).
            if not auto_poll and isinstance(directives, list) and directives:
                bridge.execute_directives(directives, workspace=workspace)
            return json.dumps(res, ensure_ascii=False)

        return _handler

    for name, schema in TOOL_SCHEMAS.items():
        ctx.register_tool(
            name=name,
            toolset="agent_teams",
            schema=schema,
            handler=make_handler(name),
            description=schema.get("description", ""),
            emoji="🦀",
        )

    # Register `agent-teams` as a native Hermes Skill so `/agent-teams <goal>`
    # routes directly to the LLM chat turn (unlike `register_command`, which
    # Hermes WebUI intercepts locally via /api/commands/exec without invoking the LLM).
    skill_md = Path(__file__).resolve().parent / "skills" / "agent-teams" / "SKILL.md"
    if skill_md.is_file() and callable(getattr(ctx, "register_skill", None)):
        try:
            ctx.register_skill(
                "agent-teams",
                skill_md,
                description="Assemble a coordinated multi-agent Ferris team with DAG tasks and live panel",
            )
        except Exception as exc:  # noqa: BLE001
            _log.debug("Could not register bundled agent-teams skill: %s", exc)

    # Register `/teams-status` as an instant zero-token diagnostic slash command.
    def _slash_teams_status(_raw_args: str = "", **_kwargs: Any) -> str:
        health = client.health()
        if not health.get("ok"):
            return f"🦀 hermes-teamsd offline: {health.get('error', 'unreachable')}"
        ws = _resolve_workspace(ctx)
        res = client._request("GET", f"/api/v1/state?workspace={ws}")
        teams = res.get("teams") or []
        archived = res.get("archived") or []
        if not teams:
            return (
                f"🦀 **Hermes Agent Teams** online (v{health.get('version')}, uptime {health.get('uptimeSecs', 0)}s)\n"
                f"No active teams in `{ws}` ({len(archived)} archived).\n"
                "Start one with `/agent-teams <goal>`."
            )
        lines = [f"🦀 **Hermes Agent Teams** — {len(teams)} active team(s):"]
        for t in teams:
            st = t.get("stats") or {}
            lines.append(
                f"- **{t.get('name')}** (`{t.get('id')}`) · phase=`{t.get('phase')}` · "
                f"{st.get('memberCount', 0)} members · {st.get('completed', 0)}/{st.get('total', 0)} done"
            )
        return "\n".join(lines)

    ctx.register_command(
        "teams-status",
        _slash_teams_status,
        description="Show live Hermes Agent Teams daemon and active team status (zero tokens)",
    )

    # Inject concise fixed Captain protocol section without rewriting prompts per turn.
    if callable(getattr(ctx, "register_system_prompt_section", None)):
        ctx.register_system_prompt_section(
            PLUGIN_NAME,
            CAPTAIN_PROTOCOL_SECTION,
            max_chars=MAX_PROMPT_CHARS,
        )

    # Gesture boundary: plain-text `/agent-teams <goal>` on surfaces without command adjudication.
    def _pre_llm_call(event: dict[str, Any] | None = None, **_kwargs: Any) -> dict[str, Any] | None:
        if not isinstance(event, dict):
            return None
        user_text = str(event.get("user_message") or event.get("last_user_message") or "").strip()
        if user_text.startswith("/agent-teams "):
            goal = user_text[len("/agent-teams ") :].strip()
            if goal:
                return {"context": slash_activation_directive(goal)}
        return None

    def _on_subagent_start(**kwargs: Any) -> None:
        bridge.register_child_session(
            str(kwargs.get("child_session_id") or ""),
            str(kwargs.get("child_subagent_id") or ""),
        )

    def _on_pre_tool_call(**kwargs: Any) -> dict[str, Any] | None:
        return bridge.handle_pre_tool_call(
            tool_name=str(kwargs.get("tool_name") or ""),
            args=kwargs.get("args"),
            session_id=str(kwargs.get("session_id") or ""),
            task_id=str(kwargs.get("task_id") or ""),
        )

    def _on_post_tool_call(**kwargs: Any) -> None:
        bridge.handle_post_tool_call(
            tool_name=str(kwargs.get("tool_name") or ""),
            args=kwargs.get("args"),
            result=kwargs.get("result"),
            session_id=str(kwargs.get("session_id") or ""),
            task_id=str(kwargs.get("task_id") or ""),
            duration_ms=int(kwargs.get("duration_ms") or 0),
            status=kwargs.get("status"),
            error_message=kwargs.get("error_message"),
        )

    if callable(getattr(ctx, "register_hook", None)):
        ctx.register_hook("pre_llm_call", _pre_llm_call)
        ctx.register_hook("subagent_start", _on_subagent_start)
        ctx.register_hook("pre_tool_call", _on_pre_tool_call)
        ctx.register_hook("post_tool_call", _on_post_tool_call)

    if callable(getattr(ctx, "on_unload", None)):
        ctx.on_unload(bridge.stop)

    _log.info("Registered %s plugin (%d tools, sidecar=%s)", PLUGIN_NAME, len(TOOL_SCHEMAS), sidecar_url)
