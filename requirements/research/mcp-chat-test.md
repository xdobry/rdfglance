
# **Testing an RDF Analysis MCP Server with AI Chat**

Your RDF analysis tool could become much more than a graph viewer with an AI interface. With MCP (Model Context Protocol), users could **explore an unfamiliar RDF dataset, ask analytical questions, discover relationships, run graph algorithms, and interactively visualize results** using natural language.

I would evaluate it in two dimensions:

1. **What questions can users ask?** Build a realistic benchmark of questions ranging from simple data exploration to complex graph analysis.
2. **What can your tool do better than generic AI data analysis?** Identify where RDF semantics, graph algorithms, and interactive visualizations provide unique value.

A Star Wars RDF dataset is a good starting point because it contains understandable entities and relationships, making answers relatively easy to verify.

## **1. Example conversation: Exploring a Star Wars RDF dataset**

The following is a possible conversation between a user and an AI assistant connected to your RDF tool through MCP.

**Step 1 — Dataset discovery**

USER

Load the Star Wars dataset. What is this dataset about?

AI ASSISTANT — EXAMPLE RESPONSE

The dataset describes the Star Wars universe, including films, characters, actors, planets, species, starships, and vehicles.

## **12**

Films

## **184**

Characters

## **96**

Actors

## **43**

Planets

## **67**

Starships

## **1,420**

Relationships

Illustrative figures only — not actual dataset results.

The most common relationships are *appears in*, *portrayed by*, *homeworld*, and *pilots*.

**Step 2 — Querying entities**

USER

Which characters appear in the most films, and which actors portray them?

AI ASSISTANT — EXAMPLE RESPONSE

Here are some of the most frequently appearing characters:

| Character      | Films | Actor                         |
| -------------- | ----- | ----------------------------- |
| C-3PO          | 9     | Anthony Daniels               |
| R2-D2          | 9     | Kenny Baker / Jimmy Vee       |
| Chewbacca      | 8     | Peter Mayhew / Joonas Suotamo |
| Luke Skywalker | 5     | Mark Hamill                   |

Illustrative query output; counts and actor mappings must be verified against the loaded RDF dataset and its film scope.

**Step 3 — Graph exploration**

USER

Show the relationships between Luke Skywalker, Darth Vader, Leia, and Han Solo.

AI ASSISTANT — EXAMPLE RESPONSE

I found relationships involving family, friendship, and film appearances. Here is a simplified relationship graph.

Conceptual graph; actual relationships should be read from RDF, not inferred from general Star Wars knowledge.

The important point is that the AI should not simply answer from its pretrained knowledge. It should **query the actual dataset and distinguish retrieved facts from its own explanations**.

## **2. A benchmark of questions users might ask**

I would organize your test cases into eight categories. Each category tests a different MCP capability and represents a realistic user workflow.

## **MCP test question library**

48 test questions across 8 categories

0 selected

AllBasicIntermediateAdvancedExpert

1\. Dataset discovery and understanding6 questions · Basic

What is this dataset about?

Dataset summary, classes, predicates, counts

What are the main entity types and how are they connected?

Schema exploration

How many films, characters, actors, and planets are represented?

Aggregation

What properties describe a character? Show examples.

Property inspection

Are there multiple vocabularies or ontologies in this dataset?

Namespace discovery

What interesting questions could I ask about this dataset?

AI-guided exploration

2\. Entity search and tabular analysis6 questions · Basic

Which characters appear in the most films?

Group and count

Which actors portray more than one character?

Multi-hop aggregation

List all films released after 1980, sorted by year.

Filtering and sorting

Which planets have the most associated characters?

Relationship aggregation

Find characters whose names contain 'Skywalker'.

Entity search

Show a table of films, their directors, and their release years.

Multi-entity table

3\. Relationship exploration and paths6 questions · Intermediate

How are Luke Skywalker and Darth Vader connected?

Path discovery

Find the shortest connection between Yoda and Han Solo.

Shortest path

Which characters have appeared together in at least three films?

Derived co-occurrence

Show all actors within two relationship steps of Luke Skywalker.

Neighborhood traversal

Which characters connect the original and prequel trilogies?

Cross-group relationships

Show the subgraph of characters, actors, and films connected to Leia.

Subgraph visualization

4\. Centrality and importance6 questions · Advanced

Who are the ten most central characters in the Star Wars network?

Degree/PageRank

Which characters act as bridges between otherwise separate groups?

Betweenness centrality

Compare the top characters by degree, PageRank, and betweenness.

Algorithm comparison

Which characters are important despite appearing in relatively few films?

