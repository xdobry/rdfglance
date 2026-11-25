# Stage 1

Introduce cargo workspace structure with mulitple crates.

- create crates in followin structure (proposal)

- rdf core
- rdf analytics
- ui
- ui-desktop (main start)
- ui-wasm (main start)


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

6. **Extract crates only when useful.** Once the RDF and analytics boundaries no longer import UI/app code, they can become workspace crates (for example, `rdf-core` and `rdf-analytics`). Keep MCP native-only if its chosen transport requires desktop process capabilities. This lets WASM continue using

# Stage 2

The key point is that `crates/ui` is currently more like the **application crate** than a widget-only crate: it contains the egui screens, app state, import/export, persistence, layout algorithms, and some analysis/query logic. The desktop and WASM crates are thin launchers around it. So it’s reasonable to slim it, but “doesn’t use egui” isn’t by itself a good extraction boundary.

## Most promising extractions

| Candidate | Destination | Effort | Why / caveat |
|---|---|---:|---|
| **Format parsers** | New `rdf-import` crate | Medium | CSV, JSON, and XML parsing look like a cohesive unit and account for several direct dependencies in `crates/ui/Cargo.toml`. Keep the adapter that updates app graph state in the UI crate initially. The current `json2rdf.rs` even uses `egui::TextBuffer`; that’s an incidental UI tie to remove before extracting it. |
| **Graph statistics and reusable analysis** | Extend `rdf-analytics` | Medium | `statistics.rs` and pure calculations from `type_index.rs` are candidates. But `TypeInstanceIndex` also holds selection, filtering, and table-view state, so move the calculation/data parts—not the entire type. |
| **Visual-query semantics and execution** | Possibly new `rdf-query` crate, or `rdf-analytics` | Medium–high | Query filters, aggregations, and execution can be independent of egui. The current `visual_query.rs` also stores editor state such as table position, so separate the query representation/execution from its on-screen editor state. |
| **Graph layout algorithms** | New `rdf-layout` crate | High | These aren’t rendering, but they consume egui geometry and UI-owned `SortedNodeLayout` state; see `layoutalg/mod.rs` and `layout.rs`. A clean extraction needs a layout input/output API—nodes, edges, sizes in; positions/routes out—with egui conversions kept in UI. |
| **Connection-finding algorithm** | Extend `rdf-analytics` | Low–medium | `find_connections.rs` may be a small first move if it can accept core graph data without app/UI state. `rdf-analytics` already exists for this kind of functionality. |

## Keep in the UI/application layer

Rendering and interaction code—screens, menus, pointer/selection state, egui styling and colors—belongs there. The same applies to persistence that saves the whole UI session: `persistency.rs` serializes `UIState`, layout state, visualization styles, and egui geometry, so it’s not just RDF persistence.

Likewise, RDF label formatting is already partly in core: `IriDisplay`, `LabelContext`, and label/display methods are in `rdf-core`. That’s a good model for sharing *domain* behavior without moving UI presentation into core.

## A practical order

1. Extract pure functions and data from `TypeInstanceIndex`/statistics into the existing `rdf-analytics` crate.
2. Extract CSV/JSON/XML parser code into `rdf-import`, leaving app-specific graph updates in `crates/ui`.
3. Split visual-query execution from visual-query editor state.
4. Consider layout extraction only if you want a reusable layout API; it requires the most untangling.

This would reduce `crates/ui`’s responsibilities and direct dependency list. It **wouldn’t necessarily reduce the final app’s total dependencies or binary size**, because the app still uses those extracted crates. Its main payoff is clearer boundaries and making non-UI functionality reusable and independently testable.