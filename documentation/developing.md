# Test Coverage


Install it once:

```powershell
rustup component add llvm-tools-preview
cargo install cargo-llvm-cov
```

Then you have two ways to run tests:

```powershell
# Ordinary tests, without coverage
cargo test --workspace

# Tests with an HTML coverage report
cargo llvm-cov --workspace --all-targets -j 2 --html --open
```

The HTML report highlights uncovered code; the function coverage summaries help identify functions tests never reached. For a text summary instead, run:

```powershell
cargo llvm-cov --workspace --all-targets
```

# Visual Queries

Query definitions, filtering, execution, sorting, and aggregation live in
`rdf-analytics::domain::visual_query` and operate on `rdf_core::NodeData`.
Call `refresh_table_data` after changing the query tree before executing it.
The UI's `VisualQueryView` owns presentation state keyed by stable `TableQueryId`;
allocate a new ID and view for each table created by the UI and synchronize the
view map after removing tables. CSV export uses the UI's visible columns.
