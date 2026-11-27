use crate::error::{Result, TeamsError};
use crate::types::{
    now_ms, AcceptanceResult, ArtifactDigest, ArtifactScopeGap, ArtifactSnapshot, CheckOutcome,
    CommandResult, ReviewFinding, ReviewVerdict, TaskKind, TaskStatus, TeamState, TeamTask,
};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Component, Path};

/// Validate quality-gate creation contracts (matching `dsh-agent-teams/docs/quality-gates.md`).
pub fn validate_task_creation_contract(task: &TeamTask) -> Result<()> {
    let kind = task.effective_kind();
    if !kind.is_quality() {
        return Ok(());
    }

    if task
        .objective
        .as_deref()
        .is_none_or(|s| s.trim().is_empty())
    {
        return Err(TeamsError::QualityGateViolation {
            task_id: task.id.clone(),
            reason: format!("quality task ({kind:?}) requires non-empty 'objective'"),
        });
    }

    if task.acceptance.as_ref().is_none_or(Vec::is_empty) {
        return Err(TeamsError::QualityGateViolation {
            task_id: task.id.clone(),
            reason: format!("quality task ({kind:?}) requires non-empty 'acceptance' criteria"),
        });
    }

    if kind.requires_scope_and_verify() {
        if task.in_scope.as_ref().is_none_or(Vec::is_empty) {
            return Err(TeamsError::QualityGateViolation {
                task_id: task.id.clone(),
                reason: format!("{kind:?} task requires non-empty 'inScope'"),
            });
        }
        if task.verify.as_ref().is_none_or(Vec::is_empty) {
            return Err(TeamsError::QualityGateViolation {
                task_id: task.id.clone(),
                reason: format!("{kind:?} task requires non-empty 'verify' commands"),
            });
        }
    }

    Ok(())
}

/// Parameters evaluated when closing a quality task as `Completed` or `Failed`.
#[derive(Debug, Clone, Default)]
pub struct QualityUpdateCheck<'a> {
    pub verdict: Option<ReviewVerdict>,
    pub findings: Option<&'a [ReviewFinding]>,
    pub changed_paths: Option<&'a [String]>,
    pub acceptance_results: Option<&'a [AcceptanceResult]>,
    pub commands_run: Option<&'a [CommandResult]>,
}

