# Architecture & Upgrade-Safety Design

## Why Three Clean Layers?

1. **`crates/hermes-teams-core` + `crates/hermes-teamsd` (Rust)**
   - Owns all durable state under `<workspace>/.agent-teams/<teamId>/`:
     - `team.json` (atomic temp-file + `sync_all` + `rename`)
     - `inbox/captain.jsonl` and `inbox/<member>.jsonl`
     - `archive/<teamId>/` on completion or discard
   - Serializes per-workspace mutations with a per-workspace lock (zero cross-workspace blocking, zero Python GIL contention).
   - Emits typed `EngineDirective` instructions (`spawn_member`, `wake_member`, `cancel_member`, `wake_captain`) instead of tightly coupling to any specific LLM SDK.
   - Publishes real-time SSE updates (`GET /api/v1/events`) so the WebUI reflects tool transitions instantly.

2. **`hermes-plugin/` (Thin Python Shim for `hermes-agent`)**
   - Uses ONLY public `PluginContext` APIs (`register_tool`, `register_command`, `register_system_prompt_section`, `register_hook("pre_llm_call")`, `subagent_lifecycle`, `dispatch_tool("delegate_task", ...)`, `inject_message`).
   - Never imports private internals of `hermes-agent` or edits files inside `/root/hermes/hermes-agent`.
   - Automatically inherits Hermes's configured models, custom providers, API keys, MCP servers, skills, and workspace boundaries.

3. **`webui-extension/` (Official `hermes-webui` Extension)**
   - Uses ONLY the documented extension contract in `hermes-webui/docs/EXTENSIONS.md`:
     - Static assets served from `/extensions/hermes-agent-teams/...`
     - Mount-relative paths (`extensions/...`, `api/...`) so subpath deployments like `/hermes/` behind Caddy work transparently
     - `window.hermesExt.register("hermes-agent-teams")` for settings, storage, and `turn:start` / `turn:complete` hooks
     - Loopback sidecar proxy at `/api/extensions/hermes-agent-teams/sidecar/*`
     - Live model discovery via `GET api/models`
   - Never modifies `server.py`, `api/routes.py`, `static/index.html`, or `static/panels.js`.
