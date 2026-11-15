## Recommended direction

Keep **egui as the UI**, not as a layer to abstract. First make the existing modules depend in one direction; only extract Cargo crates once those boundaries are stable. The RDF model and analytics are good candidates for headless modules, but the current `domain` and `graph_algorithms` modules are not yet egui-independent.

For example, `graph_model.rs` imports `ValueTypes` from `type_index`, which also contains UI-oriented state. `rdf_data.rs` imports the RDF-loading adapter and layout state. `type_index.rs` and `graph_styles.rs` use egui types. Some graph algorithms also depend on UI layout or the app itself, such as `degree.rs` and `find_connections.rs`.

A reasonable target structure is:

```text
src/
  main.rs                 # Minimal target-specific dispatch
  platform/
    desktop.rs            # Native window, file paths, desktop startup
    wasm.rs               # Browser canvas, URL startup, browser APIs
  app/
    commands.rs           # Requests to load, query, analyze, navigate
    events.rs             # DatasetLoaded, DatasetChanged, AnalysisCompleted
    services.rs           # Use cases shared by UI and MCP
    session.rs            # Application/session coordination
  rdf/
    model.rs              # Nodes, literals, IDs, prefixes
    store.rs              # In-memory dataset and indexes
    import/                # RDF, CSV, JSON, XML adapters
    project_io.rs          # Dataset/project serialization boundary
  analytics/
    profile.rs            # Dataset overview, types, attributes, relationships
    query.rs               # Reusable query/filter/aggregation operations
    graph/                 # Centrality, clustering, other graph analyses
  mcp/
    server.rs              # MCP lifecycle and transport
    tools.rs               # Tool definitions and handlers
    resources.rs           # Dataset and analysis resources
  ui/                      # egui screens and widgets
  graph_view/              # egui-specific graph layout/rendering
```

Names and exact grouping can vary. The important dependency direction is:

```text
platform ──> app ──> rdf
               ├──> analytics ──> rdf
               ├──> mcp ────────> app/analytics
               └──> ui ─────────> app/analytics/rdf
```

The core should not depend on `egui`, `eframe`, `ui`, or `uistate`. Keep graph *visual layout* in the egui-facing side; it is different from dataset analysis. Also separate `TypeInstanceIndex`’s reusable type/property statistics from its table view, sorting state, and column sizing.

## Suggested split, in stages

1. **Establish boundaries in the existing crate.** Move files and adjust module paths without changing behavior. Start with a low-risk split of `main.rs` into platform startup modules. Keep `src/main.rs` as a small `#[cfg(target_arch = "wasm32")]` dispatcher to the desktop or browser startup code; both should still construct the same `RdfGlanceApp` from `app.rs`.

2. **Make the RDF model headless.** Separate the node/literal store, interners, prefixes, and parsing adapters from graph-view actions. In particular, move or redesign `NodeChangeContext` and expansion operations so RDF storage does not need layout types. Keep `NodeData`’s index-based references and insertion ordering intact: current nodes refer to one another by cache index.

3. **Extract analytics interfaces.** Move reusable analysis and dataset profiling behind functions or services that accept RDF data and return plain results. Keep egui-specific table state, `Pos2`, widths, and selections in UI state. Algorithms that currently take UI `Edge` or mutate `RdfGlanceApp` will need a small, UI-independent graph input/output type.

4. **Introduce shared application use cases.** Add a small command/event boundary for operations both clients might request—for example, load a dataset, get its profile, query entities, or run an analysis. Emit coarse application events such as `DatasetLoaded` or `DatasetChanged`; keep MCP notifications separate from those internal events. There is currently no general app-wide command/event bus, so introduce only the pieces needed by these use cases rather than trying to generalize layout worker channels.

5. **Add MCP as an adapter.** Keep MCP protocol and transport code in `mcp`; make tool handlers call the same application/analytics services as the egui UI. Return serializable RDF identifiers and result types, not internal node indexes or egui types. The existing MCP research proposes starting with dataset overview/type/relationship resources and a few analytical tools; see `mcp-design.md`. If handlers are asynchronous, avoid holding the app’s `RwLock` across an `await`.

6. **Extract crates only when useful.** Once the RDF and analytics boundaries no longer import UI/app code, they can become workspace crates (for example, `rdf-core` and `rdf-analytics`). Keep MCP native-only if its chosen transport requires desktop process capabilities. This lets WASM continue using the same core without pulling in desktop-only MCP dependencies.

## Existing seams and caveats

- The desktop and WASM startup branches already live in `main.rs`, while target-specific loading is also handled inside `app.rs`. Move those responsibilities gradually rather than duplicating app initialization.
- `persistency.rs` serializes more than the RDF dataset, including visual and UI state. Treat separating dataset persistence from whole-project persistence as its own boundary change.
- The existing `mcp-workflow.md` distinguishes internal application events from MCP notifications. Preserve that distinction: MCP is an external interface, not the app’s event system.

I recommend starting with **module separation inside the current crate**, then extracting the RDF and analytics modules into workspace crates after their dependency boundaries are clear. This avoids turning a structural refactor into a rewrite and keeps both platform builds testable at each step.