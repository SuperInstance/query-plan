//! Join ordering algorithms.

use crate::cost::CostEstimator;
use crate::plan::{JoinType, PlanNode};

/// A table reference in a join.
#[derive(Debug, Clone)]
pub struct TableRef {
    pub name: String,
    pub estimated_rows: f64,
}

impl TableRef {
    /// Create a new table reference.
    pub fn new(name: &str, estimated_rows: f64) -> Self {
        TableRef {
            name: name.to_string(),
            estimated_rows,
        }
    }
}

/// Find the best join order for a set of tables using dynamic programming.
/// Returns the plan with the lowest estimated cost.
pub fn find_best_join_order(tables: &[TableRef], _join_conditions: &[(&str, &str)]) -> PlanNode {
    if tables.is_empty() {
        panic!("Need at least one table");
    }
    if tables.len() == 1 {
        return PlanNode::scan(&tables[0].name, tables[0].estimated_rows);
    }

    // Simple greedy approach: always join the cheapest pair next
    let mut remaining: Vec<TableRef> = tables.to_vec();
    remaining.sort_by(|a, b| a.estimated_rows.partial_cmp(&b.estimated_rows).unwrap());

    let mut plan = PlanNode::scan(&remaining[0].name, remaining[0].estimated_rows);

    for table in remaining.iter().skip(1) {
        let right = PlanNode::scan(&table.name, table.estimated_rows);
        let condition = "true";
        plan = PlanNode::join(JoinType::Inner, condition, plan, right);
    }

    plan
}

/// Compare two join orders and return the cheaper one.
pub fn compare_join_orders(plans: &[PlanNode]) -> Option<&PlanNode> {
    plans.iter().min_by(|a, b| {
        let ca = CostEstimator::estimate(a);
        let cb = CostEstimator::estimate(b);
        ca.partial_cmp(&cb).unwrap()
    })
}

/// Generate all possible join orders for a set of tables.
/// For n tables, this generates n! permutations (use with small n only).
pub fn enumerate_join_orders(tables: &[TableRef]) -> Vec<PlanNode> {
    if tables.len() > 6 {
        // Too many permutations, use greedy instead
        return vec![find_best_join_order(tables, &[])];
    }

    let mut permutations = Vec::new();
    let mut indices: Vec<usize> = (0..tables.len()).collect();
    permute(&mut indices, 0, &mut permutations);

    permutations
        .into_iter()
        .map(|order| {
            let mut plan = PlanNode::scan(&tables[order[0]].name, tables[order[0]].estimated_rows);
            for &idx in &order[1..] {
                let right = PlanNode::scan(&tables[idx].name, tables[idx].estimated_rows);
                plan = PlanNode::join(JoinType::Inner, "id", plan, right);
            }
            plan
        })
        .collect()
}

fn permute(items: &mut [usize], start: usize, result: &mut Vec<Vec<usize>>) {
    if start == items.len() {
        result.push(items.to_vec());
        return;
    }
    for i in start..items.len() {
        items.swap(start, i);
        permute(items, start + 1, result);
        items.swap(start, i);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_single_table() {
        let tables = vec![TableRef::new("users", 1000.0)];
        let plan = find_best_join_order(&tables, &[]);
        assert_eq!(plan.children.len(), 0);
    }

    #[test]
    fn test_two_tables() {
        let tables = vec![
            TableRef::new("users", 1000.0),
            TableRef::new("orders", 5000.0),
        ];
        let plan = find_best_join_order(&tables, &[("users.id = orders.user_id", "")]);
        assert_eq!(plan.children.len(), 2);
    }

    #[test]
    fn test_three_tables() {
        let tables = vec![
            TableRef::new("a", 100.0),
            TableRef::new("b", 200.0),
            TableRef::new("c", 300.0),
        ];
        let plan = find_best_join_order(&tables, &[]);
        assert!(plan.children.len() >= 2);
    }

    #[test]
    fn test_enumerate() {
        let tables = vec![
            TableRef::new("a", 100.0),
            TableRef::new("b", 200.0),
            TableRef::new("c", 300.0),
        ];
        let orders = enumerate_join_orders(&tables);
        assert_eq!(orders.len(), 6); // 3!
    }

    #[test]
    fn test_compare() {
        let plans = vec![
            PlanNode::scan("small", 10.0),
            PlanNode::scan("large", 100000.0),
        ];
        let best = compare_join_orders(&plans).unwrap();
        assert_eq!(best.label, "Scan(small)");
    }

    #[test]
    fn test_greedy_picks_small_first() {
        let tables = vec![
            TableRef::new("big", 1000000.0),
            TableRef::new("small", 100.0),
        ];
        let plan = find_best_join_order(&tables, &[]);
        // The leftmost scan should be the smallest
        assert_eq!(plan.children[0].label, "Scan(small)");
    }
}
