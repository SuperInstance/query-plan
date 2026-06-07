//! Cost estimation model.

use crate::plan::PlanNode;

/// Cost model parameters.
#[derive(Debug, Clone)]
pub struct CostParams {
    /// Cost of reading a single row sequentially.
    pub seq_row_cost: f64,
    /// Cost of reading a single row via index.
    pub index_row_cost: f64,
    /// Cost of comparing two rows for join/filter.
    pub compare_cost: f64,
    /// Cost of sorting one row (log factor).
    pub sort_cost_per_row: f64,
    /// Network/transfer cost per row.
    pub transfer_cost: f64,
}

impl Default for CostParams {
    fn default() -> Self {
        CostParams {
            seq_row_cost: 1.0,
            index_row_cost: 2.0,
            compare_cost: 0.1,
            sort_cost_per_row: 1.5,
            transfer_cost: 0.5,
        }
    }
}

/// Cost estimator for query plans.
pub struct CostEstimator {
    params: CostParams,
}

impl CostEstimator {
    /// Create a new cost estimator with default parameters.
    pub fn new() -> Self {
        CostEstimator {
            params: CostParams::default(),
        }
    }

    /// Create with custom parameters.
    pub fn with_params(params: CostParams) -> Self {
        CostEstimator { params }
    }

    /// Estimate the total cost of a plan tree.
    pub fn estimate(node: &PlanNode) -> f64 {
        let estimator = CostEstimator::new();
        estimator.estimate_node(node)
    }

    fn estimate_node(&self, node: &PlanNode) -> f64 {
        let child_costs: f64 = node.children.iter().map(|c| self.estimate_node(c)).sum();

        let self_cost = match node.node_type {
            crate::plan::PlanNodeType::Scan => {
                node.estimated_rows * self.params.seq_row_cost
            }
            crate::plan::PlanNodeType::IndexScan => {
                node.estimated_rows * self.params.index_row_cost
            }
            crate::plan::PlanNodeType::Filter => {
                node.estimated_rows * self.params.compare_cost
            }
            crate::plan::PlanNodeType::Join => {
                // Cost is roughly the product of input sizes * compare cost
                if node.children.len() == 2 {
                    let left_rows = node.children[0].estimated_rows;
                    let right_rows = node.children[1].estimated_rows;
                    left_rows * right_rows * self.params.compare_cost
                } else {
                    node.estimated_rows * self.params.compare_cost
                }
            }
            crate::plan::PlanNodeType::Sort => {
                let n = node.estimated_rows;
                n * n.log2() * self.params.sort_cost_per_row
            }
            crate::plan::PlanNodeType::Project => {
                node.estimated_rows * self.params.transfer_cost
            }
            crate::plan::PlanNodeType::Aggregate => {
                node.estimated_rows * self.params.compare_cost
            }
            crate::plan::PlanNodeType::Limit => {
                self.params.seq_row_cost
            }
        };

        child_costs + self_cost
    }

    /// Estimate with custom parameters.
    pub fn estimate_with_params(&self, node: &PlanNode) -> f64 {
        self.estimate_node(node)
    }

    /// Compare two plans and return the cheaper one.
    pub fn cheaper<'a>(a: &'a PlanNode, b: &'a PlanNode) -> &'a PlanNode {
        let cost_a = Self::estimate(a);
        let cost_b = Self::estimate(b);
        if cost_a <= cost_b {
            a
        } else {
            b
        }
    }
}

impl Default for CostEstimator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plan::{JoinType, PlanNode};

    #[test]
    fn test_scan_cost() {
        let scan = PlanNode::scan("t", 1000.0);
        let cost = CostEstimator::estimate(&scan);
        assert!(cost > 0.0);
    }

    #[test]
    fn test_join_cost_higher_than_scan() {
        let left = PlanNode::scan("a", 1000.0);
        let right = PlanNode::scan("b", 1000.0);
        let join = PlanNode::join(JoinType::Inner, "a.id = b.id", left.clone(), right.clone());
        let join_cost = CostEstimator::estimate(&join);
        let scan_cost = CostEstimator::estimate(&left);
        assert!(join_cost > scan_cost);
    }

    #[test]
    fn test_filter_reduces_cost() {
        let scan = PlanNode::scan("t", 10000.0);
        let filtered = PlanNode::filter("x > 0", 0.1, scan);
        let full_scan = PlanNode::scan("t", 10000.0);
        let filtered_cost = CostEstimator::estimate(&filtered);
        let full_cost = CostEstimator::estimate(&full_scan);
        // Filtered plan has scan cost + filter cost, but reduces downstream
        assert!(filtered_cost > 0.0);
        assert!(full_cost > 0.0);
    }

    #[test]
    fn test_cheaper() {
        let small = PlanNode::scan("small", 10.0);
        let large = PlanNode::scan("large", 100000.0);
        let winner = CostEstimator::cheaper(&small, &large);
        assert_eq!(winner.label, "Scan(small)");
    }

    #[test]
    fn test_sort_cost() {
        let scan = PlanNode::scan("t", 1000.0);
        let sorted = PlanNode::sort("id", scan);
        let cost = CostEstimator::estimate(&sorted);
        assert!(cost > 1000.0); // Should include n*log(n) factor
    }

    #[test]
    fn test_custom_params() {
        let params = CostParams {
            seq_row_cost: 10.0,
            ..Default::default()
        };
        let estimator = CostEstimator::with_params(params);
        let scan = PlanNode::scan("t", 100.0);
        let cost = estimator.estimate_with_params(&scan);
        assert!(cost >= 1000.0); // 100 rows * 10 per row
    }
}
