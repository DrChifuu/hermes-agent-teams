"""Exact OpenAI-compatible JSON schemas for all 14 DSH Agent Teams tools."""

from __future__ import annotations

TOOL_SCHEMAS: dict[str, dict] = {
    "agent_teams_create": {
        "name": "agent_teams_create",
        "description": "Create an Agent Team (staged plan by default for user review, or automatic if explicitly requested). Caller becomes Captain.",
        "parameters": {
            "type": "object",
            "properties": {
                "name": {"type": "string", "description": "Short team name (e.g. harbor-stockroom)."},
                "description": {"type": "string", "description": "Team goal and scope summary."},
                "profile": {"type": "string", "description": "Optional named multi-role profile id."},
                "approval": {
                    "type": "string",
                    "enum": ["manual", "automatic"],
                    "description": "Default 'manual' stages plan for Approve & Run review; 'automatic' starts immediately.",
                },
                "members": {
                    "type": "array",
                    "description": "Initial member roster.",
                    "items": {
                        "type": "object",
                        "properties": {
                            "name": {"type": "string"},
                            "role": {"type": "string"},
                            "provider": {"type": "string"},
                            "model": {"type": "string"},
                            "reasoningEffort": {"type": "string"},
                            "executionPrompt": {"type": "string"},
                            "toolPolicy": {
                                "type": "string",
                                "description": "Optional tool policy id. Built-ins: 'readonly' (blocks write_file/patch/edit/delete/skill_manage; auto-applied to reviewer/architect/auditor roles), 'strict-readonly' (also blocks terminal/execute_code), 'coding', 'full'. Custom policies can be declared in ~/.hermes/agent-teams.yaml.",
                            },
                        },
                        "required": ["name"],
                    },
                },
                "tasks": {
                    "type": "array",
                    "description": "Initial dependency-aware task DAG.",
                    "items": {
                        "type": "object",
                        "properties": {
                            "id": {"type": "string"},
                            "subject": {"type": "string"},
                            "description": {"type": "string"},
                            "assignee": {"type": "string"},
                            "dependencies": {"type": "array", "items": {"type": "string"}},
                            "kind": {
                                "type": "string",
                                "enum": ["work", "requirements", "implementation", "verification", "review", "repair", "integration"],
                            },
                            "objective": {"type": "string"},
                            "inScope": {"type": "array", "items": {"type": "string"}},
                            "outOfScope": {"type": "array", "items": {"type": "string"}},
                            "acceptance": {"type": "array", "items": {"type": "string"}},
                            "verify": {"type": "array", "items": {"type": "string"}},
                            "reviewedTaskId": {"type": "string"},
                            "coverageOf": {"type": "array", "items": {"type": "string"}},
                        },
                        "required": ["subject"],
                    },
                },
            },
            "required": ["name"],
        },
    },
    "agent_teams_approve": {
        "name": "agent_teams_approve",
        "description": "Approve a staged team plan, spawn all members, and begin scheduling ready tasks.",
        "parameters": {
            "type": "object",
            "properties": {
                "teamId": {"type": "string", "description": "Team id to approve."},
            },
            "required": ["teamId"],
        },
    },
    "agent_teams_edit_plan": {
        "name": "agent_teams_edit_plan",
        "description": "Atomically revise a staged team draft (description, members, and/or tasks) without creating a new team.",
        "parameters": {
            "type": "object",
            "properties": {
                "teamId": {"type": "string"},
                "description": {"type": "string"},
                "members": {"type": "array", "items": {"type": "object"}},
                "tasks": {"type": "array", "items": {"type": "object"}},
            },
            "required": ["teamId"],
        },
    },
    "agent_teams_add_member": {
        "name": "agent_teams_add_member",
        "description": "Add a durable member to the team (inherits Captain model/provider unless explicitly overridden).",
        "parameters": {
            "type": "object",
            "properties": {
                "teamId": {"type": "string"},
                "name": {"type": "string"},
                "role": {"type": "string"},
                "provider": {"type": "string"},
                "model": {"type": "string"},
                "reasoningEffort": {"type": "string"},
                "executionPrompt": {"type": "string"},
                "toolPolicy": {"type": "string"},
            },
            "required": ["teamId", "name"],
        },
    },
    "agent_teams_remove_member": {
        "name": "agent_teams_remove_member",
        "description": "Safely remove a member, revoke active attempts, reclaim unfinished tasks, and reschedule.",
        "parameters": {
            "type": "object",
            "properties": {
                "teamId": {"type": "string"},
                "name": {"type": "string"},
            },
            "required": ["teamId", "name"],
        },
    },
    "agent_teams_create_task": {
        "name": "agent_teams_create_task",
        "description": "Create a task with optional assignee, dependencies, and quality-gate contract fields.",
        "parameters": {
            "type": "object",
            "properties": {
                "teamId": {"type": "string"},
                "subject": {"type": "string"},
                "description": {"type": "string"},
                "assignee": {"type": "string"},
                "dependencies": {"type": "array", "items": {"type": "string"}},
                "kind": {
                    "type": "string",
                    "enum": ["work", "requirements", "implementation", "verification", "review", "repair", "integration"],
                },
                "objective": {"type": "string"},
                "inScope": {"type": "array", "items": {"type": "string"}},
                "outOfScope": {"type": "array", "items": {"type": "string"}},
                "acceptance": {"type": "array", "items": {"type": "string"}},
                "verify": {"type": "array", "items": {"type": "string"}},
                "reviewedTaskId": {"type": "string"},
                "coverageOf": {"type": "array", "items": {"type": "string"}},
                "resume": {"type": "boolean"},
                "resumeReason": {"type": "string"},
            },
            "required": ["teamId", "subject"],
        },
    },
    "agent_teams_reassign_task": {
        "name": "agent_teams_reassign_task",
        "description": "Atomically revoke the old attempt, interrupt the old worker, and reassign a task (use assignee='captain' for captain takeover).",
        "parameters": {
            "type": "object",
            "properties": {
                "teamId": {"type": "string"},
                "taskId": {"type": "string"},
                "assignee": {"type": "string"},
                "reason": {"type": "string"},
            },
            "required": ["teamId", "taskId", "assignee"],
        },
    },
    "agent_teams_claim_task": {
        "name": "agent_teams_claim_task",
        "description": "Claim a ready task whose dependencies are completed; issues a unique attemptId.",
        "parameters": {
            "type": "object",
            "properties": {
                "teamId": {"type": "string"},
                "taskId": {"type": "string"},
                "assignee": {"type": "string"},
            },
            "required": ["teamId", "taskId"],
        },
    },
    "agent_teams_update_task": {
        "name": "agent_teams_update_task",
        "description": "Advance or finish a claimed task (requires current attemptId for members; enforces quality-gate contracts).",
        "parameters": {
            "type": "object",
            "properties": {
                "teamId": {"type": "string"},
                "taskId": {"type": "string"},
                "attemptId": {"type": "string"},
                "status": {
                    "type": "string",
                    "enum": ["in_progress", "completed", "failed", "cancelled"],
                },
                "output": {"type": "string"},
                "verdict": {"type": "string", "enum": ["pass", "needs_revision", "reject"]},
                "findings": {"type": "array", "items": {"type": "object"}},
                "changedPaths": {"type": "array", "items": {"type": "string"}},
                "acceptanceResults": {"type": "array", "items": {"type": "object"}},
                "commandsRun": {"type": "array", "items": {"type": "object"}},
            },
            "required": ["teamId", "taskId", "status"],
        },
    },
    "agent_teams_amend_task": {
        "name": "agent_teams_amend_task",
        "description": "Captain-only: amend contract fields of a non-terminal quality task and append to the revisions ledger.",
        "parameters": {
            "type": "object",
            "properties": {
                "teamId": {"type": "string"},
                "taskId": {"type": "string"},
                "reason": {"type": "string"},
                "objective": {"type": "string"},
                "acceptance": {"type": "array", "items": {"type": "string"}},
                "verify": {"type": "array", "items": {"type": "string"}},
                "inScope": {"type": "array", "items": {"type": "string"}},
                "outOfScope": {"type": "array", "items": {"type": "string"}},
            },
            "required": ["teamId", "taskId", "reason"],
        },
    },
    "agent_teams_send_message": {
        "name": "agent_teams_send_message",
        "description": "Send a durable direct mailbox message to any teammate or 'captain' and wake them.",
        "parameters": {
            "type": "object",
            "properties": {
                "teamId": {"type": "string"},
                "to": {"type": "string", "description": "Recipient member name or 'captain'."},
                "content": {"type": "string"},
                "taskId": {"type": "string"},
                "attemptId": {"type": "string"},
            },
            "required": ["teamId", "to", "content"],
        },
    },
    "agent_teams_status": {
        "name": "agent_teams_status",
        "description": "Read full team overview: members, tasks, dependencies, waitingOn, unlocks, verdicts, and unread counts.",
        "parameters": {
            "type": "object",
            "properties": {
                "teamId": {"type": "string"},
            },
            "required": ["teamId"],
        },
    },
    "agent_teams_resume": {
        "name": "agent_teams_resume",
        "description": "Explicitly resume a halted team with a non-empty reason (does not recreate cancelled tasks).",
        "parameters": {
            "type": "object",
            "properties": {
                "teamId": {"type": "string"},
                "reason": {"type": "string"},
            },
            "required": ["teamId", "reason"],
        },
    },
    "agent_teams_delete": {
        "name": "agent_teams_delete",
        "description": "End the team, interrupt active members, and archive the full team history to .agent-teams/archive/.",
        "parameters": {
            "type": "object",
            "properties": {
                "teamId": {"type": "string"},
                "reason": {"type": "string"},
            },
            "required": ["teamId"],
        },
    },
}

MEMBER_TOOL_SET = {
    "agent_teams_claim_task",
    "agent_teams_update_task",
    "agent_teams_send_message",
    "agent_teams_status",
}
