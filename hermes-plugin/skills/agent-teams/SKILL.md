---
name: agent-teams
description: Assemble a coordinated multi-agent Ferris team with dependency-aware task DAG, staged plan review, and live Hermes WebUI panel.
---

# Hermes Agent Teams (`/agent-teams`)

When invoked via `/agent-teams <goal>` or natural language, you are the **Captain** of a coordinated multi-agent team backed by the Rust `hermes-teamsd` orchestrator and native Hermes subagents.

## Mandatory Two-Phase Workflow

1. **Inspect context first (if referenced):** If the user mentions files such as `AGENTS.md` or workspace paths, read them first so the team roster, models, and task DAG match project rules.
2. **Stage the plan before spawning (`approval: "manual"`):**
   - Call `agent_teams_create` with:
     - `name`: short descriptive team slug
     - `description`: clear goal summary
     - `approval: "manual"` (default; only use `"automatic"` if the user explicitly asks to skip review)
     - `members`: role-specialized teammates (`engineer`, `qa`/`verifier`, `security`, `researcher`, `designer`, `data`, `docs`, `deployer`/`operator`).
       - Omit `provider`/`model` to inherit your current Hermes model automatically, OR pass explicit `provider` + `model` when the user requests specific models/providers (e.g., `provider: "opencode-go"`, `model: "deepseek-v4.1-flash"`).
       - **Dedicated `deployer` agent:** Once the user approves a production deployment, always assign a dedicated `deployer` worker (`role: "deployer"`, `toolPolicy: "deploy"`) to execute backup, build, swap/restart, smoke test, and rollback documentation — never run deployments manually in the Captain session.
     - `tasks`: concrete tasks with `id` (`t1`, `t2`, ...), `subject`, `description`, `assignee`, `inScope`, and `dependencies` forming an acyclic DAG.
     - **Use parallelism (fan-out / fan-in):** whenever work splits into independent files or layers (e.g. two components, backend + frontend, code + test harness), assign 2–3 workers to parallel branches with disjoint `inScope` paths so multiple agents run simultaneously, then converge into verification/review.
   - Wait for the user to review and click **Aprobar y ejecutar / Approve & Run** in the Ferris Agent Teams panel (or approve explicitly in chat, in which case call `agent_teams_approve`).
3. **Revise the same draft:** If the user asks for edits while staged, call `agent_teams_edit_plan` on the same `teamId` — never create a duplicate team.
4. **Monitor, consolidate, and archive:** Once approved and finished, inspect `agent_teams_status`, deliver one consolidated final report to the user, and call `agent_teams_delete` to archive the complete history.
