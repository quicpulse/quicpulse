//! Unit tests for Workflow DAG dependency resolution and cycle detection

use quicpulse::pipeline::dependency::resolve_dependencies;
use quicpulse::pipeline::workflow::WorkflowStep;

fn create_step(name: &str, depends_on: Vec<&str>) -> WorkflowStep {
    WorkflowStep {
        name: name.to_string(),
        depends_on: depends_on.into_iter().map(String::from).collect(),
        ..Default::default()
    }
}

#[test]
fn test_dag_empty_steps() {
    let steps: Vec<WorkflowStep> = vec![];
    let step_refs: Vec<&WorkflowStep> = steps.iter().collect();
    let order = resolve_dependencies(&step_refs).unwrap();
    assert!(order.order.is_empty());
    assert!(order.levels.is_empty());
}

#[test]
fn test_dag_linear_dependencies() {
    let s1 = create_step("step1", vec![]);
    let s2 = create_step("step2", vec!["step1"]);
    let s3 = create_step("step3", vec!["step2"]);
    let steps = vec![&s1, &s2, &s3];

    let order = resolve_dependencies(&steps).unwrap();
    assert_eq!(order.order, vec![0, 1, 2]);
    assert_eq!(order.levels.len(), 3);
}

#[test]
fn test_dag_diamond_graph() {
    let s1 = create_step("root", vec![]);
    let s2 = create_step("branch_a", vec!["root"]);
    let s3 = create_step("branch_b", vec!["root"]);
    let s4 = create_step("join", vec!["branch_a", "branch_b"]);
    let steps = vec![&s1, &s2, &s3, &s4];

    let order = resolve_dependencies(&steps).unwrap();
    assert_eq!(order.order[0], 0); // root first
    assert_eq!(order.order[3], 3); // join last
    assert_eq!(order.levels.len(), 3);
    assert_eq!(order.levels[0], vec![0]);
    assert_eq!(order.levels[1], vec![1, 2]);
    assert_eq!(order.levels[2], vec![3]);
}

#[test]
fn test_dag_cycle_detection() {
    let s1 = create_step("node_a", vec!["node_b"]);
    let s2 = create_step("node_b", vec!["node_a"]);
    let steps = vec![&s1, &s2];

    let result = resolve_dependencies(&steps);
    assert!(result.is_err(), "Expected cycle detection error");
}

#[test]
fn test_dag_missing_dependency() {
    let s1 = create_step("node_a", vec!["non_existent_node"]);
    let steps = vec![&s1];

    let result = resolve_dependencies(&steps);
    assert!(result.is_err(), "Expected missing dependency error");
}

#[test]
fn test_dag_duplicate_step_names() {
    let s1 = create_step("same_name", vec![]);
    let s2 = create_step("same_name", vec![]);
    let steps = vec![&s1, &s2];

    let result = resolve_dependencies(&steps);
    assert!(result.is_err(), "Expected duplicate step name error");
}
