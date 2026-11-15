Yes, this is a very interesting workflow, and it is possible to implement most of it with MCP. However, there is one important distinction:

**MCP can notify a connected chat client that something has changed, but MCP does not generally give a server the authority to start a new AI conversation or trigger an assistant turn.**

The chat application (the MCP host) controls when the LLM is invoked.

I would therefore distinguish two scenarios:

1. You use an external chat application such as Claude Desktop.
2. You build your own chat inside your RDF viewer.

The second gives you much more control over the workflow.

# **1. Your desired workflow**

I understand the intended interaction as follows:

1. **User loads RDF data**

   The application calculates its metadata and statistical profile.
2. **MCP server announces new data**

   A notification indicates that the current dataset has changed.
3. **AI receives the dataset description**

   The AI retrieves the structural profile and interprets it.
4. **AI proposes interesting explorations**

   For example:
   - Visualize the entity types and relationships.
   - Find the most central entities.
   - Identify unusual relationship patterns.
   - Show attribute distributions.
5. **User chooses an exploration**

   The AI calls the corresponding application tool.
6. **RDF viewer updates its visualization**

   The result appears in the graph viewer, not just in the chat.

I would call this a *dataset onboarding workflow* or *AI-guided dataset exploration*.

# **2. Can MCP send events to the chat?**

MCP has several notification mechanisms, but they serve different purposes.

| MCP mechanism                 | What it does                                                 | Can start a chat turn?     |
| ----------------------------- | ------------------------------------------------------------ | -------------------------- |
| Resource updated notification | Announces changed resource data                              | No, not by itself          |
| Resource list changed         | Announces changed available resources                        | No                         |
| Tool list changed             | Announces changed tools                                      | No                         |
| Progress notification         | Reports progress of an operation                             | No                         |
| Custom protocol notification  | Can communicate additional events if both sides support them | Not standard chat behavior |

For your case, the most relevant mechanism is:

`notifications/resources/updated`

Suppose your server exposes:

```
dataset://current/overview
```

When the user loads another dataset, the server can send a notification that this resource has changed.

The chat client can then retrieve the new resource contents.

But there is a difference between:

- The chat client knowing that new data exists.
- The chat client deciding to ask the LLM to analyze it.

The latter is controlled by the host application.

# **3. How I would implement it with MCP**

I would expose one stable resource and send update notifications whenever the dataset changes.

For example:

```
dataset://current/overview
```

When the user loads a new dataset:

1. Update the dataset profile.
2. Increment its version.
3. Notify subscribed MCP clients that the resource changed.
4. The client can retrieve the new profile.

Conceptually, the notification looks like this:

```
{
  "jsonrpc": "2.0",
  "method": "notifications/resources/updated",
  "params": {
    "uri": "dataset://current/overview"
  }
}
```

Your resource could return:

```
{
  "dataset_id": "dataset-42",
  "version": 3,
  "summary": {
    "entity_count": 510,
    "entity_type_count": 2,
    "relationship_count": 1200
  },
  "entity_types": [
    {
      "name": "Student",
      "count": 500
    },
    {
      "name": "Vorlesung",
      "count": 10
    }
  ],
  "relationships": [
    {
      "name": "attends",
      "source": "Student",
      "target": "Vorlesung",
      "count": 1200
    }
  ]
}
```

The actual notification contains only the resource URI, not the entire dataset description. The client reads the resource separately.

