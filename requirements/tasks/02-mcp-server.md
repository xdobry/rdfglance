The is mcp server first implementation in `crates\ui-desktop-mcp\src\mcp.rs`.

The should be 2 variants of application executable. 
The origin desktop and the mcp-version that opens mcp as stdio. 

It should have first the same ui. So ai agent can access the data statistics (stuct: `TypeInstanceIndex`).

My first plan is
* MCP should describe own purpose in instruction. The instructions should be separate file.
  The idea is that the app do not just offer rdf like sparql interface.
  * So the app offers the type-instance-index as resource
  * AI Agent can understand from rdf data metainformation something about nature the data (types, names, attributes, relations)
  * AI Agent can run tools on application like
     * show instances of some type (possibly filtered) in tables
     * show some instance in visual graph (for example based on table)
     * modify the visual graph
     * run queries
     * run statistics on visual graph nodes
* As first step on the tool (show instances of type with attribute filtering should be offered)
* Questions to clearify
   * What is the best way to offer agent metainformation (should it be resource, or tool that get it)
   * How implement some kind of feedback (callback) from ui to agent. For example data are loaded (or changed).
   * The mcp server should run in seprate thread (see existing implementaiton) so probably I need to clone TypeInstanceIndex to be returned by tool (or as resource). Can I reuse the type or better make own json supported type and init it from internal type

## Open points and decisions

1. **Dataset metainformation**
   - Expose a stable MCP resource named `dataset://current/profile`.
   - The resource will be a new serializable DTO, not `TypeInstanceIndex` directly.
   - Include dataset counts, type names/counts, properties, reference types, and unresolved/reference counts.
   - Add a `get_dataset_profile` tool only if resource support proves inadequate.

2. **Dataset change feedback**
   - MCP notifications cannot guarantee that an external chat host starts an assistant turn.
   - Add an internal `DatasetLoaded`/`DatasetChanged` application event for automatic handling by an embedded chat.
   - Send `notifications/resources/updated` for `dataset://current/profile` when supported and subscribed.
   - MCP remains an external interface; it should not control chat turn initiation.

3. **MCP/UI thread boundary**
   - The MCP server runs in a Tokio thread.
   - `RdfGlanceApp` remains owned and accessed exclusively on the egui/UI thread.
   - MCP handlers send `UiCommand` messages and optionally wait through `oneshot::Sender`.
   - Do not move `Arc<RwLock<RdfData>>` or `TypeInstanceIndex` into MCP tasks.
   - Return serializable snapshot DTOs, never internal numeric indexes.

## First functionality: show instances of a type with attribute filtering

### Tool contract

Create `show_type_instances` with:

- `type_iri`: exact RDF type IRI or supported display name
- `filters`: optional property/predicate IRI and literal/reference value pairs
- `limit`: positive number with a default
- `offset`: non-negative number
- `update_ui`: show the matching rows in the RDFGlance table

The tool should return:

- resolved type IRI and display label
- total match count
- returned count
- instance IRIs
- display labels
- selected property values
- warnings for unknown types, predicates, or unsupported filters

Use stable RDF IRIs in MCP responses rather than `IriIndex` values.

### UI command

Add a command such as:

```rust
UiCommand::ShowTypeInstances {
    type_name: String,
    filters: Vec<...>,
    limit: usize,
    offset: usize,
    reply: oneshot::Sender<Result<InstanceQueryResult, String>>,
}


    