/// Enforce quality-gate completion/failure contracts before transitioning a task.
pub fn validate_quality_transition(
    task: &TeamTask,
    target_status: TaskStatus,
    check: &QualityUpdateCheck<'_>,
) -> Result<()> {
    let kind = task.effective_kind();
    if !kind.is_quality() || !target_status.is_terminal() || target_status == TaskStatus::Cancelled
    {
        return Ok(());
    }

    if kind.requires_verdict() {
        let verdict =
            check
                .verdict
                .or(task.verdict)
                .ok_or_else(|| TeamsError::QualityGateViolation {
                    task_id: task.id.clone(),
                    reason: format!(
                        "{kind:?} task requires 'verdict' (pass | needs_revision | reject)"
                    ),
                })?;

        match (target_status, verdict) {
            (TaskStatus::Completed, ReviewVerdict::Pass) => {}
            (TaskStatus::Completed, _) => {
                return Err(TeamsError::QualityGateViolation {
                    task_id: task.id.clone(),
                    reason: format!(
                        "{kind:?} task can only be marked 'completed' with verdict='pass'; use status='failed' for {verdict:?}"
                    ),
                });
            }
            (TaskStatus::Failed, ReviewVerdict::NeedsRevision | ReviewVerdict::Reject) => {
                let findings = check.findings.or(task.findings.as_deref()).unwrap_or(&[]);
                if findings.is_empty() {
                    return Err(TeamsError::QualityGateViolation {
                        task_id: task.id.clone(),
                        reason: format!(
                            "{kind:?} task failing with {verdict:?} must include at least one structured finding"
                        ),
                    });
                }
            }
            (TaskStatus::Failed, ReviewVerdict::Pass) => {
                return Err(TeamsError::QualityGateViolation {
                    task_id: task.id.clone(),
                    reason: "cannot mark failed with verdict='pass'".to_string(),
                });
            }
            _ => {}
        }
    }

    if target_status == TaskStatus::Completed && kind.requires_scope_and_verify() {
        // Completion-time scope audit against declared inScope/outOfScope.
        if let Some(changed) = check.changed_paths.or(task.changed_paths.as_deref()) {
            if let Some(out_of_scope) = task.out_of_scope.as_deref() {
                for path in changed {
                    if path_matches_any(path, out_of_scope) {
                        return Err(TeamsError::QualityGateViolation {
                            task_id: task.id.clone(),
                            reason: format!(
                                "changed path '{path}' violates declared outOfScope boundary"
                            ),
                        });
                    }
                }
            }
            if let Some(in_scope) = task.in_scope.as_deref() {
                for path in changed {
                    if !path_matches_any(path, in_scope) {
                        return Err(TeamsError::QualityGateViolation {
                            task_id: task.id.clone(),
                            reason: format!(
                                "changed path '{path}' is outside declared inScope ({in_scope:?})"
                            ),
                        });
                    }
                }
            }
        }

        if let Some(results) = check
            .acceptance_results
            .or(task.acceptance_results.as_deref())
        {
            if results.iter().any(|r| r.status == CheckOutcome::Failed) {
                return Err(TeamsError::QualityGateViolation {
                    task_id: task.id.clone(),
                    reason:
                        "cannot mark completed while an acceptance criterion has status='failed'"
                            .to_string(),
                });
            }
        }

        if let Some(cmds) = check.commands_run.or(task.commands_run.as_deref()) {
            if cmds.iter().any(|c| c.status == CheckOutcome::Failed) {
                return Err(TeamsError::QualityGateViolation {
                    task_id: task.id.clone(),
                    reason: "cannot mark completed while a verification command failed".to_string(),
                });
            }
        }
    }

    Ok(())
}

