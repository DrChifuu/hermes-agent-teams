use crate::error::{Result, TeamsError};
use crate::types::{TaskStatus, TeamTask};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet, VecDeque};

/// Compute topological depth for every task in the DAG (0 = no dependencies).
/// Returns an error if a cycle exists.
pub fn compute_depths(tasks: &[TeamTask]) -> Result<BTreeMap<String, usize>> {
    let by_id: HashMap<&str, &TeamTask> = tasks.iter().map(|t| (t.id.as_str(), t)).collect();
    let mut in_degree: HashMap<&str, usize> = HashMap::with_capacity(tasks.len());
    let mut dependents: HashMap<&str, Vec<&str>> = HashMap::with_capacity(tasks.len());

    for task in tasks {
        in_degree.entry(task.id.as_str()).or_insert(0);
        for dep in &task.dependencies {
            if by_id.contains_key(dep.as_str()) {
                *in_degree.entry(task.id.as_str()).or_insert(0) += 1;
                dependents
                    .entry(dep.as_str())
                    .or_default()
                    .push(task.id.as_str());
            }
        }
    }

    let mut queue: VecDeque<&str> = VecDeque::new();
    let mut depths: BTreeMap<String, usize> = BTreeMap::new();

    for task in tasks {
        if in_degree.get(task.id.as_str()).copied().unwrap_or(0) == 0 {
            queue.push_back(task.id.as_str());
            depths.insert(task.id.clone(), 0);
        }
    }

    let mut visited = 0usize;
    while let Some(curr) = queue.pop_front() {
        visited += 1;
        let curr_depth = depths.get(curr).copied().unwrap_or(0);
        if let Some(children) = dependents.get(curr) {
            for &child in children {
                let next_depth = curr_depth + 1;
                let entry = depths.entry(child.to_string()).or_insert(0);
                if next_depth > *entry {
                    *entry = next_depth;
                }
                if let Some(deg) = in_degree.get_mut(child) {
                    *deg = deg.saturating_sub(1);
                    if *deg == 0 {
                        queue.push_back(child);
                    }
                }
            }
        }
    }

    if visited != tasks.len() {
        let cycle_node = tasks
            .iter()
            .find(|t| in_degree.get(t.id.as_str()).copied().unwrap_or(0) > 0)
            .map_or_else(|| "unknown".to_string(), |t| t.id.clone());
        return Err(TeamsError::DependencyCycle(cycle_node));
    }

    Ok(depths)
}

/// Validate that adding or editing a task with `task_id` and `deps` does not introduce a cycle
/// or reference unknown tasks.
pub fn validate_dependencies(existing: &[TeamTask], task_id: &str, deps: &[String]) -> Result<()> {
    let mut known: HashSet<&str> = existing.iter().map(|t| t.id.as_str()).collect();
    known.insert(task_id);

    for dep in deps {
        if dep == task_id {
            return Err(TeamsError::DependencyCycle(task_id.to_string()));
        }
        if !known.contains(dep.as_str()) {
            return Err(TeamsError::TaskNotFound(dep.clone()));
        }
    }

    // Build simulated task list to check acyclicity via Kahn's algorithm.
    let mut simulated: Vec<TeamTask> = existing.to_vec();
    if let Some(slot) = simulated.iter_mut().find(|t| t.id == task_id) {
        slot.dependencies = deps.to_vec();
    } else {
        simulated.push(TeamTask {
            profile_seed_id: None,
            id: task_id.to_string(),
            subject: String::new(),
            description: None,
            status: TaskStatus::Pending,
            assignee: None,
            dependencies: deps.to_vec(),
            output: None,
            attempt: None,
            attempt_id: None,
            handoff_id: None,
            handoff_from_member_id: None,
            reassigning: None,
            kind: None,
            round: None,
            verdict: None,
            findings: None,
            objective: None,
            in_scope: None,
            out_of_scope: None,
            acceptance: None,
            verify: None,
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
            source_task_id: None,
            finding_source_task_ids: None,
            source_finding_ids: None,
            coverage_of: None,
            revisions: None,
            created_at: 0,
            updated_at: 0,
        });
    }

    compute_depths(&simulated)?;
    Ok(())
}

/// Return the list of dependency task IDs that have NOT yet reached `Completed`.
pub fn unsatisfied_dependencies(tasks: &[TeamTask], task: &TeamTask) -> Vec<String> {
    let completed: HashSet<&str> = tasks
        .iter()
        .filter(|t| t.status == TaskStatus::Completed)
        .map(|t| t.id.as_str())
        .collect();

    task.dependencies
        .iter()
        .filter(|dep| !completed.contains(dep.as_str()))
        .cloned()
        .collect()
}

/// Whether a pending task has all dependencies satisfied and is not being reassigned.
pub fn is_task_ready(tasks: &[TeamTask], task: &TeamTask) -> bool {
    task.status == TaskStatus::Pending
        && !task.reassigning.unwrap_or(false)
        && unsatisfied_dependencies(tasks, task).is_empty()
}

/// Return direct downstream task IDs unlocked when `task_id` completes.
pub fn direct_unlocks(tasks: &[TeamTask], task_id: &str) -> Vec<String> {
    tasks
        .iter()
        .filter(|t| t.dependencies.iter().any(|d| d == task_id))
        .map(|t| t.id.clone())
        .collect()
}

/// Return full transitive upstream and downstream sets for UI highlighting.
pub fn lineage_sets(tasks: &[TeamTask], target_id: &str) -> (BTreeSet<String>, BTreeSet<String>) {
    let by_id: HashMap<&str, &TeamTask> = tasks.iter().map(|t| (t.id.as_str(), t)).collect();
    let mut upstream = BTreeSet::new();
    let mut stack = vec![target_id];
    while let Some(id) = stack.pop() {
        if let Some(task) = by_id.get(id) {
            for dep in &task.dependencies {
                if upstream.insert(dep.clone()) {
                    stack.push(dep.as_str());
                }
            }
        }
    }

    let mut downstream = BTreeSet::new();
    let mut stack = vec![target_id];
    while let Some(id) = stack.pop() {
        for task in tasks {
            if task.dependencies.iter().any(|d| d == id) && downstream.insert(task.id.clone()) {
                stack.push(task.id.as_str());
            }
        }
    }

    (upstream, downstream)
}
