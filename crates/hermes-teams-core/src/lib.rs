//! Core domain engine for Hermes Agent Teams (1:1 port of DSH Agent Teams).
//!
//! Provides durable team state persistence, dependency-aware DAG scheduling,
//! attempt capability verification, P2P JSONL mailboxes, quality gates,
//! owner-declared tool policies, and live UI snapshot projections.

pub mod dag;
pub mod error;
pub mod mailbox;
pub mod operations;
pub mod policies;
pub mod quality_gates;
pub mod scheduler;
pub mod snapshot;
pub mod store;
pub mod types;

pub use error::{Result, TeamsError};
pub use operations::TeamEngine;
pub use policies::TeamsConfig;
pub use snapshot::{collect_workspace_activity, TeamSnapshot, WorkspaceActivityPayload};
pub use store::TeamStore;
pub use types::{
    EngineDirective, MemberStatus, PlanReviewState, TaskKind, TaskStatus, TeamMember, TeamMessage,
    TeamPhase, TeamState, TeamTask, CAPTAIN_TOOL_NAMES, MEMBER_TOOL_NAMES, TEAM_TOOL_NAMES,
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::operations::{
        AddMemberInput, AmendTaskInput, CreateTaskInput, CreateTeamInput, EditPlanInput,
        PlanMemberInput, PlanTaskInput, ReassignTaskInput, SendMessageInput, UpdateTaskInput,
    };
    use crate::policies::RawToolPolicy;
    use crate::scheduler::recover_stranded_tasks;
    use crate::types::{
        CheckOutcome, CommandResult, FindingSeverity, ReviewFinding, ReviewVerdict,
    };
    use tempfile::TempDir;

    fn make_engine() -> (TempDir, TeamEngine) {
        let dir = TempDir::new().expect("tempdir");
        let store = TeamStore::for_workspace(dir.path(), None);
        let mut cfg = TeamsConfig::default();
        cfg.tool_policies.insert(
            "researcher".to_string(),
            RawToolPolicy {
                allow: Some(vec!["read_file".to_string(), "grep".to_string()]),
                deny: None,
            },
        );
        cfg.tool_policies.insert(
            "implementer".to_string(),
            RawToolPolicy {
                allow: None,
                deny: Some(vec!["web_search".to_string()]),
            },
        );
        (dir, TeamEngine::new(store, cfg))
    }

    #[test]
    fn staged_plan_approve_schedule_dag_and_archive() {
        let (_dir, engine) = make_engine();

        // 1. Create staged team matching the harbor-stockroom screenshot scenario
        let created = engine
            .create_team(CreateTeamInput {
                name: "harbor-stockroom".to_string(),
                description: Some("Build inventory and receipt modules".to_string()),
                captain_session_id: "sess-cap-1".to_string(),
                profile: None,
                approval: Some("manual".to_string()),
                members: vec![
                    PlanMemberInput {
                        name: "inventory".to_string(),
                        role: Some("engineer".to_string()),
                        provider: Some("deepseek".to_string()),
                        model: Some("deepseek-flash".to_string()),
                        reasoning_effort: None,
                        execution_prompt: None,
                        tool_policy: Some("implementer".to_string()),
                    },
                    PlanMemberInput {
                        name: "receipt".to_string(),
                        role: Some("engineer".to_string()),
                        provider: Some("deepseek".to_string()),
                        model: Some("deepseek-flash".to_string()),
                        reasoning_effort: None,
                        execution_prompt: None,
                        tool_policy: None,
                    },
                    PlanMemberInput {
                        name: "verifier".to_string(),
                        role: Some("verifier".to_string()),
                        provider: Some("deepseek".to_string()),
                        model: Some("deepseek-flash".to_string()),
                        reasoning_effort: None,
                        execution_prompt: None,
                        tool_policy: Some("researcher".to_string()),
                    },
                ],
                tasks: vec![
                    PlanTaskInput {
                        id: Some("t1".to_string()),
                        subject: "Implement src/inventory.js".to_string(),
                        description: None,
                        assignee: Some("inventory".to_string()),
                        dependencies: vec![],
                        kind: None,
                        objective: None,
                        in_scope: None,
                        out_of_scope: None,
                        acceptance: None,
                        verify: None,
                        reviewed_task_id: None,
                        coverage_of: None,
                    },
                    PlanTaskInput {
                        id: Some("t2".to_string()),
                        subject: "Implement src/receipt.js".to_string(),
                        description: None,
                        assignee: Some("receipt".to_string()),
                        dependencies: vec![],
                        kind: None,
                        objective: None,
                        in_scope: None,
                        out_of_scope: None,
                        acceptance: None,
                        verify: None,
                        reviewed_task_id: None,
                        coverage_of: None,
                    },
                    PlanTaskInput {
                        id: Some("t3".to_string()),
                        subject: "Write tests & run npm test".to_string(),
                        description: None,
                        assignee: Some("verifier".to_string()),
                        dependencies: vec!["t1".to_string(), "t2".to_string()],
                        kind: None,
                        objective: None,
                        in_scope: None,
                        out_of_scope: None,
                        acceptance: None,
                        verify: None,
                        reviewed_task_id: None,
                        coverage_of: None,
                    },
                ],
                review_policy: None,
                captain_provider: Some("deepseek".to_string()),
                captain_model: Some("deepseek-v4".to_string()),
                captain_reasoning_effort: Some("high".to_string()),
            })
            .expect("create staged");

        assert!(created.team.is_staged());
        assert_eq!(created.directives, Vec::<EngineDirective>::new());

        // Cannot claim while staged
        let err = engine.claim_task_op("harbor-stockroom", "t1", "inventory");
        assert!(matches!(err, Err(TeamsError::TeamStaged(_))));

        // 2. Approve & Run lazily spawns the 2 members with ready tasks (t1, t2) + 1 WakeCaptain
        let approved = engine.approve_team("harbor-stockroom").expect("approve");
        assert!(!approved.team.is_staged());
        assert_eq!(approved.directives.len(), 3);
        assert_eq!(
            approved.team.find_task("t1").unwrap().status,
            TaskStatus::Claimed
        );
        assert_eq!(
            approved.team.find_task("t2").unwrap().status,
            TaskStatus::Claimed
        );
        assert_eq!(
            approved.team.find_task("t3").unwrap().status,
            TaskStatus::Pending
        );

        // 3. Bind spawned Hermes subagents -> t1 & t2 stay claimed with attempt_ids; t3 still waits on deps
        let b1 = engine
            .bind_member_session("harbor-stockroom", "inventory", "sub-inv", None)
            .expect("bind inv");
        assert_eq!(b1.team.find_task("t1").unwrap().status, TaskStatus::Claimed);
        let t1_aid = b1.team.find_task("t1").unwrap().attempt_id.clone().unwrap();

        let b2 = engine
            .bind_member_session("harbor-stockroom", "receipt", "sub-rec", None)
            .expect("bind rec");
        assert_eq!(b2.team.find_task("t2").unwrap().status, TaskStatus::Claimed);
        let t2_aid = b2.team.find_task("t2").unwrap().attempt_id.clone().unwrap();

        let b3 = engine
            .bind_member_session("harbor-stockroom", "verifier", "sub-ver", None)
            .expect("bind ver");
        assert_eq!(b3.team.find_task("t3").unwrap().status, TaskStatus::Pending);

        // 4. Stale attempt_id is rejected
        let stale = engine.update_task(
            UpdateTaskInput {
                team_id: "harbor-stockroom".to_string(),
                task_id: "t1".to_string(),
                actor: "inventory".to_string(),
                attempt_id: Some("wrong-attempt".to_string()),
                status: TaskStatus::Completed,
                output: Some("bad".to_string()),
                verdict: None,
                findings: None,
                changed_paths: None,
                acceptance_results: None,
                commands_run: None,
            },
            None,
        );
        assert!(matches!(stale, Err(TeamsError::StaleAttempt { .. })));

        // 5. Complete t1 & t2 -> automatically schedules t3 on verifier!
        engine
            .update_task(
                UpdateTaskInput {
                    team_id: "harbor-stockroom".to_string(),
                    task_id: "t1".to_string(),
                    actor: "inventory".to_string(),
                    attempt_id: Some(t1_aid),
                    status: TaskStatus::Completed,
                    output: Some("inventory done".to_string()),
                    verdict: None,
                    findings: None,
                    changed_paths: None,
                    acceptance_results: None,
                    commands_run: None,
                },
                None,
            )
            .expect("complete t1");

        let after_t2 = engine
            .update_task(
                UpdateTaskInput {
                    team_id: "harbor-stockroom".to_string(),
                    task_id: "t2".to_string(),
                    actor: "receipt".to_string(),
                    attempt_id: Some(t2_aid),
                    status: TaskStatus::Completed,
                    output: Some("receipt done".to_string()),
                    verdict: None,
                    findings: None,
                    changed_paths: None,
                    acceptance_results: None,
                    commands_run: None,
                },
                None,
            )
            .expect("complete t2");

        // Verifier automatically claimed t3!
        assert_eq!(
            after_t2.team.find_task("t3").unwrap().status,
            TaskStatus::Claimed
        );
        let t3_aid = after_t2
            .team
            .find_task("t3")
            .unwrap()
            .attempt_id
            .clone()
            .unwrap();

        // 6. Send direct P2P message and complete t3 -> wakes captain
        let (msg, msg_dirs) = engine
            .send_message_op(SendMessageInput {
                team_id: "harbor-stockroom".to_string(),
                from: "verifier".to_string(),
                to: "captain".to_string(),
                content: "All tests pass!".to_string(),
                task_id: Some("t3".to_string()),
                attempt_id: Some(t3_aid.clone()),
            })
            .expect("send msg");
        assert_eq!(msg.to, "captain");
        assert_eq!(msg_dirs.len(), 1);

        let after_t3 = engine
            .update_task(
                UpdateTaskInput {
                    team_id: "harbor-stockroom".to_string(),
                    task_id: "t3".to_string(),
                    actor: "verifier".to_string(),
                    attempt_id: Some(t3_aid),
                    status: TaskStatus::Completed,
                    output: Some("npm test 100% pass".to_string()),
                    verdict: None,
                    findings: None,
                    changed_paths: None,
                    acceptance_results: None,
                    commands_run: None,
                },
                None,
            )
            .expect("complete t3");

        assert!(after_t3
            .directives
            .iter()
            .any(|d| matches!(d, EngineDirective::WakeCaptain { .. })));

        // 7. Archive retains full history
        let archived = engine
            .delete_and_archive("harbor-stockroom", None)
            .expect("archive");
        assert!(archived.extra.contains_key("archivePath"));

        let snap = collect_workspace_activity(engine.store(), Some("sess-cap-1"));
        assert!(snap.teams.is_empty());
        assert_eq!(snap.archived.len(), 1);
        assert_eq!(snap.archived[0].stats.completed, 3);
        assert_eq!(snap.archived[0].stats.message_count, 1);
    }

    #[test]
    fn quality_gate_review_failure_spawns_repair_and_next_review() {
        let (_dir, engine) = make_engine();

        engine
            .create_team(CreateTeamInput {
                name: "qg-team".to_string(),
                description: None,
                captain_session_id: "cap-qg".to_string(),
                profile: None,
                approval: Some("automatic".to_string()),
                members: vec![
                    PlanMemberInput {
                        name: "dev".to_string(),
                        role: Some("engineer".to_string()),
                        provider: None,
                        model: None,
                        reasoning_effort: None,
                        execution_prompt: None,
                        tool_policy: None,
                    },
                    PlanMemberInput {
                        name: "rev".to_string(),
                        role: Some("security reviewer".to_string()),
                        provider: None,
                        model: None,
                        reasoning_effort: None,
                        execution_prompt: None,
                        tool_policy: None,
                    },
                ],
                tasks: vec![],
                review_policy: None,
                captain_provider: None,
                captain_model: None,
                captain_reasoning_effort: None,
            })
            .expect("create");

        engine
            .bind_member_session("qg-team", "dev", "sub-dev", None)
            .expect("bind dev");
        engine
            .bind_member_session("qg-team", "rev", "sub-rev", None)
            .expect("bind rev");

        // Implementation task requires inScope & verify
        let bad_impl = engine.create_task(CreateTaskInput {
            team_id: "qg-team".to_string(),
            subject: "Impl auth".to_string(),
            description: None,
            assignee: Some("dev".to_string()),
            dependencies: vec![],
            kind: Some(TaskKind::Implementation),
            objective: Some("Add auth".to_string()),
            in_scope: None,
            out_of_scope: None,
            acceptance: Some(vec!["Auth works".to_string()]),
            verify: None,
            deliverables: None,
            non_goals: None,
            reviewed_task_id: None,
            coverage_of: None,
            resume: None,
            resume_reason: None,
        });
        assert!(matches!(
            bad_impl,
            Err(TeamsError::QualityGateViolation { .. })
        ));

        // Valid implementation task
        let t1 = engine
            .create_task(CreateTaskInput {
                team_id: "qg-team".to_string(),
                subject: "Impl auth".to_string(),
                description: None,
                assignee: Some("dev".to_string()),
                dependencies: vec![],
                kind: Some(TaskKind::Implementation),
                objective: Some("Add auth".to_string()),
                in_scope: Some(vec!["src/".to_string()]),
                out_of_scope: Some(vec!["deploy/".to_string()]),
                acceptance: Some(vec!["Auth works".to_string()]),
                verify: Some(vec!["cargo test".to_string()]),
                deliverables: None,
                non_goals: None,
                reviewed_task_id: None,
                coverage_of: None,
                resume: None,
                resume_reason: None,
            })
            .expect("t1");
        let t1_aid = t1.team.find_task("t1").unwrap().attempt_id.clone().unwrap();

        // Scope violation rejected on completion
        let bad_scope = engine.update_task(
            UpdateTaskInput {
                team_id: "qg-team".to_string(),
                task_id: "t1".to_string(),
                actor: "dev".to_string(),
                attempt_id: Some(t1_aid.clone()),
                status: TaskStatus::Completed,
                output: Some("edited deploy".to_string()),
                verdict: None,
                findings: None,
                changed_paths: Some(vec!["deploy/prod.yml".to_string()]),
                acceptance_results: None,
                commands_run: None,
            },
            None,
        );
        assert!(matches!(
            bad_scope,
            Err(TeamsError::QualityGateViolation { .. })
        ));

        // Complete t1 cleanly
        engine
            .update_task(
                UpdateTaskInput {
                    team_id: "qg-team".to_string(),
                    task_id: "t1".to_string(),
                    actor: "dev".to_string(),
                    attempt_id: Some(t1_aid),
                    status: TaskStatus::Completed,
                    output: Some("implemented auth".to_string()),
                    verdict: None,
                    findings: None,
                    changed_paths: Some(vec!["src/auth.rs".to_string()]),
                    acceptance_results: None,
                    commands_run: Some(vec![CommandResult {
                        command: "cargo test".to_string(),
                        status: CheckOutcome::Passed,
                        exit_code: Some(0),
                        evidence: None,
                    }]),
                },
                None,
            )
            .expect("complete t1");

        // Create review task t2
        let t2 = engine
            .create_task(CreateTaskInput {
                team_id: "qg-team".to_string(),
                subject: "Review auth".to_string(),
                description: None,
                assignee: Some("rev".to_string()),
                dependencies: vec!["t1".to_string()],
                kind: Some(TaskKind::Review),
                objective: Some("Audit auth".to_string()),
                in_scope: None,
                out_of_scope: None,
                acceptance: Some(vec!["No security issues".to_string()]),
                verify: None,
                deliverables: None,
                non_goals: None,
                reviewed_task_id: Some("t1".to_string()),
                coverage_of: None,
                resume: None,
                resume_reason: None,
            })
            .expect("t2");
        let t2_aid = t2.team.find_task("t2").unwrap().attempt_id.clone().unwrap();

        // Failing review with NeedsRevision + finding auto-spawns repair (t3) + re-review (t4)!
        let failed = engine
            .update_task(
                UpdateTaskInput {
                    team_id: "qg-team".to_string(),
                    task_id: "t2".to_string(),
                    actor: "rev".to_string(),
                    attempt_id: Some(t2_aid),
                    status: TaskStatus::Failed,
                    output: Some("Missing constant-time compare".to_string()),
                    verdict: Some(ReviewVerdict::NeedsRevision),
                    findings: Some(vec![ReviewFinding {
                        id: "SEC-001".to_string(),
                        severity: FindingSeverity::High,
                        file: Some("src/auth.rs".to_string()),
                        line: Some(42),
                        problem: "Non-constant-time token check".to_string(),
                        required_fix: "Use constant-time comparison".to_string(),
                        resolved: None,
                    }]),
                    changed_paths: None,
                    acceptance_results: None,
                    commands_run: None,
                },
                None,
            )
            .expect("fail review");

        assert!(failed.team.find_task("t3").is_some());
        assert!(failed.team.find_task("t4").is_some());
        let repair = failed.team.find_task("t3").unwrap();
        assert_eq!(repair.effective_kind(), TaskKind::Repair);
        assert_eq!(repair.dependencies, Vec::<String>::new()); // Must NOT depend on failed review!
        assert_eq!(
            repair.finding_source_task_ids.as_deref(),
            Some(&["t2".to_string()][..])
        );
        // Auto-claimed immediately by idle dev!
        assert_eq!(repair.status, TaskStatus::Claimed);
    }

    #[test]
    fn halt_resume_reassign_amend_and_cold_recovery() {
        let (_dir, engine) = make_engine();
        engine
            .create_team(CreateTeamInput {
                name: "ops-team".to_string(),
                description: None,
                captain_session_id: "cap-ops".to_string(),
                profile: None,
                approval: Some("manual".to_string()),
                members: vec![],
                tasks: vec![],
                review_policy: None,
                captain_provider: None,
                captain_model: None,
                captain_reasoning_effort: None,
            })
            .expect("create");

        engine
            .edit_plan(EditPlanInput {
                team_id: "ops-team".to_string(),
                description: Some("Updated plan".to_string()),
                members: Some(vec![PlanMemberInput {
                    name: "alice".to_string(),
                    role: Some("operator".to_string()),
                    provider: None,
                    model: None,
                    reasoning_effort: None,
                    execution_prompt: None,
                    tool_policy: None,
                }]),
                tasks: None,
                captain_provider: None,
                captain_model: None,
                captain_reasoning_effort: None,
            })
            .expect("edit plan");

        engine.approve_team("ops-team").expect("approve");
        engine
            .add_member(AddMemberInput {
                team_id: "ops-team".to_string(),
                name: "bob".to_string(),
                role: Some("engineer".to_string()),
                provider: None,
                model: None,
                reasoning_effort: None,
                execution_prompt: None,
                tool_policy: None,
                captain_provider: None,
                captain_model: None,
                captain_reasoning_effort: None,
            })
            .expect("add bob");
        engine
            .bind_member_session("ops-team", "alice", "sub-alice", None)
            .expect("bind alice");
        engine
            .bind_member_session("ops-team", "bob", "sub-bob", None)
            .expect("bind bob");

        engine
            .create_task(CreateTaskInput {
                team_id: "ops-team".to_string(),
                subject: "Deploy pipeline".to_string(),
                description: None,
                assignee: Some("alice".to_string()),
                dependencies: vec![],
                kind: None,
                objective: Some("old obj".to_string()),
                in_scope: None,
                out_of_scope: None,
                acceptance: None,
                verify: None,
                deliverables: None,
                non_goals: None,
                reviewed_task_id: None,
                coverage_of: None,
                resume: None,
                resume_reason: None,
            })
            .expect("t1");

        // Amend task records revision
        let amended = engine
            .amend_task(AmendTaskInput {
                team_id: "ops-team".to_string(),
                task_id: "t1".to_string(),
                reason: "clarify target".to_string(),
                objective: Some("new obj".to_string()),
                acceptance: None,
                verify: None,
                in_scope: None,
                out_of_scope: None,
            })
            .expect("amend");
        assert_eq!(
            amended
                .team
                .find_task("t1")
                .unwrap()
                .revisions
                .as_ref()
                .unwrap()
                .len(),
            1
        );

        // Reassign from alice to bob
        let reassigned = engine
            .reassign_task(&ReassignTaskInput {
                team_id: "ops-team".to_string(),
                task_id: "t1".to_string(),
                assignee: "bob".to_string(),
                reason: Some("load balance".to_string()),
            })
            .expect("reassign");
        assert_eq!(
            reassigned.team.find_task("t1").unwrap().assignee.as_deref(),
            Some("bob")
        );

        // Cold recovery retries open task with a fresh attempt
        let mut loaded = engine.store().load_team("ops-team").expect("load");
        let rec_dirs = recover_stranded_tasks(&mut loaded);
        assert_eq!(rec_dirs.len(), 1);

        // Halt rejects normal create_task, requires explicit resume
        engine.halt_team("ops-team", Some("pause")).expect("halt");
        let blocked = engine.create_task(CreateTaskInput {
            team_id: "ops-team".to_string(),
            subject: "Should fail".to_string(),
            description: None,
            assignee: None,
            dependencies: vec![],
            kind: None,
            objective: None,
            in_scope: None,
            out_of_scope: None,
            acceptance: None,
            verify: None,
            deliverables: None,
            non_goals: None,
            reviewed_task_id: None,
            coverage_of: None,
            resume: None,
            resume_reason: None,
        });
        assert!(matches!(blocked, Err(TeamsError::TeamHalted(_))));

        engine
            .resume_team("ops-team", "ready to continue")
            .expect("resume");
    }
}