/// When a `Review` task fails with `NeedsRevision`, automatically create a `Repair` task
/// (not depending on the failed review!) + next-round `Review` task, unless the review-loop
/// ceiling is reached (in which case mark `team.escalated = Some(true)`).
pub fn maybe_spawn_repair_loop(team: &mut TeamState, failed_review_id: &str) -> Vec<String> {
    let Some(review) = team.find_task(failed_review_id).cloned() else {
        return Vec::new();
    };
    if review.effective_kind() != TaskKind::Review
        || review.status != TaskStatus::Failed
        || review.verdict != Some(ReviewVerdict::NeedsRevision)
    {
        return Vec::new();
    }

    let current_round = review.round.unwrap_or(1);
    let max_rounds = team
        .review_policy
        .as_ref()
        .and_then(|p| p.code_max_rounds)
        .unwrap_or(3);

    if current_round >= max_rounds {
        team.escalated = Some(true);
        return Vec::new();
    }

    let source_task = review
        .reviewed_task_id
        .as_deref()
        .and_then(|id| team.find_task(id))
        .cloned();

    let next_round = current_round + 1;
    let now = now_ms();

    // 1) Create repair task (no dependency on failed review, uses findingSourceTaskIds instead).
    team.task_seq += 1;
    let repair_id = format!("t{}", team.task_seq);
    let finding_ids: Vec<String> = review
        .findings
        .as_ref()
        .map(|fs| fs.iter().map(|f| f.id.clone()).collect())
        .unwrap_or_default();

    let repair_assignee = source_task
        .as_ref()
        .and_then(|t| t.assignee.clone())
        .or_else(|| review.assignee.clone());

    let repair_task = TeamTask {
        profile_seed_id: None,
        id: repair_id.clone(),
        subject: format!("Repair findings from {} (round {next_round})", review.id),
        description: Some(format!(
            "Address review findings from {} ({finding_ids:?}).",
            review.id
        )),
        status: TaskStatus::Pending,
        assignee: repair_assignee,
        dependencies: Vec::new(),
        output: None,
        attempt: None,
        attempt_id: None,
        handoff_id: None,
        handoff_from_member_id: None,
        reassigning: None,
        kind: Some(TaskKind::Repair),
        round: Some(next_round),
        verdict: None,
        findings: None,
        objective: Some(format!("Resolve all findings from review {}", review.id)),
        in_scope: source_task
            .as_ref()
            .and_then(|t| t.in_scope.clone())
            .or_else(|| Some(vec![".".to_string()])),
        out_of_scope: source_task.as_ref().and_then(|t| t.out_of_scope.clone()),
        acceptance: Some(vec![format!(
            "All findings from {} resolved and verified",
            review.id
        )]),
        verify: source_task
            .as_ref()
            .and_then(|t| t.verify.clone())
            .or_else(|| Some(vec!["echo verified".to_string()])),
        deliverables: None,
        non_goals: None,
        changed_paths: None,
        acceptance_results: None,
        commands_run: None,
        supplemental_evidence: None,
        reviewed_task_id: None,
        reviewed_attempt: None,
        reviewed_artifacts: None,
        reviewed_gaps: None,
        reviewed_scope: None,
        reviewed_during_drift: None,
        verdict_start_snapshot: None,
        contract_snapshot: None,
        source_task_id: source_task.as_ref().map(|t| t.id.clone()),
        finding_source_task_ids: Some(vec![review.id.clone()]),
        source_finding_ids: Some(finding_ids),
        coverage_of: review.coverage_of.clone(),
        revisions: None,
        created_at: now,
        updated_at: now,
    };

    // 2) Create next-round review task depending on the repair task.
    team.task_seq += 1;
    let next_review_id = format!("t{}", team.task_seq);
    let next_review_task = TeamTask {
        profile_seed_id: None,
        id: next_review_id.clone(),
        subject: format!("Re-review {repair_id} (round {next_round})"),
        description: Some(format!(
            "Verify that findings from {} were resolved in {}.",
            review.id, repair_id
        )),
        status: TaskStatus::Pending,
        assignee: review.assignee.clone(),
        dependencies: vec![repair_id.clone()],
        output: None,
        attempt: None,
        attempt_id: None,
        handoff_id: None,
        handoff_from_member_id: None,
        reassigning: None,
        kind: Some(TaskKind::Review),
        round: Some(next_round),
        verdict: None,
        findings: None,
        objective: review.objective.clone(),
        in_scope: review.in_scope.clone(),
        out_of_scope: review.out_of_scope.clone(),
        acceptance: review.acceptance.clone(),
        verify: review.verify.clone(),
        deliverables: None,
        non_goals: None,
        changed_paths: None,
        acceptance_results: None,
        commands_run: None,
        supplemental_evidence: None,
        reviewed_task_id: Some(repair_id.clone()),
        reviewed_attempt: None,
        reviewed_artifacts: None,
        reviewed_gaps: None,
        reviewed_scope: review.reviewed_scope.clone(),
        reviewed_during_drift: None,
        verdict_start_snapshot: None,
        contract_snapshot: None,
        source_task_id: None,
        finding_source_task_ids: None,
        source_finding_ids: None,
        coverage_of: review.coverage_of.clone(),
        revisions: None,
        created_at: now,
        updated_at: now,
    };

    team.tasks.push(repair_task);
    team.tasks.push(next_review_task);

    // 3) Rewire any downstream pending tasks that depended on `failed_review_id`
    //    so they now wait on `next_review_id` instead of deadlocking on the failed review.
    for task in &mut team.tasks {
        if task.status == TaskStatus::Pending
            && task.id != repair_id
            && task.id != next_review_id
            && task.dependencies.iter().any(|d| d == failed_review_id)
        {
            for dep in &mut task.dependencies {
                if dep == failed_review_id {
                    dep.clone_from(&next_review_id);
                }
            }
            task.updated_at = now;
        }
    }

    vec![repair_id, next_review_id]
}