See the [MCP resource specification](https://github.com/modelcontextprotocol/modelcontextprotocol/blob/main/docs/specification/2026-07-28/server/resources.mdx?utm_source=chatgpt.com) and [MCP notification specification](https://github.com/modelcontextprotocol/modelcontextprotocol/blob/main/docs/specification/2026-07-28/basic/index.mdx?utm_source=chatgpt.com).

One transport detail: the current 2026-07 specification uses `subscriptions/listen` for server-to-client change notifications over Streamable HTTP. Older MCP implementations use resource subscriptions. If you want compatibility with existing clients, check which protocol version and subscription mechanism they support.&#x20;

[image](https://www.google.com/s2/favicons?domain=https://github.com\&sz=32)

GitHub+1



## **4. The missing piece: the chat host**

Here is how the complete workflow differs depending on who owns the chat.

**Scenario A: External chat**

**Your RDF application**

Loads dataset and updates profile

MCP resource notification

**External MCP client**

Receives notification, if supported

Client decides what to do

**LLM**

Analyzes data only if the host invokes it

You cannot guarantee that Claude Desktop or another generic MCP client will automatically start a new assistant turn. That is a host-specific feature.

**Scenario B: Your own embedded chat**

**RDF application**

DatasetLoaded event

**Chat controller**

Starts assistant turn automatically

**LLM**

Retrieves dataset profile and suggests explorations

Fully under your control

This second scenario is straightforward to implement. You don't even need an MCP notification between your application and your own chat.

You can use an internal Rust event:

```
enum AppEvent {
    DatasetLoaded {
        dataset_id: DatasetId,
        version: u64,
    },
}
```

Your chat controller listens for it and starts the LLM request.

MCP then remains the external integration interface.

# **5. How the AI should generate exploration suggestions**

I would not hardcode suggestions such as "show central nodes" into the MCP server.

Instead, give the LLM two things:

1. A structural description of the dataset.
2. A list of available analytical capabilities.

Then let it decide which explorations are interesting.

For example, your application could calculate these observations:

```
{
  "observations": [
    {
      "type": "high_cardinality_relationship",
      "description": "Students are connected to multiple courses."
    },
    {
      "type": "imbalanced_distribution",
      "description": "There are 500 students but only 10 courses."
    },
    {
      "type": "highly_connected_entities",
      "description": "Some course entities have many incoming relationships."
    }
  ]
}
```

The LLM can turn these into suggestions:

**Suggested explorations**

**Visualize the data structure**

Show the entity types, attributes and relationships as a diagram.

**Find the most popular courses**

Rank courses by the number of distinct students.

**Find central entities**

Identify entities with unusually many connections.

**Compare students**

Find students attending the most and fewest courses.

There is also an opportunity to incorporate user preferences.

For example, if a user repeatedly investigates centrality, community structures and relationship distributions, the application could maintain a small preference profile:

```
{
  "preferred_analyses": [
    "centrality",
    "community_detection",
    "relationship_statistics"
  ]
}
```

The LLM can use this when generating suggestions.

I would keep this preference profile separate from the dataset profile.

# **6. My recommendation for your Rust implementation**

Given your existing egui architecture and command bus, I would implement the following:

```
Application
│
├── DatasetManager
│   └── DatasetProfile
│
├── ApplicationEventBus
│   └── DatasetLoaded
│
├── AiController
│   ├── ContextManager
│   ├── LlmProvider
│   └── ToolExecutor
│
├── McpServer
│   ├── Resources
│   ├── Tools
│   └── Notifications
│
└── UI
    ├── GraphViewer
    └── ChatPanel
```

The most important design decision is that `DatasetProfile` and `ApplicationEventBus` are independent of MCP.

Both your AI controller and MCP server can subscribe to application events.

This gives you a clean separation:

- Application events describe what happened.
- MCP communicates changes to external clients.
- The AI controller decides when to invoke the LLM.
- The LLM decides what to investigate.
- Your application executes the requested operations.

## **7. One additional idea: make dataset onboarding a reusable AI workflow**

I would define a named workflow, something like:

`ExploreNewDataset`

It could have these stages:

1. Retrieve dataset profile.
2. Understand the entity types and relationships.
3. Generate a short domain-oriented description.
4. Identify interesting structural characteristics.
5. Generate three to five exploration suggestions.
6. Present suggestions to the user.

The output should be structured, for example:

```
struct DatasetIntroduction {
    description: String,
    inferred_domain: Option<String>,
    observations: Vec<String>,
    suggestions: Vec<ExplorationSuggestion>,
}

struct ExplorationSuggestion {
    title: String,
    explanation: String,
    tool: String,
    arguments: serde_json::Value,
}
```

The `tool` and `arguments` should be validated against your registered tool definitions before execution. Generating a suggestion must not automatically execute it.

This workflow could eventually become an MCP Skill, but it can also simply be implemented in your own chat controller.

**My final recommendation:** For your first experiment, I would implement `DatasetLoaded` as an internal application event, automatically trigger the LLM in your own chat, and expose the same dataset profile through MCP resources.

That gives you the exact experience you want while avoiding dependence on external chat clients' support for automatic event handling.

MCP should be the interface for external AI clients, not the event system responsible for controlling your own application.

You can later test how much of the same workflow works with Claude Desktop or other MCP hosts.

If useful, I can next sketch a concrete Rust implementation using `rmcp`, Tokio channels and your existing command-bus pattern, including the resource-update notification and automatic chat trigger.

🔧Track MCP protocol and Rust SDK changesZum Einrichten tippen