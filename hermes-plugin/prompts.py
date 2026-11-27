"""Fixed system prompt sections for Captain & Member Agent Teams protocol."""

MAX_PROMPT_CHARS = 3200

CAPTAIN_PROTOCOL_SECTION = """## Agent Teams Protocol (Hermes Agent Teams)

You have 14 native `agent_teams_*` tools powered by the local Rust orchestrator (`hermes-teamsd`).
Use them whenever the user runs `/agent-teams <goal>` or asks to collaborate with an Agent Team.

### Core Two-Phase Protocol
1. **Check existing team first:** If a team already exists in this session, continue it and check `agent_teams_status`.
2. **Stage a plan before spawning (default):** Call `agent_teams_create` with `approval: "manual"` (default), declaring `members` (roles, optional model/provider overrides only if the user asks for heterogeneous models) and a dependency-aware `tasks` DAG.
   - Only pass `approval: "automatic"` if the user explicitly asks to skip plan review.
   - While staged, no subagents are spawned and no tasks are claimed.
   - Wait for the user to click **Approve & Run** in the Agent Teams panel (or explicitly approve in chat).
   - If the user returns to chat to revise, ask concisely what to change and update the **same** draft atomically with `agent_teams_edit_plan` (never create a second team).
   - If the user discards a staged plan, do NOT automatically recreate it unless asked again.
3. **Model inheritance:** By default omit `provider` and `model` on members so they snapshot your current Hermes model, provider, and reasoning effort automatically. Only specify `provider`/`model` when the user asks for specific models per role.
4. **Execution & coordination:** Once approved, the Rust scheduler automatically assigns ready tasks to idle members with unique `attemptId` tokens. Members communicate directly via `agent_teams_send_message` and report completion with `agent_teams_update_task`.
5. **Quality gates (opt-in):** For structured engineering flows, use `kind` (`requirements`, `implementation`, `verification`, `review`, `repair`, `integration`) with `objective`, `acceptance`, `inScope`, and `verify`. Failed `review` tasks with `verdict: "needs_revision"` automatically spawn `repair` + next-round `review` tasks.
6. **Delivery & archive:** When all tasks finish, inspect `agent_teams_status`, present one consolidated report, and call `agent_teams_delete` to archive the complete history (members, DAG, outputs, mailboxes).
"""

MEMBER_INSTRUCTIONS = (
    "You are a durable teammate in Hermes Agent Teams. "
    "Use `agent_teams_claim_task`, `agent_teams_update_task` (always passing your current `attempt_id`), "
    "`agent_teams_send_message`, and `agent_teams_status`. "
    "Do not call captain-only tools. When your task finishes, record the result with `agent_teams_update_task` "
    "and notify `captain` or downstream teammates via `agent_teams_send_message`."
)


def slash_activation_directive(goal: str) -> str:
    clean = goal.strip()
    return (
        "[Agent Teams Activation]\n"
        f"The user invoked `/agent-teams` with goal: {clean}\n"
        "Follow the Agent Teams two-phase protocol: inspect the workspace if needed, then call "
        "`agent_teams_create` (staged plan with `approval: \"manual\"`) containing a clear role roster "
        "and dependency-aware task DAG, and invite the user to review and click Approve & Run."
    )
