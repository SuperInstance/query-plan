# query-plan

Query plan representation with cost estimation and join ordering in pure Rust.

## Features

- Tree-structured query plans (scan, filter, join, project)
- Cost estimation model (I/O, CPU, cardinality)
- Join ordering via dynamic programming
- Plan enumeration and comparison
- Heuristic optimization rules
- Zero external dependencies

## Usage

```rust
use query_plan::{PlanBuilder, CostEstimator};

let plan = PlanBuilder::new()
    .scan("users", 1000)
    .filter("age > 30", 0.5)
    .build();
let cost = CostEstimator::estimate(&plan);
```

## Modules

- `plan` — Plan node types and structure
- `cost` — Cost estimation model
- `join` — Join ordering algorithms
- `enumerate` — Plan enumeration
- `optimize` — Heuristic optimization rules
