//! Plan node types and structure.

/// Types of plan nodes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlanNodeType {
    /// Full table scan.
    Scan,
    /// Index scan.
    IndexScan,
    /// Filter (WHERE clause).
    Filter,
    /// Join operation.
    Join,
    /// Projection (SELECT columns).
    Project,
    /// Sort operation.
    Sort,
    /// Aggregate operation.
    Aggregate,
    /// Limit operation.
    Limit,
}

/// Join type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JoinType {
    Inner,
    LeftOuter,
    RightOuter,
    FullOuter,
    Cross,
}

/// A node in the query plan tree.
#[derive(Debug, Clone)]
pub struct PlanNode {
    /// Type of this node.
    pub node_type: PlanNodeType,
    /// Human-readable label.
    pub label: String,
    /// Estimated number of rows output.
    pub estimated_rows: f64,
    /// Children nodes (inputs).
    pub children: Vec<PlanNode>,
    /// Estimated cost.
    pub estimated_cost: f64,
    /// Extra metadata.
    pub metadata: Vec<(String, String)>,
}

impl PlanNode {
    /// Create a new plan node.
    pub fn new(node_type: PlanNodeType, label: &str) -> Self {
        PlanNode {
            node_type,
            label: label.to_string(),
            estimated_rows: 0.0,
            children: Vec::new(),
            estimated_cost: 0.0,
            metadata: Vec::new(),
        }
    }

    /// Create a scan node.
    pub fn scan(table: &str, rows: f64) -> Self {
        let mut node = PlanNode::new(PlanNodeType::Scan, &format!("Scan({})", table));
        node.estimated_rows = rows;
        node.metadata.push(("table".to_string(), table.to_string()));
        node
    }

    /// Create a filter node.
    pub fn filter(predicate: &str, selectivity: f64, child: PlanNode) -> Self {
        let mut node = PlanNode::new(PlanNodeType::Filter, &format!("Filter({})", predicate));
        node.estimated_rows = child.estimated_rows * selectivity;
        node.children.push(child);
        node.metadata.push(("predicate".to_string(), predicate.to_string()));
        node.metadata.push(("selectivity".to_string(), selectivity.to_string()));
        node
    }

    /// Create a join node.
    pub fn join(join_type: JoinType, condition: &str, left: PlanNode, right: PlanNode) -> Self {
        let estimated_rows = match join_type {
            JoinType::Cross => left.estimated_rows * right.estimated_rows,
            _ => left.estimated_rows * right.estimated_rows * 0.1,
        };
        let mut node = PlanNode::new(
            PlanNodeType::Join,
            &format!("Join({:?}, {})", join_type, condition),
        );
        node.estimated_rows = estimated_rows;
        node.children.push(left);
        node.children.push(right);
        node.metadata.push(("condition".to_string(), condition.to_string()));
        node
    }

    /// Create a project node.
    pub fn project(columns: &[&str], child: PlanNode) -> Self {
        let mut node = PlanNode::new(PlanNodeType::Project, &format!("Project({:?})", columns));
        node.estimated_rows = child.estimated_rows;
        node.children.push(child);
        node
    }

    /// Create a sort node.
    pub fn sort(key: &str, child: PlanNode) -> Self {
        let mut node = PlanNode::new(PlanNodeType::Sort, &format!("Sort({})", key));
        node.estimated_rows = child.estimated_rows;
        node.children.push(child);
        node
    }

    /// Create a limit node.
    pub fn limit(count: usize, child: PlanNode) -> Self {
        let mut node = PlanNode::new(PlanNodeType::Limit, &format!("Limit({})", count));
        node.estimated_rows = child.estimated_rows.min(count as f64);
        node.children.push(child);
        node
    }

    /// Create an aggregate node.
    pub fn aggregate(func: &str, child: PlanNode) -> Self {
        let mut node = PlanNode::new(PlanNodeType::Aggregate, &format!("Agg({})", func));
        node.estimated_rows = 1.0;
        node.children.push(child);
        node
    }

    /// Get the total number of nodes in the plan tree.
    pub fn node_count(&self) -> usize {
        1 + self.children.iter().map(|c| c.node_count()).sum::<usize>()
    }

    /// Get the depth of the plan tree.
    pub fn depth(&self) -> usize {
        1 + self
            .children
            .iter()
            .map(|c| c.depth())
            .max()
            .unwrap_or(0)
    }

    /// Pretty-print the plan tree.
    pub fn explain(&self) -> String {
        let mut result = String::new();
        self.explain_into(&mut result, 0);
        result
    }

    fn explain_into(&self, result: &mut String, indent: usize) {
        for _ in 0..indent {
            result.push_str("  ");
        }
        result.push_str(&format!(
            "-> {} (rows={:.0}, cost={:.2})\n",
            self.label, self.estimated_rows, self.estimated_cost
        ));
        for child in &self.children {
            child.explain_into(result, indent + 1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scan_node() {
        let node = PlanNode::scan("users", 1000.0);
        assert_eq!(node.node_type, PlanNodeType::Scan);
        assert_eq!(node.estimated_rows, 1000.0);
    }

    #[test]
    fn test_filter_node() {
        let scan = PlanNode::scan("t", 100.0);
        let filter = PlanNode::filter("age > 30", 0.5, scan);
        assert_eq!(filter.estimated_rows, 50.0);
        assert_eq!(filter.children.len(), 1);
    }

    #[test]
    fn test_join_node() {
        let left = PlanNode::scan("a", 100.0);
        let right = PlanNode::scan("b", 200.0);
        let join = PlanNode::join(JoinType::Inner, "a.id = b.id", left, right);
        assert_eq!(join.children.len(), 2);
    }

    #[test]
    fn test_node_count() {
        let plan = PlanNode::project(
            &["name"],
            PlanNode::filter(
                "active",
                0.5,
                PlanNode::scan("users", 1000.0),
            ),
        );
        assert_eq!(plan.node_count(), 3);
    }

    #[test]
    fn test_depth() {
        let plan = PlanNode::limit(
            10,
            PlanNode::sort(
                "id",
                PlanNode::scan("t", 100.0),
            ),
        );
        assert_eq!(plan.depth(), 3);
    }

    #[test]
    fn test_explain() {
        let plan = PlanNode::filter("x > 0", 0.5, PlanNode::scan("t", 100.0));
        let explain = plan.explain();
        assert!(explain.contains("Filter"));
        assert!(explain.contains("Scan"));
    }

    #[test]
    fn test_aggregate() {
        let plan = PlanNode::aggregate("COUNT(*)", PlanNode::scan("t", 100.0));
        assert_eq!(plan.estimated_rows, 1.0);
    }
}
