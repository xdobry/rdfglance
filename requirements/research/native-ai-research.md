# **Making your RDF viewer an AI-native application**

I think this is a very promising direction, particularly because an RDF viewer has something that many AI applications lack: **a structured, explorable knowledge model combined with a visual representation of the data.**

I would not simply add a chatbot to your existing application. I would consider evolving it into an **AI-assisted knowledge exploration environment**, where the user can move naturally between:

- Visual exploration of RDF graphs.
- Natural-language questions about the data.
- AI-generated analyses and visualizations.
- Interactive investigation, where the AI and user manipulate the same graph.
- Reusable analytical workflows.

My recommendation would be to keep the existing RDF analysis and visualization engine, but introduce an AI interaction layer that can control it through well-defined commands.

There are several architectural approaches. They are not mutually exclusive.

## **1. The most important architectural decision: AI as a controller, not just a chatbot**

Imagine a user opening a large RDF dataset and asking:

> Find the most important entities in this dataset and show me how they are connected.

A conventional chatbot might return a textual explanation or a SPARQL query.

An AI-native RDF viewer should instead:

1. Understand the request.
2. Determine which analysis is appropriate.
3. Execute graph algorithms.
4. Modify the current visualization.
5. Explain the result.
6. Allow the user to continue investigating.

For example:

**RDF exploration**

Example interaction

USER

Find the most central entities and highlight their connections.

**AI assistant**

I will calculate PageRank and betweenness centrality, then highlight the most important nodes.

**Graph analysis**

Completed

# **42**

Important entities

# **3**

Communities detected

Visualization updated: important entities highlighted, remaining nodes faded.

USER

Why is the second entity important?

The entity connects two otherwise weakly connected communities. Its betweenness centrality is particularly high.

The important distinction is that the chat is not an independent component. It is an alternative interface to the application's functionality.

I would build the architecture around this idea.

## **2. Three possible architectural approaches**

