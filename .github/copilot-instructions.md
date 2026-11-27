# RDFGlance project instructions

## Build, test, and lint

- Native release build: `cargo build --release`
- Development run: `cargo run`
- Check: `cargo check`
- Full test suite: `cargo test`
- Run one unit test by name: `cargo test --lib <test_name>`
- Run the memory usage integration test: `cargo test --test memory_usage -- --nocapture`
- Build the desktop MCP server: `cargo build --package rdf-glance-desktop-mcp`
- Run the Python MCP stdio integration test (after building the server): `python tests/mcp-test.py`
- Override the MCP test executable or dataset with `--executable <path>` or `--dataset <path>`; adjust its wait limit with `--timeout <seconds>`.
- Formatting check (uses `rustfmt.toml`): `cargo fmt --check`
- Optional Clippy lint: `cargo clippy --all-targets` (not currently part of CI)
- WASM compile: `rustup target add wasm32-unknown-unknown`, then `cargo build --target wasm32-unknown-unknown`
- WASM site build: `trunk build --release --public-url /rdfglance/`

CI runs the native build and full tests, then compiles the WASM target; see `.github/workflows/rust.yml`. The user-facing build and run details are in `Readme.md`.

## Architecture

- `src/main.rs` starts the same `RdfGlanceApp` in either an eframe native window or browser canvas. `src/uistate/app.rs` coordinates application state, loading, navigation, and screen selection.
- Import adapters in `src/integration/` parse RDF and convert CSV, JSON, or XML into the shared domain graph. `RdfData` holds that graph and its prefixes; `NodeData` stores typed nodes, properties, and resolved forward/reverse references.
- `src/domain/` owns the RDF model, indexing, statistics, graph styles, persistence data, and visual-query logic. `TypeInstanceIndex` and other derived views are built from `NodeData`. Graph algorithms and layout algorithms operate on this model; `src/ui/` renders it with egui, while `src/uistate/` holds interaction and layout state.
- `src/integration/persistency.rs` reads and writes the application's versioned binary project format. `src/domain/app_persistence.rs` handles persisted application preferences.
- `crates/ui-desktop-mcp/` provides the `rdf-glance-mcp` desktop server over stdio and Streamable HTTP. `tests/mcp-test.py` launches the stdio server as a subprocess, loads the programming-languages sample dataset, and checks dataset, node, search, type-instance, and working-graph tools; it uses only the Python standard library.

## Repository-specific conventions

- The graph is type-oriented: nodes and table/statistics views are organized around RDF types. Named graphs are ignored, as described in `Readme.md`.
- RDF IRIs are stored with prefixes such as `dbr:`, `dbo:`, and `rdfs:`; preserve this form in data and MCP results rather than expanding IRIs.
- Node, predicate, type, language, and short-literal IDs are numeric indexes into interners or the insertion-ordered node cache. Long literal values use spans into a shared string cache. Preserve these indexes when changing storage or iteration; removing/reordering cached nodes can invalidate references.
- When changing `Literal` or another serialized model, update all consumers (parsing, labels/property selection, tables/queries/statistics) and the matching binary serialization/deserialization in `src/integration/persistency.rs`.
- `TypeInstanceIndex` is derived from `NodeData`; after changing graph data, follow the existing index-refresh path rather than treating those statistics as live.
- Native and browser paths share most code but use `#[cfg(target_arch = "wasm32")]` for platform-specific loading and UI. Keep both target builds in mind; native parallel processing does not provide multithreading in WASM.
- Most Rust tests are colocated with the implementation in `#[cfg(test)]` modules. `tests/memory_usage.rs` is a separate integration test and reads the checked-in sample RDF data.
