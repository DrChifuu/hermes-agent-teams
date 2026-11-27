# 🦀 Hermes Agent Teams (Ferris Edition)

A clean, high-efficiency port of [`NanmiCoder/dsh-agent-teams`](https://github.com/NanmiCoder/dsh-agent-teams) natively engineered for **[Hermes Agent](https://github.com/NousResearch/hermes-agent)** and **[Hermes WebUI](https://github.com/nesquena/hermes-webui)** — featuring a **Rust core + Axum SSE sidecar (`hermes-teamsd`)**, a thin native **Hermes Python plugin shim**, an official **zero-patch Hermes WebUI extension**, and **16 custom transparent Ferris the Rust crab worker mascots**.

> **Zero-patch upstream compatibility:** Neither `hermes-agent` nor `hermes-webui` source files are modified. You can `git pull` / upgrade Hermes and Hermes WebUI anytime without losing Agent Teams.

---

## Architecture

```text
┌─────────────────────────────────────────────────────────────────────────┐
│  Hermes WebUI Browser (Light / Dark / Skins · Desktop & Mobile)         │
│  Official Extension (~/.hermes/webui/extensions/hermes-agent-teams/)    │
│  · Ferris Worker Mascots (16 transparent PNGs)                          │
│  · Interactive SVG Task Dependency DAG + Segmented Progress             │
│  · Staged Plan Editor connected directly to Hermes /api/models          │
│  · Real-time SSE updates via /api/extensions/hermes-agent-teams/sidecar │
└───────────────────────────────┬─────────────────────────────────────────┘
                                │ Same-origin sidecar proxy (loopback)
┌───────────────────────────────▼─────────────────────────────────────────┐
│  hermes-teamsd (Rust · Axum + Tokio · ~10–18 MB RSS · <30 ms startup)   │
│  crates/hermes-teams-core + crates/hermes-teamsd                        │
│  · All 14 DSH Agent Teams operations & atomic disk persistence          │
│  · Topological DAG validator & cycle detector                           │
│  · Attempt-capability scheduler (attempt_id, revocation, cold recovery) │
│  · Durable P2P JSONL mailboxes (.agent-teams/<id>/inbox/*.jsonl)        │
│  · Quality Gates (contracts, SHA-256 artifact audit, auto-repair loops) │
│  · Owner-declared toolPolicies & YAML profiles + full archive history   │
└───────────────────────────────▲─────────────────────────────────────────┘
                                │ Loopback JSON API + directive queue
┌───────────────────────────────┴─────────────────────────────────────────┐
│  Native Hermes Agent Plugin (~/.hermes/plugins/hermes-agent-teams/)     │
│  · Registers all 14 `agent_teams_*` tools + `/agent-teams` command      │
│  · Fixed Captain protocol section + `pre_llm_call` gesture boundary     │
│  · Bridges directives to native `ctx.subagent_lifecycle` &              │
│    `delegate_task(action="continue")` — inheriting ALL Hermes models,   │
│    providers, API keys, skills, MCP servers, and sessions!              │
└─────────────────────────────────────────────────────────────────────────┘
```

---

## Performance on ARM64 SBCs (Orange Pi 4 Pro · 5.7 GiB RAM)

| Metric | Standalone Python Server (`/opt/agent-teams`) | `hermes-teamsd` (Rust) |
| --- | ---: | ---: |
| Resident Memory (RSS) | ~487 MB | **~10 – 18 MB** |
| Cold Start | ~2.0 s | **< 30 ms** |
| State / DAG / Lock Latency | ~5 – 20 ms | **< 0.5 ms** |
| UI Live Updates | 1 s HTTP polling | **Event-driven SSE stream** |
| Upstream Hermes/WebUI Patches | Required | **0 patches (official plugin + extension)** |

---

## Feature Parity with `dsh-agent-teams`

- **All 14 native tools**: `agent_teams_create`, `agent_teams_approve`, `agent_teams_edit_plan`, `agent_teams_add_member`, `agent_teams_remove_member`, `agent_teams_create_task`, `agent_teams_reassign_task`, `agent_teams_claim_task`, `agent_teams_update_task`, `agent_teams_amend_task`, `agent_teams_send_message`, `agent_teams_status`, `agent_teams_resume`, `agent_teams_delete`.
- **`/agent-teams <goal>`** slash command + natural-language activation.
- **Plan-Before-Execution (`staged`)**: review & edit roles, Hermes provider/model routes (`/api/models`), and DAG dependencies before spawning any subagent — with **Approve & Run**, **Return to chat**, and **Discard** (2-step confirmation).
- **Interactive SVG Task DAG**: left-to-right topological layout with Bézier curves, hover/click upstream-downstream lineage highlight, and node inspector card showing assignee, model, unlocks, and output.
- **16 Ferris Worker Mascots**: Captain Ferris (`team-lead.png`), 8 role Ferris (`engineer`, `qa`, `security`, `data`, `researcher`, `designer`, `docs`, `operator`, plus `default`), and 6 state action Ferris (`working`, `sleeping`, `thinking`, `reporting`, `sending`, `celebrating`).
- **Quality Gates & Tool Policies**: structured contracts (`requirements`, `implementation`, `verification`, `review`, `repair`, `integration`), automatic repair + re-review loops, scope boundary checks, SHA-256 artifact digests, and fail-closed `allow`/`deny` member tool policies.
- **Full Archive History**: `agent_teams_delete` archives everything to `<workspace>/.agent-teams/archive/<teamId>/` so completed teams remain inspectable in the Archive tab.

---

## Quick Install (Linux / Orange Pi 4 Pro)

```bash
git clone https://github.com/DrChifuu/hermes-agent-teams.git
cd hermes-agent-teams
chmod +x deploy/install.sh deploy/uninstall.sh
./deploy/install.sh
```

The installer:
1. Builds `target/release/hermes-teamsd` (defaults to `CARGO_BUILD_JOBS=3` to stay gentle on Orange Pi RAM) and installs it to `/usr/local/bin/hermes-teamsd`.
2. Copies `hermes-plugin/` to `~/.hermes/plugins/hermes-agent-teams/`.
3. Copies `webui-extension/` (including the 16 transparent Ferris PNGs) to `~/.hermes/webui/extensions/hermes-agent-teams/` and registers loopback sidecar consent.
4. Installs and enables `hermes-teamsd.service` (hardened loopback-only systemd unit capped at `MemoryMax=128M`).

### Authentication Modes
- **Behind Caddy `forward_auth` / external reverse-proxy auth** (default): uses `HERMES_TEAMS_AUTH_MODE=legacy` on `127.0.0.1:17890`.
- **With `HERMES_WEBUI_PASSWORD` enabled**: set `"proxy_auth": "token-v1"` in `webui-extension/manifest.json` and `HERMES_TEAMS_AUTH_MODE=token-v1` in `hermes-teamsd.service`; `hermes-teamsd` and `hermes-plugin` both validate `X-Hermes-Sidecar-Token` automatically in constant time.

---

## Development & Verification

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
```

## License

MIT — inspired by [`NanmiCoder/dsh-agent-teams`](https://github.com/NanmiCoder/dsh-agent-teams) and built for [`nesquena/hermes-webui`](https://github.com/nesquena/hermes-webui) & [`NousResearch/hermes-agent`](https://github.com/NousResearch/hermes-agent).
