//! Step dependency resolution with topological sorting
//!
//! Handles `depends_on` field in workflow steps to ensure correct execution order.

use std::collections::{HashMap, HashSet, VecDeque};
use crate::errors::QuicpulseError;
use super::workflow::WorkflowStep;

/// Result of dependency resolution
#[derive(Debug)]
pub struct DependencyOrder {
    /// Steps in execution order (indices into original step list)
    pub order: Vec<usize>,
    /// Steps that can run in parallel (grouped by level)
    pub levels: Vec<Vec<usize>>,
}

/// Build a dependency graph and return topologically sorted execution order
///
/// Returns an error if:
/// - A step depends on a non-existent step
/// - There is a dependency cycle
pub fn resolve_dependencies(steps: &[&WorkflowStep]) -> Result<DependencyOrder, QuicpulseError> {
    if steps.is_empty() {
        return Ok(DependencyOrder {
            order: vec![],
            levels: vec![],
        });
    }

    // Build name -> index mapping
    let name_to_idx: HashMap<&str, usize> = steps.iter()
        .enumerate()
        .map(|(i, s)| (s.name.as_str(), i))
        .collect();

    // Check for duplicate step names
    if name_to_idx.len() != steps.len() {
        // Find the duplicate
        let mut seen = HashSet::new();
        for step in steps {
            if !seen.insert(&step.name) {
                return Err(QuicpulseError::Argument(format!(
                    "Duplicate step name in workflow: {}", step.name
                )));
            }
        }
    }

    // Build adjacency list (dependency graph)
    // adj[i] contains the indices of steps that step i depends on
    let mut deps: Vec<Vec<usize>> = vec![Vec::new(); steps.len()];

    for (i, step) in steps.iter().enumerate() {
        for dep_name in &step.depends_on {
            match name_to_idx.get(dep_name.as_str()) {
                Some(&dep_idx) => {
                    deps[i].push(dep_idx);
                }
                None => {
                    return Err(QuicpulseError::Argument(format!(
                        "Step '{}' depends on non-existent step '{}'",
                        step.name, dep_name
                    )));
                }
            }
        }
    }

    // Kahn's algorithm for topological sort
    // Also builds levels for parallel execution

    // Calculate in-degree and build reverse adjacency (dependents)
    // in_degree[i] = number of steps that step i depends on
    // dependents[i] = steps that depend on step i
    let mut in_degree: Vec<usize> = vec![0; steps.len()];
    let mut dependents: Vec<Vec<usize>> = vec![Vec::new(); steps.len()];
    for (i, step_deps) in deps.iter().enumerate() {
        for &dep_idx in step_deps {
            dependents[dep_idx].push(i);
        }
        in_degree[i] = step_deps.len();
    }

    // Queue of steps with no remaining dependencies
    let mut queue: VecDeque<usize> = VecDeque::new();
    for (i, &deg) in in_degree.iter().enumerate() {
        if deg == 0 {
            queue.push_back(i);
        }
    }

    let mut order = Vec::with_capacity(steps.len());
    let mut levels: Vec<Vec<usize>> = Vec::new();

    // Process level by level for parallel execution grouping
    while !queue.is_empty() {
        // All steps in the current queue can run in parallel
        let level: Vec<usize> = queue.drain(..).collect();

        for &idx in &level {
            order.push(idx);

            // Reduce in-degree of dependents
            for &dependent_idx in &dependents[idx] {
                in_degree[dependent_idx] -= 1;
                if in_degree[dependent_idx] == 0 {
                    queue.push_back(dependent_idx);
                }
            }
        }

        levels.push(level);
    }

    // Check for cycles
    if order.len() != steps.len() {
        // Find steps in the cycle
        let in_cycle: Vec<&str> = steps.iter()
            .enumerate()
            .filter(|(i, _)| !order.contains(i))
            .map(|(_, s)| s.name.as_str())
            .collect();

        return Err(QuicpulseError::Argument(format!(
            "Dependency cycle detected involving steps: {}",
            in_cycle.join(", ")
        )));
    }

    Ok(DependencyOrder { order, levels })
}

/// Get steps in execution order, respecting depends_on
pub fn get_execution_order<'a>(
    steps: &[&'a WorkflowStep],
) -> Result<Vec<&'a WorkflowStep>, QuicpulseError> {
    let dep_order = resolve_dependencies(steps)?;
    Ok(dep_order.order.iter().map(|&i| steps[i]).collect())
}

/// Check if any step has dependencies defined
pub fn has_dependencies(steps: &[&WorkflowStep]) -> bool {
    steps.iter().any(|s| !s.depends_on.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_step(name: &str, depends_on: Vec<&str>) -> WorkflowStep {
        WorkflowStep {
            name: name.to_string(),
            depends_on: depends_on.into_iter().map(String::from).collect(),
            ..Default::default()
        }
    }

    #[test]
    fn test_no_dependencies() {
        let steps = vec![
            make_step("a", vec![]),
            make_step("b", vec![]),
            make_step("c", vec![]),
        ];
        let refs: Vec<&WorkflowStep> = steps.iter().collect();

        let result = resolve_dependencies(&refs).unwrap();
        assert_eq!(result.order.len(), 3);
        // All should be in level 0 (can run in parallel)
        assert_eq!(result.levels.len(), 1);
        assert_eq!(result.levels[0].len(), 3);
    }

    #[test]
    fn test_linear_dependencies() {
        let steps = vec![
            make_step("a", vec![]),
            make_step("b", vec!["a"]),
            make_step("c", vec!["b"]),
        ];
        let refs: Vec<&WorkflowStep> = steps.iter().collect();

        let result = resolve_dependencies(&refs).unwrap();
        assert_eq!(result.order, vec![0, 1, 2]); // a -> b -> c
        assert_eq!(result.levels.len(), 3);
    }

    #[test]
    fn test_diamond_dependencies() {
        // a -> b, a -> c, b -> d, c -> d
        let steps = vec![
            make_step("a", vec![]),
            make_step("b", vec!["a"]),
            make_step("c", vec!["a"]),
            make_step("d", vec!["b", "c"]),
        ];
        let refs: Vec<&WorkflowStep> = steps.iter().collect();

        let result = resolve_dependencies(&refs).unwrap();

        // a must be first, d must be last, b and c can be in any order
        assert_eq!(result.order[0], 0); // a
        assert_eq!(result.order[3], 3); // d

        // Check levels: a, then b&c, then d
        assert_eq!(result.levels.len(), 3);
        assert_eq!(result.levels[0], vec![0]); // a
        assert!(result.levels[1].contains(&1) && result.levels[1].contains(&2)); // b, c
        assert_eq!(result.levels[2], vec![3]); // d
    }

    #[test]
    fn test_cycle_detection() {
        let steps = vec![
            make_step("a", vec!["c"]),
            make_step("b", vec!["a"]),
            make_step("c", vec!["b"]),
        ];
        let refs: Vec<&WorkflowStep> = steps.iter().collect();

        let result = resolve_dependencies(&refs);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("cycle"));
    }

    #[test]
    fn test_missing_dependency() {
        let steps = vec![
            make_step("a", vec!["nonexistent"]),
        ];
        let refs: Vec<&WorkflowStep> = steps.iter().collect();

        let result = resolve_dependencies(&refs);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("non-existent"));
    }

    #[test]
    fn test_duplicate_step_names() {
        let steps = vec![
            make_step("a", vec![]),
            make_step("a", vec![]),
        ];
        let refs: Vec<&WorkflowStep> = steps.iter().collect();

        let result = resolve_dependencies(&refs);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("Duplicate"));
    }
}
