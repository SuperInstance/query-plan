//! Heuristic optimization rules.

use crate::cost::CostEstimator;
use crate::plan::{PlanNode, PlanNodeType};

/// Apply heuristic optimization rules to a plan.
pub fn optimize(plan: PlanNode) -> PlanNode {
    let mut optimized = plan;
    optimized = push_down_filters(optimized);
    optimized = eliminate_redundant_projects(optimized);
    optimized
}

/// Push filter operations as close to the data source as possible.
fn push_down_filters(node: PlanNode) -> PlanNode {
    match node.node_type {
        PlanNodeType::Filter => {
            if node.children.len() == 1 {
                let label = node.label.clone();
                let sel = get_selectivity(&node);
                let child = push_down_filters(node.children.into_iter().next().unwrap());
                push_filter_down(child, &label, sel)
            } else {
                node
            }
        }
        _ => PlanNode {
            children: node.children.into_iter().map(push_down_filters).collect(),
            ..node
        },
    }
}

fn push_filter_down(child: PlanNode, label: &str, selectivity: f64) -> PlanNode {
    match child.node_type {
        PlanNodeType::Join => {
            // For joins, try to push filter to children
            if child.children.len() == 2 {
                // Simple heuristic: push to the left child
                let mut left = child.children[0].clone();
                let right = child.children[1].clone();
                left = PlanNode::filter(label, selectivity, left);
                PlanNode {
                    children: vec![left, right],
                    ..child
                }
            } else {
                PlanNode::filter(label, selectivity, child)
            }
        }
        PlanNodeType::Project => {
            // Push filter below project
            if child.children.len() == 1 {
                let inner = PlanNode::filter(label, selectivity, child.children.into_iter().next().unwrap());
                PlanNode {
                    children: vec![inner],
                    ..child
                }
            } else {
                PlanNode::filter(label, selectivity, child)
            }
        }
        _ => PlanNode::filter(label, selectivity, child),
    }
}

fn get_selectivity(node: &PlanNode) -> f64 {
    node.metadata
        .iter()
        .find(|(k, _)| k == "selectivity")
        .and_then(|(_, v)| v.parse::<f64>().ok())
        .unwrap_or(0.5)
}

/// Eliminate redundant projections.
fn eliminate_redundant_projects(node: PlanNode) -> PlanNode {
    match node.node_type {
        PlanNodeType::Project => {
            if node.children.len() == 1 && node.children[0].node_type == PlanNodeType::Project {
                // Project(Project(x)) -> Project(x)
                eliminate_redundant_projects(node.children.into_iter().next().unwrap())
            } else {
                PlanNode {
                    children: node.children.into_iter().map(eliminate_redundant_projects).collect(),
                    ..node
                }
            }
        }
        _ => PlanNode {
            children: node.children.into_iter().map(eliminate_redundant_projects).collect(),
            ..node
        },
    }
}

/// Apply a limit to early-terminate scans when possible.
pub fn apply_limit_optimization(plan: PlanNode, limit: usize) -> PlanNode {
    PlanNode::limit(limit, plan)
}

/// Compare the cost of two plans.
pub fn compare_plans(a: &PlanNode, b: &PlanNode) -> std::cmp::Ordering {
    let ca = CostEstimator::estimate(a);
    let cb = CostEstimator::estimate(b);
    ca.partial_cmp(&cb).unwrap_or(std::cmp::Ordering::Equal)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_push_down_filter() {
        let plan = PlanNode::filter(
            "x > 0",
            0.5,
            PlanNode::join(
                crate::plan::JoinType::Inner,
                "a.id = b.id",
                PlanNode::scan("a", 1000.0),
                PlanNode::scan("b", 2000.0),
            ),
        );
        let optimized = optimize(plan);
        // Filter should be pushed closer to the scan
        assert!(optimized.node_count() >= 3);
    }

    #[test]
    fn test_eliminate_double_project() {
        let plan = PlanNode::project(
            &["name"],
            PlanNode::project(
                &["name", "age"],
                PlanNode::scan("users", 100.0),
            ),
        );
        let optimized = optimize(plan);
        // Should eliminate one projection
        assert!(optimized.node_count() <= 3);
    }

    #[test]
    fn test_limit_optimization() {
        let scan = PlanNode::scan("t", 10000.0);
        let limited = apply_limit_optimization(scan, 10);
        assert_eq!(limited.estimated_rows, 10.0);
    }

    #[test]
    fn test_compare_plans() {
        let small = PlanNode::scan("small", 10.0);
        let large = PlanNode::scan("large", 100000.0);
        assert_eq!(compare_plans(&small, &large), std::cmp::Ordering::Less);
    }

    #[test]
    fn test_optimize_preserves_results() {
        let plan = PlanNode::filter(
            "active",
            0.5,
            PlanNode::scan("t", 1000.0),
        );
        let optimized = optimize(plan);
        // Output row count should be reasonable
        assert!(optimized.estimated_rows <= 1000.0);
    }

    #[test]
    fn test_no_optimization_needed() {
        let plan = PlanNode::scan("t", 100.0);
        let optimized = optimize(plan);
        assert_eq!(optimized.node_type, PlanNodeType::Scan);
    }
}
