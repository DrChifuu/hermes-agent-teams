"""Zero-dependency loopback HTTP client to the Rust hermes-teamsd daemon."""

from __future__ import annotations

import json
import os
import urllib.error
import urllib.parse
import urllib.request
from pathlib import Path
from typing import Any

EXTENSION_ID = "hermes-agent-teams"


def resolve_sidecar_token() -> str | None:
    """Resolve Hermes WebUI token-v1 secret in the exact documented precedence order."""
    candidates: list[Path] = []
    if explicit := os.environ.get("HERMES_EXT_SIDECAR_TOKEN_FILE", "").strip():
        candidates.append(Path(explicit))
    if state_dir := os.environ.get("HERMES_WEBUI_STATE_DIR", "").strip():
        candidates.append(Path(state_dir) / "sidecar-auth" / f"{EXTENSION_ID}.token")
    if hermes_home := os.environ.get("HERMES_HOME", "").strip():
        candidates.append(Path(hermes_home) / "webui" / "sidecar-auth" / f"{EXTENSION_ID}.token")
    candidates.append(Path.home() / ".hermes" / "webui" / "sidecar-auth" / f"{EXTENSION_ID}.token")

    for path in candidates:
        try:
            if path.is_file():
                val = path.read_text(encoding="utf-8").strip()
                if val:
                    return val
        except OSError:
            continue
    return None


class TeamsDaemonClient:
    """Thin JSON client talking to local loopback `hermes-teamsd`."""

    def __init__(self, base_url: str = "http://127.0.0.1:17890", timeout: float = 10.0) -> None:
        self.base_url = base_url.rstrip("/")
        self.timeout = timeout

    def _headers(self) -> dict[str, str]:
        headers = {"Content-Type": "application/json", "Accept": "application/json"}
        if token := resolve_sidecar_token():
            headers["X-Hermes-Sidecar-Token"] = token
        return headers

    def _request(self, method: str, path: str, body: dict[str, Any] | None = None) -> dict[str, Any]:
        url = f"{self.base_url}{path}"
        data = json.dumps(body).encode("utf-8") if body is not None else None
        req = urllib.request.Request(url, data=data, headers=self._headers(), method=method)
        try:
            with urllib.request.urlopen(req, timeout=self.timeout) as resp:
                raw = resp.read().decode("utf-8")
                return json.loads(raw) if raw.strip() else {"ok": True}
        except urllib.error.HTTPError as exc:
            raw = exc.read().decode("utf-8", errors="replace")
            try:
                parsed = json.loads(raw)
                return {"ok": False, "status": exc.code, "error": parsed.get("error", raw)}
            except json.JSONDecodeError:
                return {"ok": False, "status": exc.code, "error": raw or str(exc)}
        except Exception as exc:  # noqa: BLE001
            return {"ok": False, "error": f"hermes-teamsd unreachable at {self.base_url}: {exc}"}

    def health(self) -> dict[str, Any]:
        return self._request("GET", "/health")

    def dispatch_tool(
        self,
        tool_name: str,
        args: dict[str, Any],
        *,
        workspace: str,
        actor: str = "captain",
    ) -> dict[str, Any]:
        return self._request(
            "POST",
            f"/api/v1/tools/{urllib.parse.quote(tool_name, safe='')}",
            {"workspace": workspace, "actor": actor, "args": args},
        )

    def bind_member(
        self,
        *,
        workspace: str,
        team_id: str,
        member_name: str,
        subagent_id: str,
        spawn_error: str | None = None,
    ) -> dict[str, Any]:
        payload: dict[str, Any] = {
            "workspace": workspace,
            "teamId": team_id,
            "memberName": member_name,
            "subagentId": subagent_id,
        }
        if spawn_error:
            payload["spawnError"] = spawn_error
        return self._request("POST", "/api/v1/members/bind", payload)

    def poll_directives(self, workspace: str | None = None) -> list[dict[str, Any]]:
        qs = f"?workspace={urllib.parse.quote(workspace)}" if workspace else ""
        res = self._request("GET", f"/api/v1/directives/poll{qs}")
        items = res.get("directives")
        return items if isinstance(items, list) else []