/// Capture SHA-256 digests of workspace-relative paths for audit / drift detection.
pub fn snapshot_artifacts(workspace: &Path, paths: &[String]) -> ArtifactSnapshot {
    let mut artifacts = Vec::new();
    let mut gaps = Vec::new();
    let canonical_ws = fs::canonicalize(workspace).ok();

    for raw in paths {
        let rel = raw.trim().trim_start_matches("./");
        if rel.is_empty() || !is_safe_relative(rel) {
            gaps.push(ArtifactScopeGap {
                pattern: raw.clone(),
                reason: "path is not workspace-relative".to_string(),
            });
            continue;
        }
        let full = workspace.join(rel);
        if !full.exists() {
            artifacts.push(ArtifactDigest {
                path: rel.to_string(),
                present: false,
                sha256: None,
                size: None,
                mtime_ms: None,
                unreadable: None,
                reason: None,
            });
            continue;
        }

        if let (Some(ws_root), Ok(canon)) = (&canonical_ws, fs::canonicalize(&full)) {
            if !canon.starts_with(ws_root) {
                artifacts.push(ArtifactDigest {
                    path: rel.to_string(),
                    present: true,
                    sha256: None,
                    size: None,
                    mtime_ms: None,
                    unreadable: Some(true),
                    reason: Some("symlink escapes workspace".to_string()),
                });
                continue;
            }
        }

        match fs::read(&full) {
            Ok(bytes) => {
                let mut hasher = Sha256::new();
                hasher.update(&bytes);
                let hex = format!("{:x}", hasher.finalize());
                let meta = fs::metadata(&full).ok();
                let mtime_ms = meta
                    .as_ref()
                    .and_then(|m| m.modified().ok())
                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                    .map(|d| d.as_millis() as u64);
                artifacts.push(ArtifactDigest {
                    path: rel.to_string(),
                    present: true,
                    sha256: Some(hex),
                    size: Some(bytes.len() as u64),
                    mtime_ms,
                    unreadable: None,
                    reason: None,
                });
            }
            Err(err) => {
                artifacts.push(ArtifactDigest {
                    path: rel.to_string(),
                    present: true,
                    sha256: None,
                    size: None,
                    mtime_ms: None,
                    unreadable: Some(true),
                    reason: Some(err.to_string()),
                });
            }
        }
    }

    ArtifactSnapshot {
        at: now_ms(),
        artifacts,
        gaps: if gaps.is_empty() { None } else { Some(gaps) },
    }
}

fn is_safe_relative(p: &str) -> bool {
    let path = Path::new(p);
    if path.is_absolute() {
        return false;
    }
    !path.components().any(|c| {
        matches!(
            c,
            Component::ParentDir | Component::RootDir | Component::Prefix(_)
        )
    })
}

fn path_matches_any(path: &str, patterns: &[String]) -> bool {
    let norm = path.trim().trim_start_matches("./");
    patterns.iter().any(|pat| {
        let p = pat.trim().trim_start_matches("./");
        if p == "." || p == "*" || p == "**" {
            return true;
        }
        if let Some(prefix) = p.strip_suffix("/**") {
            return norm == prefix || norm.starts_with(&format!("{prefix}/"));
        }
        if let Some(prefix) = p.strip_suffix("/*") {
            return norm.starts_with(&format!("{prefix}/"));
        }
        if let Some(prefix) = p.strip_suffix('/') {
            return norm.starts_with(&format!("{prefix}/"));
        }
        norm == p || norm.starts_with(&format!("{p}/"))
    })
}
