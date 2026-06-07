//! Query plan representation with cost estimation, join ordering, and plan enumeration.

pub mod cost;
pub mod enumerate;
pub mod join;
pub mod optimize;
pub mod plan;

/// Convenience builder for query plans.
pub struct PlanBuilder {
    tables: Vec<plan::PlanNode>,
    filters: Vec<(String, f64)>,
    projections: Vec<String>,
    limit: Option<usize>,
    sort_key: Option<String>,
}

impl PlanBuilder {
    /// Create a new plan builder.
    pub fn new() -> Self {
        PlanBuilder {
            tables: Vec::new(),
            filters: Vec::new(),
            projections: Vec::new(),
            limit: None,
            sort_key: None,
        }
    }

    /// Add a scan of a table.
    pub fn scan(mut self, table: &str, estimated_rows: f64) -> Self {
        self.tables.push(plan::PlanNode::scan(table, estimated_rows));
        self
    }

    /// Add a filter.
    pub fn filter(mut self, predicate: &str, selectivity: f64) -> Self {
        self.filters.push((predicate.to_string(), selectivity));
        self
    }

    /// Add a projection.
    pub fn project(mut self, columns: &[&str]) -> Self {
        self.projections = columns.iter().map(|s| s.to_string()).collect();
        self
    }

    /// Add a limit.
    pub fn limit(mut self, count: usize) -> Self {
        self.limit = Some(count);
        self
    }

    /// Add a sort.
    pub fn sort(mut self, key: &str) -> Self {
        self.sort_key = Some(key.to_string());
        self
    }

    /// Build the plan.
    pub fn build(self) -> plan::PlanNode {
        let mut plan = if self.tables.len() == 1 {
            self.tables.into_iter().next().unwrap()
        } else if self.tables.len() > 1 {
            // Chain joins
            let mut iter = self.tables.into_iter();
            let mut plan = iter.next().unwrap();
            for right in iter {
                plan = plan::PlanNode::join(plan::JoinType::Inner, "id", plan, right);
            }
            plan
        } else {
            plan::PlanNode::scan("empty", 0.0)
        };

        // Apply filters
        for (predicate, selectivity) in &self.filters {
            plan = plan::PlanNode::filter(predicate, *selectivity, plan);
        }

        // Apply sort
        if let Some(ref key) = self.sort_key {
            plan = plan::PlanNode::sort(key, plan);
        }

        // Apply projection
        if !self.projections.is_empty() {
            let cols: Vec<&str> = self.projections.iter().map(|s| s.as_str()).collect();
            plan = plan::PlanNode::project(&cols, plan);
        }

        // Apply limit
        if let Some(count) = self.limit {
            plan = plan::PlanNode::limit(count, plan);
        }

        plan
    }
}

impl Default for PlanBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plan_builder_scan() {
        let plan = PlanBuilder::new().scan("users", 1000.0).build();
        assert_eq!(plan.node_type, plan::PlanNodeType::Scan);
    }

    #[test]
    fn test_plan_builder_full_query() {
        let plan = PlanBuilder::new()
            .scan("users", 1000.0)
            .filter("active", 0.5)
            .sort("name")
            .limit(10)
            .build();
        assert!(plan.depth() >= 3);
        assert_eq!(plan.estimated_rows, 10.0);
    }

    #[test]
    fn test_plan_builder_join() {
        let plan = PlanBuilder::new()
            .scan("users", 100.0)
            .scan("orders", 500.0)
            .build();
        assert_eq!(plan.node_type, plan::PlanNodeType::Join);
    }
}