Centrality versus frequency

Who is most centrally located in the character co-appearance network?

Closeness centrality

Highlight the top 5% of characters by PageRank in the graph.

Algorithm + visualization

5\. Communities and clustering6 questions · Advanced

6\. RDF semantics and knowledge graph quality6 questions · Advanced

7\. Interactive exploration and visualization6 questions · Advanced

8\. Multi-step investigation6 questions · Expert

Select all

Clear Copy selected (0)

The last category is especially important. A useful MCP integration should support an **analytical conversation**, not just isolated tool calls.

For example, the user might first ask for central characters, then ask why one ranks highly, then request a graph containing only its relevant relationships. The AI should reuse previous results rather than repeating every computation.

## **3. Compare with AI-native data analysis tools**

I would use three reference systems:

- **ChatGPT Data Analysis** — a general-purpose AI analyst that can work with files, Python, tables, and charts.&#x20;

  [image](https://www.google.com/s2/favicons?domain=https://help.openai.com\&sz=32)

  OpenAI Help Center

- **Neo4j Graph Data Science** — a graph-specific analytical system with centrality, community detection, similarity, and pathfinding algorithms. It is not itself a conversational AI, but provides a useful feature benchmark.&#x20;

  [image](https://www.google.com/s2/favicons?domain=https://neo4j.com\&sz=32)

  Getting Started+1

- **Your RDF tool + MCP** — a specialized knowledge graph analyst with direct access to RDF semantics, graph algorithms, and interactive views.

### **Feature comparison**

✓ Native or straightforward · ◐ Possible with additional code or integration · — Not a native feature

| Capability                         | ChatGPT Data Analysis | Neo4j GDS | Your RDF MCP target |
| ---------------------------------- | --------------------- | --------- | ------------------- |
| Natural-language questions         | ✓                     | ◐         | ✓                   |
| Dataset summaries                  | ✓                     | ◐         | ✓                   |
| Filtering and aggregation          | ✓                     | ✓         | ✓                   |
| Interactive tables                 | ✓                     | ◐         | ✓                   |
| Statistical charts                 | ✓                     | ◐         | ◐                   |
| RDF triples and namespaces         | ◐                     | ◐         | ✓                   |
| RDFS/OWL semantics                 | ◐                     | ◐         | ✓                   |
| SPARQL queries                     | ◐                     | ◐         | ✓                   |
| Shortest paths                     | ◐                     | ✓         | ✓                   |
| Centrality algorithms              | ◐                     | ✓         | ✓                   |
| Community detection                | ◐                     | ✓         | ✓                   |
| Interactive graph exploration      | ◐                     | ◐         | ✓                   |
| Visual graph queries               | —                     | ◐         | ✓                   |
| Conversational multi-step analysis | ✓                     | ◐         | ✓                   |

This compares the proposed target capabilities, not features already proven to work through your MCP implementation. Neo4j's ecosystem offers visualization and AI integrations, but its Graph Data Science library alone does not provide all of those interfaces.

ChatGPT Data Analysis already supports Python-based calculations, structured tables, and static or interactive charts.&#x20;

[image](https://www.google.com/s2/favicons?domain=https://help.openai.com\&sz=32)

OpenAI Help Center

&#x20;Neo4j GDS provides a broad set of graph algorithms, including PageRank, betweenness centrality, Louvain, and similarity analysis.&#x20;

[image](https://www.google.com/s2/favicons?domain=https://neo4j.com\&sz=32)

Neo4j Graph Data Science+1



### **Where your RDF tool can differentiate itself**

The main opportunity is not reproducing pandas or Python functionality. It is making knowledge graph analysis much easier.

For example:

**Generic AI Data Analysis**

A user uploads RDF data. The AI may need to parse the triples, understand the vocabulary, create a graph representation, write Python code, calculate centrality, and generate a visualization.

**Your RDF MCP**

The AI discovers the dataset's classes and predicates, queries the graph, runs the native centrality implementation, and asks the visualization to highlight the results.

This can be faster and more consistent, particularly for larger RDF datasets and repeated graph exploration.

## **4. Which MCP tools should you expose?**

I would avoid creating one MCP tool for every possible question. Instead, expose a relatively small set of generic operations.

MCP supports executable tools, contextual resources, and reusable prompt templates.&#x20;

[image](https://www.google.com/s2/favicons?domain=https://github.com\&sz=32)

GitHub



| MCP tool               | Purpose                            | Example question                       |
| ---------------------- | ---------------------------------- | -------------------------------------- |
| `load_dataset`         | Load an RDF file or URL            | Load Star Wars                         |
| `dataset_info`         | Counts, vocabularies, statistics   | What is this about?                    |
| `explore_schema`       | Classes, predicates, relationships | How is the data structured?            |
| `search_entities`      | Search by label, type, IRI         | Find Luke Skywalker                    |
| `get_entity`           | Properties and connections         | Tell me about Yoda                     |
| `query_sparql`         | Flexible graph querying            | Which actors play multiple characters? |
| `get_subgraph`         | Extract neighborhoods or paths     | Show connections between Luke and Leia |
| `run_algorithm`        | Centrality, clustering, etc.       | Find the most influential characters   |
| `update_visualization` | Filter, highlight, color, layout   | Color nodes by community               |
| `get_table`            | Retrieve structured query results  | Show the results as a table            |

I would particularly recommend making `explore_schema` and `query_sparql` available early. Together they allow an LLM to answer many questions you have not explicitly anticipated.

For `run_algorithm`, consider a generic interface:

```
{
  "algorithm": "betweenness",
  "graph": {
    "node_types": ["Character"],
    "relationship_types": ["appearsWith"],
    "directed": false
  },
  "limit": 20
}
```

The important part is the graph projection. For example, calculating centrality across all RDF triples is often meaningless because the graph contains classes, literals, metadata, and unrelated relationship types.

The AI should be able to construct or select an appropriate analytical graph.

## **5. A particularly interesting feature: AI-controlled visual queries**

Because your tool already has a visual query interface, this could be one of its most distinctive capabilities.

Consider the question:

> Find all characters who appear in at least three films together with Luke Skywalker, and show their actors.

The AI could construct a query graph:

Conceptual visual query, assuming the dataset contains the required predicates.

The user could then modify the visual query manually, or ask the AI:

> Exclude animated films and sort by the number of shared appearances.

That creates a useful combination of natural-language querying and visual query editing. The query is no longer an invisible piece of generated SPARQL; the user can inspect and refine it.

## **6. How to evaluate MCP quality**

I would not measure success only by whether the AI produces a plausible answer.

For every benchmark question, measure four aspects:

| Dimension               | What to test                                                    |
| ----------------------- | --------------------------------------------------------------- |
| **Correctness**         | Are all reported facts supported by the dataset?                |
| **Tool selection**      | Did the AI choose suitable MCP operations?                      |
| **Analytical quality**  | Was the right graph projection, metric, and algorithm selected? |
| **Interaction quality** | Can the user inspect, visualize, and refine the result?         |

Additionally, test situations where the correct answer is not available:

- A character is not present in the dataset.
- An actor relationship is missing.
- The dataset has several entities with the same label.
- The user asks about a predicate that does not exist.
- The user requests PageRank on an unsuitable graph projection.
- A SPARQL query returns no results.

These tests help reveal hallucinations, unsupported assumptions, and poor tool selection.

For reliable automated evaluation, store the expected results as SPARQL queries or verified result tables. For algorithm tests, fix the graph projection, algorithm parameters, and tie-breaking rules.

## **7. My suggested MVP**

I would implement the MCP integration in three stages.

**Phase 1 — Conversational RDF explorer**

Load datasets, inspect the schema, search entities, execute SPARQL, and return tables.

*Goal:* An AI can answer questions about an unfamiliar RDF dataset without prior knowledge of its vocabulary.

**Phase 2 — Graph analysis assistant**

Add pathfinding, centrality, clustering, graph projections, and explanations of results.

*Goal:* The AI can perform analytical tasks that would otherwise require users to understand graph algorithms.

**Phase 3 — Interactive graph copilot**

Allow the AI to construct visual queries, select nodes, expand neighborhoods, change layouts, and highlight analytical findings.

*Goal:* Natural language and direct manipulation of the graph work together.

One architectural consideration: MCP can return structured analytical results, but it does not automatically give an AI client control over an existing desktop application's graph canvas. For visualization commands, you will need an integration between the MCP server and the running application, or a client that can render the returned graph data.

**The most compelling demonstration would be a multi-turn investigation:**

> Load this RDF dataset and explain what it contains.

> Find the most important entities using graph centrality.

> Why is this entity important?

> Show the subgraph that explains its centrality.

> Are there communities in this graph?

> Color the communities and highlight the entities connecting them.

> Summarize the findings and identify anything surprising.

This demonstrates a capability beyond ordinary question answering: **an AI assistant that can investigate a knowledge graph, calculate structural properties, explain them, and let the user explore the evidence visually.**

# Working Graph

I would recommend **keeping your existing graph construction model and exposing it to the AI through MCP**.

In other words, let the AI construct a working graph using the same operations that a human uses in your visual application.

However, I would introduce one important architectural distinction:

**Use stateful graph workspaces for exploration, but make analytical operations deterministic and reproducible through immutable graph snapshots.**

This gives you the flexibility of interactive graph exploration without the disadvantages of uncontrolled state.

## **1. Two possible approaches**

### **Approach A — Stateless graph projections**

The AI specifies the entire graph projection as part of an algorithm call.

For example:

```
{
  "algorithm": "pagerank",
  "projection": {
    "nodeTypes": ["Actor", "Film"],
    "predicates": ["playsIn"],
    "direction": "both",
    "filters": {
      "Film.releaseYear": {
        "gte": 1977,
        "lte": 1983
      }
    }
  }
}
```

This is conceptually similar to graph projections in Neo4j Graph Data Science.

**Advantages:** Reproducible, straightforward to test, easy to run in parallel, and independent of previous MCP calls.

**Disadvantages:** Complex projections become difficult to express. You effectively need to develop a graph query language for your MCP API.

It also doesn't naturally support your existing workflow, where the user incrementally expands and filters a graph.

### **Approach B — Stateful working graph**

The AI manipulates a graph workspace.

For example:

```
create_graph()
    ↓
add_nodes(type="Film")
    ↓
expand_nodes(predicate="hasActor")
    ↓
remove_nodes(type="Film", filter="year < 1977")
    ↓
disable_predicate("hasActor")
    ↓
enable_predicate("appearsIn")
    ↓
run_algorithm("pagerank")
```

This is essentially your existing visual graph workflow exposed through MCP.

**Advantages:** Reuses existing functionality, supports incremental exploration, and mirrors the user's mental model.

**Disadvantages:** The AI needs to track the current graph state. Operations can depend on previous calls, and unexpected changes to the graph can affect results.

## **2. My recommendation: A stateful graph workspace with snapshots**

Proposed architecture: mutable exploration, immutable analytical input.

The AI should work with a graph workspace just as the user does. Every change creates a new version of that workspace.

When an algorithm runs, it uses a particular version, not an unspecified current graph.

For example:

```
{
  "tool": "run_algorithm",
  "arguments": {
    "graph_id": "graph-42",
    "graph_version": 7,
    "algorithm": "pagerank"
  }
}
```

This prevents a common problem: the user modifies the graph while the AI is calculating something.

## **3. Suggested MCP interface**

I would expose a small set of composable graph operations.

| Tool                   | Purpose                                           |
| ---------------------- | ------------------------------------------------- |
| `graph_create`         | Create an empty working graph                     |
| `graph_add`            | Add entities using search, type, or query results |
| `graph_expand`         | Expand selected nodes by relationships            |
| `graph_remove`         | Remove nodes or edges                             |
| `graph_set_predicates` | Enable or disable relationship types              |
| `graph_info`           | Return current graph state and statistics         |
| `graph_snapshot`       | Capture a specific version                        |
| `graph_analyze`        | Run an algorithm on a snapshot                    |

For example, the AI could process the request:

> Find the most central characters connected to films in the original Star Wars trilogy.

1

**Create a working graph**

```
graph_create({})
// Returns: graph_id = "graph-42"
```

2

**Add the three films**

```
graph_add({
  "graph_id": "graph-42",
  "entities": ["film-4", "film-5", "film-6"]
})
```

3

**Expand to characters**

```
graph_expand({
  "graph_id": "graph-42",
  "predicate": "hasCharacter",
  "direction": "outgoing",
  "from": "all",
  "target_type": "Character"
})
```

4

**Inspect the graph**

```
graph_info({
  "graph_id": "graph-42"
})
```

Example response:

```
{
  "version": 3,
  "nodes": 46,
  "edges": 103,
  "node_types": {
    "Film": 3,
    "Character": 43
  }
}
```

5

**Run the algorithm**

```
graph_analyze({
  "graph_id": "graph-42",
  "graph_version": 3,
  "algorithm": "betweenness"
})
```

The example identifiers, counts, and predicate names are illustrative.

There is one important semantic detail: this calculation measures betweenness in the **Film–Character bipartite graph**. It is not necessarily the same as betweenness in a Character–Character co-appearance graph.

The AI should explicitly explain that distinction.

## **4. Separate graph membership from graph visibility**

Your current model has an interesting characteristic: the visual graph is also the input for algorithms.

I would preserve that behavior, but distinguish three concepts internally:

| Concept                 | Meaning                                                    |
| ----------------------- | ---------------------------------------------------------- |
| **Graph membership**    | Which nodes and edges belong to the working graph?         |
| **Algorithm inclusion** | Which nodes and edges are considered by algorithms?        |
| **Visual presentation** | Which nodes and edges are visible, highlighted, or styled? |

For example, hiding a predicate in the UI might mean two different things:

1. Hide its edges visually but retain them for calculations.
2. Exclude its edges from both visualization and calculations.

Your current tool apparently implements the second behavior. That is perfectly reasonable, but the MCP interface should make it explicit.

Otherwise the AI might say, "Hide actor relationships," unintentionally changing PageRank results.

I would expose operations such as `graph_set_edge_enabled` rather than an ambiguous `graph_hide_edges`.

## **5. How much state should the AI manage?**

I would avoid forcing the AI to remember all graph contents.

Instead, let the MCP server manage the state and provide compact summaries.

For example, `graph_info` could return:

```
{
  "graph_id": "graph-42",
  "version": 7,
  "nodes": 348,
  "edges": 1240,
  "node_types": {
    "Character": 250,
    "Film": 12,
    "Actor": 86
  },
  "enabled_predicates": [
    "appearsIn",
    "portrayedBy"
  ],
  "disabled_predicates": [
    "homeworld"
  ],
  "connected_components": 3
}
```

The AI doesn't need the complete graph in its context window. It only needs enough information to plan the next operation.

This is especially important for large datasets.

I would also make graph-changing operations return a summary of what changed:

```
{
  "graph_id": "graph-42",
  "version": 8,
  "added_nodes": 42,
  "added_edges": 87,
  "total_nodes": 390,
  "total_edges": 1327
}
```

## **6. Make graph construction reproducible**

A stateful interface does not mean you must sacrifice reproducibility.

You can record the sequence of operations that constructed the graph.

For example:

```
{
  "operations": [
    {
      "op": "add",
      "type": "Film",
      "filter": "releaseYear >= 1977"
    },
    {
      "op": "expand",
      "predicate": "hasCharacter",
      "direction": "outgoing"
    },
    {
      "op": "disable_predicate",
      "predicate": "homeworld"
    }
  ]
}
```

This gives you several additional capabilities:

- Replay the analysis on another dataset.
- Save and share graph configurations.
- Compare different graph versions.
- Restore a previous working graph.
- Reproduce an AI-generated investigation.

It also means you can later introduce a stateless `graph_build` command that accepts a sequence of operations.

You don't have to implement a complex projection language immediately.

## **7. Shared graph or separate AI workspace?**

This is another important design decision.

Suppose the user has already built a graph manually and asks:

> Which nodes are most central in my current graph?

The AI should operate on the user's existing graph.

But suppose the user asks:

> Compare centrality in this graph with centrality across all films.

Now the AI needs to construct another graph without destroying the user's current work.

I would support both:

**Shared workspace:** The AI operates directly on the graph visible in the application.

**Temporary workspace:** The AI clones or creates a graph for an independent investigation.

A possible MCP interface:

```
graph_clone({
  "source_graph_id": "visible-graph",
  "new_graph_id": "analysis-graph"
})
```

Then the AI can modify `analysis-graph` and compare results without affecting the user's original visualization.

For shared workspaces, version checks are useful to avoid accidental overwrites. For temporary workspaces, you should also define cleanup or expiration rules.

## **8. One additional feature I would strongly consider: graph previews**

An AI can easily create a graph containing thousands of nodes when it intended to create one containing 50.

Before performing expensive expansion operations, it would be useful to estimate the effect.

For example:

```
graph_expand({
  "graph_id": "graph-42",
  "predicate": "appearsIn",
  "dry_run": true
})
```

Response:

```
{
  "estimated_added_nodes": 12500,
  "estimated_added_edges": 42000
}
```

The AI can then decide to limit the expansion or ask the user.

A dry run should either return an exact count or clearly indicate that the number is an estimate.

## **Final recommendation**

I would not introduce a sophisticated stateless projection language as the primary MCP interface.

Your existing visual graph construction is already a powerful projection mechanism. Reusing it gives you a consistent conceptual model across the GUI and the AI interface.

The architecture I would choose is:

**RDF dataset → Mutable working graph → Versioned snapshot → Graph algorithms**

And the MCP API should support both incremental construction and replayable graph construction recipes.

The strongest design principle is that **the user and the AI should be able to manipulate the same graph through the same underlying operations**. That allows a particularly useful interaction:

> User constructs a graph visually → AI analyzes it → AI expands interesting regions → User manually refines the graph → AI continues the investigation.

That is something generic AI data analysis tools typically do not offer as a native workflow.