[What is an MCP server?](https://images.openai.com/static-rsc-4/_FiEi9AgJvQe_hgouFkJtq_uGxUFzXGKceYTAAFDkB4THy9HnijbluTLhvzbWRpem9ntFlPyImlVD1o3Tir2cbBHXYBlhgAMzRIPaUN2m_gtxVI-iLtCGEQDOwjapxpV7j7_NufvFXTICvSONmd6bXs2SMO-tKfZhXOK9JfTyZ4?purpose=inline)

**A. MCP server for the RDF viewer**

Expose your RDF analysis functionality as tools that external AI clients can invoke.

Examples: load dataset, query graph, calculate centrality, find communities, select nodes, change visualization.

Good first step

[Getting started | EnconvoAI](https://images.openai.com/static-rsc-4/JpokWADXkjzKjMahi3Zjt_fZFJ1mIBi2ETIILu4O4i4uTTxCGC3TThLt3bRBOR8KWgCtKWAuXztAR-zp3MDFuNhkdM0GpGL2oH9CPZb7C7tP0EZ8oWztSYHT9BEwvwbIJ1p7xlkBzEuwacKH81jCwKBg5eCH1uPMv02ks9HUK5I?purpose=inline)

**B. Embedded AI assistant**

Integrate an LLM directly into the egui application. The AI calls your application functionality through an internal tool interface.

You control the entire interaction, including visualization updates and user feedback.

Best for native AI UX

[ThoughtSpot Agentic Analytics Platform](https://images.openai.com/static-rsc-4/1tWvSXlzumsgFb8tbt8J25u5NeKEy0S_u2s9ddlzHTZLPw3hoLOolhIzbFFQon-ZEUoDx6SAvcVv_7fNy2_R-mB63_InUk9vTIdUGhrV5i2WOcn0cf8Yg0OHaCr7ci0uPrbAUIWh0pWQlnAztwrlxhiEoScNUj0OUCoJrl_qUtY?purpose=inline)

**C. Hybrid architecture**

Build a common application command/API layer. The embedded assistant and external MCP server both use it.

This also gives you a natural way to support desktop and browser versions.

My recommendation

The hybrid architecture avoids implementing the same functionality twice.

## **3. MCP integration: useful, but it does not solve the visualization problem by itself**

MCP is a good way to make your RDF viewer usable by external AI applications.

The important thing is to distinguish three MCP concepts:

| MCP capability | Possible RDF viewer implementation                          |
| -------------- | ----------------------------------------------------------- |
| Tools          | Run SPARQL, calculate PageRank, find communities            |
| Resources      | Expose dataset metadata, selected entities, graph summaries |
| Prompts        | Provide predefined workflows such as "Analyze this dataset" |

The official Rust implementation is [rmcp](https://github.com/modelcontextprotocol/rust-sdk?utm_source=chatgpt.com).

It supports building both MCP clients and servers. There are also macros for defining tools.

### **What I would expose**

I would avoid exposing every internal function as an individual tool. Instead, design tools around meaningful user tasks.

For example:

**Analysis tools**

- `analyze_graph`
- `find_important_entities`
- `detect_communities`
- `find_shortest_path`
- `compare_subgraphs`

**Data tools**

- `get_dataset_summary`
- `execute_sparql`
- `get_entity_details`
- `search_entities`
- `get_entity_relationships`

**Visualization tools**

- `show_entities`
- `highlight_entities`
- `expand_neighborhood`
- `apply_layout`
- `set_visualization_filter`
- `focus_subgraph`

The visualization tools are particularly interesting.

### **How to integrate visualization with an external chat**

There are three possible approaches.

**Approach 1: MCP tools change the application state**

The external AI invokes:

```
{
  "tool": "highlight_entities",
  "arguments": {
    "entities": ["ex:Person1", "ex:Person2"]
  }
}
```

Your RDF viewer receives the command and updates its visualization.

This requires the viewer to be running and connected to the MCP server. The chat itself does not need to render the graph.

**External AI chat**

MCP

**RDF viewer MCP server**

**Application command bus**

**Graph model**

**Visualization**

This is probably the simplest integration.

**Approach 2: Return visualization descriptions**

An MCP tool returns structured information describing a visualization.

For example:

```
{
  "type": "graph",
  "dataset": "current",
  "selected_nodes": ["ex:A", "ex:B"],
  "highlighted_edges": ["ex:relatedTo"],
  "layout": "force_directed"
}
```

The external application could render this if it understands your visualization format.

The disadvantage is that most generic chat clients will not understand your custom graph visualization.

**Approach 3: MCP Apps**

This is particularly interesting.

The MCP ecosystem has been developing a way to provide interactive UI components inside compatible host applications. The relevant extension is called MCP Apps.

It allows an MCP server to provide UI resources, potentially including interactive charts, forms and other components.

See the [MCP specification](https://github.com/modelcontextprotocol/modelcontextprotocol/blob/main/docs/specification/2026-07-28/index.mdx?utm_source=chatgpt.com) and its UI-related extensions.

However, there is an important distinction:

- Your native egui visualization cannot simply be embedded into every chat.
- You would need a web-based visualization component, or a separate visualization representation.
- The external chat must support the relevant UI extension.

I would treat MCP Apps as an additional integration opportunity, not as the foundation of your own application UI.

## **4. Embedded chat: Rust frameworks and existing projects**

You don't necessarily need a large agent framework.

I would separate three concerns:

1. LLM communication.
2. Agent/tool orchestration.
3. egui presentation.

### **Rust LLM libraries worth investigating**

**1. Rig**

My first choice to investigate

[GitHub: 0xPlaygrounds/rig](https://github.com/0xPlaygrounds/rig?utm_source=chatgpt.com)

A Rust framework for LLM applications. It provides:

- Multiple LLM providers.
- Agents and tool calling.
- Structured output.
- Embeddings and vector stores.
- Streaming responses.

It is a good candidate for implementing your own assistant without having to implement all the provider integrations yourself.&#x20;

[image](https://www.google.com/s2/favicons?domain=https://ml-rust.github.io\&sz=32)

ML Rust catalogue



**2. rmcp**

[GitHub: modelcontextprotocol/rust-sdk](https://github.com/modelcontextprotocol/rust-sdk?utm_source=chatgpt.com)

This is the official MCP implementation. Useful if you want your application to be an MCP server, client, or both.

**3. chat-rs**

[GitHub: EggerMarc/chat-rs](https://github.com/EggerMarc/chat-rs?utm_source=chatgpt.com)

Interesting because it explicitly supports:

- Multiple providers.
- Tool calling.
- Structured output.
- Streaming.
- Human approval during tool execution.

This is worth comparing with Rig.

**4. LangChainRust**

[Docs and source information](https://docs.rs/crate/langchainrust/latest?utm_source=chatgpt.com)

A more comprehensive framework with agents, RAG, retrieval, workflows and MCP support.

Potentially useful for larger agent workflows, but possibly more than you need for a small RDF viewer.

My preference would be to start with Rig, or even a thin custom implementation using an LLM API client and typed tools.

You do not need an autonomous multi-agent framework just to provide natural-language graph analysis.

### **What about an existing egui chat implementation?**

There are two different things here:

- An egui chat interface.
- An LLM agent implementation.

I would not tightly couple them.

Your egui application should only receive messages and events such as:

```
enum AiEvent {
    TextDelta(String),
    ToolStarted(ToolInfo),
    ToolFinished(ToolResult),
    ApprovalRequired(ToolCall),
    Completed,
    Error(String),
}
```

And send messages such as:

```
enum AiCommand {
    SendMessage(String),
    ApproveToolCall(String),
    RejectToolCall(String),
    Cancel,
}
```

This fits well with your existing asynchronous architecture: the LLM runs in a background task, communicates through channels, and the egui thread updates its state.

You can render the chat yourself with ordinary egui widgets.

That gives you control over the UI without committing to a large framework.

I would also investigate the existing [egui project](https://github.com/emilk/egui?utm_source=chatgpt.com) examples and community extensions, but I would not make the choice of a chat widget determine your agent architecture.

## **5. A new AI-native UX: beyond the traditional chatbot**

This is probably the most interesting part of your question.

I would not design the application as:

**Graph viewer**

**Chat window**

**A large collection of menus and tools**

Three largely independent interaction systems.

Instead, I would consider a concept I call **conversational visual analytics**.

The user interacts with the application through three connected channels:

1. Conversation.
2. Visualization.
3. Contextual actions.

The user should be able to move between them without losing context.

### **A. Context-aware chat**

The chat should understand the current application state.

For example, its context might include:

```
struct AiContext {
    dataset_id: DatasetId,
    selected_entities: Vec<EntityId>,
    visible_entities: Vec<EntityId>,
    active_analysis: Option<AnalysisId>,
    active_visualization: VisualizationState,
}
```

Now consider:

- User selects three nodes.
- User asks: "What do they have in common?"
- The assistant understands that "they" refers to the selected nodes.
- It performs an analysis.
- It highlights the relevant relationships.

No need to explain which entities the user means.

This is an important principle: **the application state becomes part of the conversational context.**

### **B. Contextual actions instead of menus**

Suppose the user selects a node.

Instead of presenting a large context menu with 20 actions, you could show a small contextual suggestion area:

**Selected entity**

1 entity

## **ex:Researcher42**

[http://example.org/researcher/42](http://example.org/researcher/42?utm_source=chatgpt.com)

**Suggested actions**

**Explore neighbors**

**Find connections**

**Analyze importance**

**Ask AI**

Illustrative contextual action panel.

The AI can dynamically suggest actions based on the selected object, current analysis and user intent.

The suggestions should be optional. They should not replace predictable, familiar controls for frequently used operations.

### **C. AI-generated UI elements**

Another possibility is to let the AI generate small interactive result components.

For example, the user asks:

> Compare the three largest communities.

The assistant could produce a comparison table, a chart, and clickable community entries.

The result becomes part of the application's UI rather than just text in a conversation.

For your application, I would define a limited set of reusable result types:

| Result type     | Purpose                                    |
| --------------- | ------------------------------------------ |
| Text            | Explanation and interpretation             |
| Entity list     | List of RDF resources                      |
| Property table  | Compare RDF properties                     |
| Graph selection | Highlight a subgraph                       |
| Chart           | Statistical comparison                     |
| Analysis card   | Algorithm and results                      |
| Query editor    | Editable SPARQL                            |
| Action proposal | Suggested operation requiring confirmation |

The AI returns a structured description, and your application renders it using native egui components.

This is much safer and more predictable than allowing the LLM to generate arbitrary UI code.

## **6. Are there standards for AI-native UX?**

There is no universally accepted standard for designing AI-native desktop applications yet. But several emerging concepts and technologies are relevant.

I would distinguish three levels.

### **A. MCP Apps: a technical standard for conversational UI**

As mentioned above, MCP Apps is particularly relevant.

It defines how an MCP server can deliver interactive UI components into a compatible chat host.

[MCP Apps documentation](https://apps.extensions.modelcontextprotocol.io/api/?utm_source=chatgpt.com) · [Official GitHub repository](https://github.com/modelcontextprotocol/ext-apps?utm_source=chatgpt.com)

This is a real standardization effort rather than just a design concept.

It is useful for:

- Interactive visualizations.
- Result tables.
- Forms.
- Dashboards.
- Multi-step workflows.

It does not define how your own egui application should look, however.

### **B. Conversational Visual Analytics**

This is probably the most relevant research field for your application.

It combines:

- Visual analytics.
- Natural-language interaction.
- AI-assisted exploration.
- Human-in-the-loop analysis.

A particularly interesting recent survey is:

[State of the Art of LLM-Enabled Interaction with Visualization (2026)](https://arxiv.org/abs/2601.14943?utm_source=chatgpt.com).

It examines 48 research papers and identifies different roles of LLMs in the visualization pipeline:

1. Data querying and transformation.
2. Visualization generation.
3. Visualization explanation.
4. Navigation and interaction.

This is almost exactly the functionality you are considering.

Another interesting project is [ProactiveVA](https://zyh1222.github.io/ProactiveVA/?utm_source=chatgpt.com) (IEEE VIS 2025).

It investigates an AI assistant that can recognize when users might need help during visual analysis and offer contextual assistance.

I find this particularly interesting for your application because the AI doesn't have to wait for an explicit question.

For example:

- The user has been exploring a particular community.
- The AI recognizes an unusual structural pattern.
- It suggests investigating a bridge node.
- The user can accept or ignore the suggestion.

This is called *mixed-initiative interaction*: both the human and the AI can initiate actions.

### **C. Generative UI**

Another emerging concept is Generative UI (GenUI).

Instead of generating only text, an LLM produces structured descriptions of UI components.

For example:

```
{
  "type": "analysis_result",
  "title": "Community analysis",
  "components": [
    {
      "type": "statistic",
      "label": "Communities",
      "value": 12
    },
    {
      "type": "entity_list",
      "items": ["ex:A", "ex:B", "ex:C"]
    },
    {
      "type": "action",
      "label": "Visualize communities",
      "command": "show_communities"
    }
  ]
}
```

Your application interprets this description and renders native components.

I would implement this using a closed set of typed UI components rather than arbitrary HTML or generated Rust code.

This is particularly suitable for egui because you can render all the components yourself.

**My recommendation:** Combine Conversational Visual Analytics and Generative UI. Use MCP Apps when integrating with external chat clients.

## **7. LLM connectivity: what resources can the application use?**

You have several possibilities. I would make the model provider interchangeable.

There are essentially five categories.

[Modeling Distributed Inference](https://images.openai.com/static-rsc-4/VojrQzA-Us4-ATTAq9RQ93FRspW75505Mjk2Q8QYd5brWbdTl1ruW1o45yZltp_3XRw4hdYv_csx3VmdGlpfUxVib2IfpBRzC5pN7KdbsHNNZR0cfHpVuD-V9eyXyAuIg-rmu7mZ9PKYRrcypb3F40DWTO78VwSwnwbjICAlJDw?purpose=inline)

**1. Cloud APIs**

Examples: OpenAI, Anthropic, Google Gemini.

Advantages:

- High-quality models.
- No local GPU requirements.
- Easy to support streaming and tool calling.

Disadvantages:

- Requires internet.
- Costs money.
- RDF data may be transmitted to an external provider.

[Part 1 - AI Lab - Understanding Ollama — Ian Myles](https://images.openai.com/static-rsc-4/f7XuvMxfvjhLA2lEA0MALf2tEZxQ1l9BK_Ogc4WUR9tqx-CQa9eqcGJvRoVApLusTw5zhgy4aWfkHywX9U24FYiPp1c1CSorNMMV9D4fg3Y1Qu0tbgv78x4jIDKihu6QslxkSKLHoIlSeKGYZGrAq8C7BPnBiafXUkgap292Pvk?purpose=inline)

**2. Local LLM server**

Examples: Ollama, llama.cpp.

Advantages:

- Data stays on the user's machine.
- Can work offline.
- No external API account required.

Disadvantages:

- Hardware requirements.
- Model quality varies.
- Installation and configuration can be complicated.

Particularly attractive for your native Rust application.

[vLLM, Ollama, LM Studio, llama.cpp: Choosing the best LLM inference engine in 2026 \[ Updated \]  | BIZON](https://images.openai.com/static-rsc-4/qtUaC-yayTTv__4fn-g3_VlEVF75Rych3qecY4k7bmFbcYYcDTJUal3rItUrDFj3v4mpafLfGSGPeuT2AUujlnKqG5J6n9OH2eaoQguE7gZ-XljqJLJb1gv2cZXxmWOXptcRBL_-WPl7PjsZp1AsaXOsH1TTlmi82YrD7t7rQmQ?purpose=inline)

**3. Embedded local inference**

The application loads a model itself.

Possibilities include:

- llama.cpp.
- ONNX Runtime.
- Candle.
- WebGPU-based inference in the browser.

This gives you more control, but increases application size, memory requirements and implementation complexity.

[How to Use OpenRouter for 400+ AI Models](https://images.openai.com/static-rsc-4/hH2GoJEUune9KLO7TC1oNqsPO7vfYHoEpGE7ddJTEhdihPE6fLeg0BqlmlzBL3Emp_QrpxnmRL4luckNwiKVrIk-0Fk8S3HmAnTsylA-O27QYL122DDA_EHYwlKg8731T8zhKxqJX-Mj8MSJQ1kyRJRvVEDtcB7T1htmUcegxdY?purpose=inline)

**4. OpenAI-compatible endpoints**

Many local and hosted services offer APIs compatible with OpenAI-style chat completions.

This is useful because you can support multiple providers with one interface.

Examples include local Ollama endpoints and compatible hosted inference services.

[Deploying AI Models in Air-Gapped Environments: A Practical Guide From the Data Center Trenches | by Siva Kiran Nandipati | Medium](https://images.openai.com/static-rsc-4/bUGAElG10UoAA8HtgdfdyBjMqs0IMa-7sh2yc9Wt3YGJd5jT7Y6BlxxLutVAIPmvCPiXHzIMVCvGJogK8XTRHYS3wKFySZ0sk1g2XujVrfuYFGeGbIt0u_zSbueanJMOuzLj8O-XgOxXzIBQlaqfxt6jiAzlTNMVkkV-tvbRAy0?purpose=inline)

**5. User-provided infrastructure**

A user or organization configures its own:

- LLM endpoint.
- API key.
- Model.
- Authentication method.

This could be a private company endpoint or a hosted service.

### **I would create a provider abstraction**

For example:

```
#[async_trait]
trait LlmProvider {
    async fn chat(
        &self,
        request: ChatRequest,
    ) -> Result<ChatResponse>;

    async fn stream(
        &self,
        request: ChatRequest,
    ) -> Result<ChatStream>;
}
```

The actual implementation could use Rig or a smaller custom provider layer.

A configuration might look like:

```
struct LlmConfig {
    provider: String,
    endpoint: Option<String>,
    model: String,
    api_key: Option<String>,
}
```

You could support several predefined providers and a generic OpenAI-compatible endpoint.

One important consideration: not every provider or model supports tool calling, structured output and streaming equally well. Your provider abstraction should expose capabilities rather than assume every feature is available.

## **8. Browser WASM: what changes?**

This is an important architectural constraint.

Your existing egui application can run as WASM, but the AI integration has different requirements.

I would think about native and browser environments separately.

| Capability                | Native Rust             | Browser WASM                                   |
| ------------------------- | ----------------------- | ---------------------------------------------- |
| egui visualization        | Yes                     | Yes                                            |
| RDF analysis              | Yes                     | Yes                                            |
| Local files               | Broad filesystem access | Browser file APIs and user-mediated access     |
| Background async requests | Yes                     | Yes, browser networking                        |
| MCP stdio server          | Yes                     | No                                             |
| MCP HTTP client           | Yes                     | Yes, subject to browser restrictions           |
| Local Ollama access       | Yes                     | Possible, subject to CORS/network restrictions |
| Local model inference     | Yes                     | Possible, but resource-dependent               |
| API keys                  | OS secure storage       | No truly secret client-side storage            |
| Local model files         | Filesystem              | Browser storage with limitations               |

The most important WASM restriction is that browser applications cannot freely access the operating system.

For example, you cannot simply start an MCP process with stdio transport from a browser application.

You would need an HTTP-based MCP server.

### **Suggested architecture for both versions**

**Shared RDF application core**

Graph model · SPARQL · Algorithms · Visualization state · Commands

**Native frontend**

egui desktop

Native AI adapter

Local / remote LLM

MCP stdio / HTTP

**WASM frontend**

egui browser

Browser AI adapter

Remote / local HTTP LLM

MCP HTTP

I would keep the application core independent of the LLM implementation.

For asynchronous execution, your existing command bus and channel-based approach can work well. You already have an architecture in which UI pages communicate with application functionality through commands, so this is a natural extension.

## **9. Security: can a WASM application access the user's LLM credentials?**

Yes, this needs careful consideration.

There are two different security problems.

### **A. Protecting API credentials**

Suppose the user enters an OpenAI API key into your browser application.

You store it in:

- Local storage.
- IndexedDB.
- WASM memory.
- A browser-side configuration file.

None of these is a secure secret store against the person using the browser or malicious code running in the application.

A user can inspect browser memory, network requests and application storage. WASM does not change this.

OpenAI explicitly advises against deploying API keys in browser applications. Anthropic provides similar guidance.&#x20;

[image](https://www.google.com/s2/favicons?domain=https://help.openai.com\&sz=32)

OpenAI Help Center+2



I see four possible solutions:

| Solution                                | Security                         | Complexity  |
| --------------------------------------- | -------------------------------- | ----------- |
| User enters API key in browser          | User is responsible for exposure | Low         |
| Your own backend proxy                  | Key stays on server              | Medium      |
| Provider-issued short-lived credentials | Limited credential exposure      | Medium/high |
| Local LLM endpoint                      | No cloud API key required        | Low/medium  |

For a public WASM application, I would recommend either:

**Option 1: User-configured backend**

The user provides an endpoint belonging to their organization or their own proxy server.

The browser never receives the permanent provider API key.

**Option 2: Local LLM**

The user runs Ollama or another local inference service.

The application connects to it over HTTP.

**Option 3: Explicitly user-managed API keys**

For an experimental application, you can allow users to enter their own keys, with a clear warning that the browser cannot securely protect them.

This can be acceptable for a personal tool, but I would not make it the recommended configuration.

### **B. Protecting the RDF data**

There is another problem: the AI can transmit the user's RDF data to an external service.

Imagine an RDF file containing:

- Personal information.
- Internal company relationships.
- Business information.
- Confidential identifiers.

Even if the user configured a trusted LLM provider, you should not automatically send the entire dataset.

I would implement context control.

For example:

```
struct AiDataPolicy {
    allow_dataset_summary: bool,
    allow_entity_details: bool,
    allow_property_values: bool,
    allow_sparql_results: bool,
    max_context_size: usize,
}
```

The user should be able to decide which data the AI can access.

I would also distinguish between:

- Analysis performed locally.
- Data sent to the LLM.
- Data returned from the LLM.

A particularly important security issue is prompt injection inside RDF literals. An RDF dataset may contain text that looks like instructions to the AI. Treat that content as untrusted data, never as system instructions.

### **C. Tool execution permissions**

Not every AI-generated action should execute automatically.

I would classify tools into three categories:

| Category          | Examples                                                 | Permission              |
| ----------------- | -------------------------------------------------------- | ----------------------- |
| Read-only         | Search entities, inspect properties                      | Automatic               |
| Visualization     | Highlight nodes, change layout                           | Automatic or reversible |
| Data modification | Delete triples, overwrite dataset, export sensitive data | Explicit confirmation   |

This also follows the security philosophy of MCP, which recommends human oversight for tool execution.&#x20;

[image](https://www.google.com/s2/favicons?domain=https://github.com\&sz=32)

GitHub+1



## **10. How I would actually implement the system**

I would avoid introducing a large agent framework, a backend server and a new frontend simultaneously.

Instead, build the system incrementally.

### **Phase 1: Introduce an application command API**

This is the most important architectural step.

Your existing application probably has functionality such as:

- Loading RDF.
- Searching.
- Filtering.
- Calculating statistics.
- Changing graph layout.
- Selecting entities.

Make these operations accessible through a typed application API.

For example:

```
enum AppCommand {
    SearchEntities(SearchRequest),
    ExecuteSparql(String),
    AnalyzeGraph(AnalysisRequest),
    FindPath(PathRequest),
    SelectEntities(Vec<EntityId>),
    HighlightEntities(Vec<EntityId>),
    ApplyLayout(LayoutConfig),
}
```

The application returns structured results:

```
enum AppResult {
    Entities(Vec<Entity>),
    Analysis(AnalysisResult),
    Path(Vec<EntityId>),
    VisualizationUpdated,
    Error(AppError),
}
```

The important thing is that this API is independent of egui.

Your normal UI, AI assistant and MCP server can all use it.

### **Phase 2: Add a minimal chat**

Implement:

- Message history.
- Streaming output.
- Tool calls.
- Tool execution.
- Error handling.
- Cancellation.

Initially, use perhaps five tools:

1. `get_dataset_summary`
2. `search_entities`
3. `execute_sparql`
4. `analyze_graph`
5. `highlight_entities`

This is enough to demonstrate the concept.

### **Phase 3: Introduce structured UI responses**

Allow the assistant to return more than text.

For example:

```
enum AiUiElement {
    Text(String),
    EntityList(Vec<EntityId>),
    PropertyTable(PropertyTable),
    AnalysisCard(AnalysisResult),
    GraphSelection(Vec<EntityId>),
    Chart(ChartData),
    ActionProposal(AppCommand),
}
```

Render these elements in the chat panel.

An entity list should be clickable. Clicking an entity should select it in the graph.

Likewise, selecting a node in the graph should update the AI context.

This is where your application starts to feel genuinely AI-native.

### **Phase 4: Add MCP**

Now implement an MCP server using `rmcp`.

It should use exactly the same application API as your embedded assistant.

You can then support both:

- Your own AI chat.
- External AI clients.

### **Phase 5: Support multiple LLM providers**

Add a provider configuration screen.

Initially support:

- OpenAI-compatible API.
- Ollama.
- One or two major hosted providers.

Don't build a sophisticated provider-management system before you know which providers your users actually need.

### **Phase 6: Introduce intelligent suggestions**

Once the assistant understands application state, add contextual recommendations.

For example:

- "This node has unusually high betweenness centrality."
- "These two communities have several bridge relationships."
- "There are isolated entities that might deserve investigation."
- "This RDF property is used inconsistently."

These are interesting because the application can detect patterns using its own algorithms and ask the LLM to explain them.

The LLM should not be responsible for calculating graph statistics that your existing Rust engine can calculate exactly.

## **11. My recommended target architecture**

Putting everything together, this is the architecture I would aim for:

**Interaction layer**

Multiple ways to use the same application

Graph UI

AI chat

MCP clients

**AI orchestration layer**

Context · Tool calling · LLM provider · Permissions · UI responses

**Application API / Command bus**

Typed commands · Events · Results · Async execution

**RDF domain and analysis engine**

RDF data model

Graph algorithms

One application core, several interaction methods and deployment targets.

I would preserve your Rust/egui architecture and build the AI functionality around it, rather than rewrite the application.

### **Technology selection**

| Area               | My recommendation                                |
| ------------------ | ------------------------------------------------ |
| Desktop UI         | egui                                             |
| WASM UI            | egui / eframe                                    |
| Application API    | Typed Rust commands and events                   |
| LLM integration    | Rig or custom lightweight provider layer         |
| MCP                | rmcp                                             |
| Local LLM          | Ollama / llama.cpp                               |
| Remote LLM         | OpenAI-compatible API + native provider adapters |
| Chat UI            | Custom egui components                           |
| AI-generated UI    | Typed declarative components                     |
| Async execution    | Tokio + channels                                 |
| Browser networking | HTTP/fetch adapter                               |

### **A few important design decisions**

I would make these principles explicit:

1. **AI is optional.** The RDF viewer must remain fully functional without an LLM.
2. **The LLM does not own the graph.** Your Rust engine remains authoritative.
3. **The visualization is part of the conversation.** Chat and graph share context and state.
4. **Tools are application capabilities.** Don't expose internal implementation details.
5. **UI generation is constrained.** Render predefined native components rather than arbitrary generated code.
6. **Provider-independent architecture.** Users can configure their own infrastructure.
7. **WASM compatibility is a first-class requirement.** Avoid putting platform-specific networking or filesystem code in the shared application core.

## **12. What I would build first**

I would actually start with a small proof of concept, not a complete AI assistant.

Suggested MVP

**RDF AI exploration prototype**

1. **Add a collapsible chat panel**

   Keep the graph as the primary workspace. Chat should not permanently consume half the screen.
2. **Implement five AI tools**

   Dataset summary, entity search, SPARQL, graph analysis and visualization control.
3. **Connect one LLM provider**

   Start with an OpenAI-compatible endpoint or Ollama.
4. **Implement shared application context**

   Selection, current visualization, active dataset and previous analysis.
5. **Render interactive results in chat**

   Entity lists, analysis cards, tables and visualization actions.
6. **Test the same architecture in WASM**

   Check browser networking, asynchronous execution and provider configuration.

After this prototype, you will know whether the architecture feels right before investing in MCP and more sophisticated agent capabilities.

## **13. Further reading and projects**

These are the resources I would prioritize investigating:

| Resource                                                                                       | Why it matters                       |
| ---------------------------------------------------------------------------------------------- | ------------------------------------ |
| [Rust MCP SDK (rmcp)](https://github.com/modelcontextprotocol/rust-sdk?utm_source=chatgpt.com) | Native MCP implementation            |
| [Rig](https://github.com/0xPlaygrounds/rig?utm_source=chatgpt.com)                             | Rust LLM framework                   |
| [chat-rs](https://github.com/EggerMarc/chat-rs?utm_source=chatgpt.com)                         | Alternative multi-provider framework |
| [MCP Apps](https://github.com/modelcontextprotocol/ext-apps?utm_source=chatgpt.com)            | Interactive UI in external AI chats  |
| [LLM-enabled visualization survey](https://arxiv.org/abs/2601.14943?utm_source=chatgpt.com)    | Research and interaction patterns    |
| [ProactiveVA](https://zyh1222.github.io/ProactiveVA/?utm_source=chatgpt.com)                   | Mixed-initiative visual analytics    |

**My overall conclusion:** Your most interesting opportunity is not to build another RDF chatbot. It is to build a visual knowledge exploration application in which the AI understands the graph, understands what the user is currently looking at, and can manipulate the visualization through natural language.

The combination of RDF, graph algorithms, interactive visualization and natural-language interaction is a particularly good foundation for this.

One useful next step would be to design the actual Rust module architecture and public interfaces for `AiContext`, `AppCommand`, `AiEvent`, `LlmProvider` and the MCP adapter, keeping native and WASM builds compatible.

The recommendation also builds on your existing egui command-bus and asynchronous execution approach.

I can also keep track of new Rust LLM frameworks and MCP/visual-analytics developments as this ecosystem evolves.

🔎Track Rust AI and MCP developmentsZum Einrichten tippen