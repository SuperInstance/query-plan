//! Plan enumeration.

use crate::cost::CostEstimator;
use crate::join::{self, TableRef};
use crate::plan::{JoinType, PlanNode};

/// Result of plan enumeration.
#[derive(Debug)]
pub struct EnumerationResult {
    /// All enumerated plans.
    pub plans: Vec<PlanNode>,
    /// Index of the best plan.
    pub best_index: usize,
    /// Cost of the best plan.
    pub best_cost: f64,
}

impl EnumerationResult {
    /// Get the best plan.
    pub fn best_plan(&self) -> &PlanNode {
        &self.plans[self.best_index]
    }

    /// Number of plans enumerated.
    pub fn count(&self) -> usize {
        self.plans.len()
    }
}

/// Enumerate possible query plans for a set of tables with optional filters.
pub fn enumerate_plans(
    tables: &[TableRef],
    filters: &[(&str, f64)],
    projections: &[&str],
) -> EnumerationResult {
    let mut plans = Vec::new();

    // Enumerate join orders
    let join_orders = join::enumerate_join_orders(tables);

    for join_plan in join_orders {
        // Try different filter placements for each join order
        let with_filters = apply_filters(join_plan, filters);
        let with_projection = apply_projection(with_filters, projections);

        plans.push(with_projection);
    }

    // Find the best plan
    let mut best_index = 0;
    let mut best_cost = f64::MAX;
    for (i, plan) in plans.iter().enumerate() {
        let cost = CostEstimator::estimate(plan);
        if cost < best_cost {
            best_cost = cost;
            best_index = i;
        }
    }

    EnumerationResult {
        plans,
        best_index,
        best_cost,
    }
}

fn apply_filters(mut plan: PlanNode, filters: &[(&str, f64)]) -> PlanNode {
    for (predicate, selectivity) in filters {
        plan = PlanNode::filter(predicate, *selectivity, plan);
    }
    plan
}

fn apply_projection(plan: PlanNode, projections: &[&str]) -> PlanNode {
    if projections.is_empty() {
        plan
    } else {
        PlanNode::project(projections, plan)
    }
}

/// Enumerate plans with different join types for two tables.
pub fn enumerate_join_types(
    left: &TableRef,
    right: &TableRef,
    condition: &str,
) -> Vec<PlanNode> {
    let join_types = [
        JoinType::Inner,
        JoinType::LeftOuter,
        JoinType::RightOuter,
        JoinType::Cross,
    ];

    join_types
        .into_iter()
        .map(|jt| {
            PlanNode::join(
                jt,
                condition,
                PlanNode::scan(&left.name, left.estimated_rows),
                PlanNode::scan(&right.name, right.estimated_rows),
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_enumerate_two_tables() {
        let tables = vec![
            TableRef::new("a", 100.0),
            TableRef::new("b", 200.0),
        ];
        let result = enumerate_plans(&tables, &[], &[]);
        assert_eq!(result.count(), 2); // 2! permutations
        assert!(result.best_cost > 0.0);
    }

    #[test]
    fn test_enumerate_with_filters() {
        let tables = vec![
            TableRef::new("users", 1000.0),
        ];
        let filters = vec![("active = true", 0.5)];
        let result = enumerate_plans(&tables, &filters, &["name"]);
        assert!(result.count() >= 1);
        let best = result.best_plan();
        assert!(best.depth() >= 2); // scan + filter + project
    }

    #[test]
    fn test_enumerate_join_types() {
        let left = TableRef::new("a", 100.0);
        let right = TableRef::new("b", 200.0);
        let plans = enumerate_join_types(&left, &right, "a.id = b.id");
        assert_eq!(plans.len(), 4);
    }

    #[test]
    fn test_best_plan_is_cheapest() {
        let tables = vec![
            TableRef::new("small", 10.0),
            TableRef::new("large", 100000.0),
        ];
        let result = enumerate_plans(&tables, &[], &[]);
        let best = result.best_plan();
        let best_cost = CostEstimator::estimate(best);
        for plan in &result.plans {
            let cost = CostEstimator::estimate(plan);
            assert!(best_cost <= cost);
        }
    }

    #[test]
    fn test_single_table_enumeration() {
        let tables = vec![TableRef::new("t", 100.0)];
        let result = enumerate_plans(&tables, &[], &[]);
        assert_eq!(result.count(), 1);
    }
}
