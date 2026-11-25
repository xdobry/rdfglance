I would approach this a little differently from a conventional MCP server that simply exposes a collection of tools.

What you describe is essentially a **data-understanding layer** between your RDF viewer and the AI.

Your application already knows a lot about the loaded dataset:

- Which entity types exist.
- How many instances each type has.
- Which attributes belong to each type.
- Which relationships exist between types.
- How frequently attributes and relationships occur.
- Which classes are connected to which other classes.

This information is much more valuable than simply exposing a generic SPARQL query tool.

I would make the first MCP integration revolve around three concepts:

1. **Dataset description:** What kind of data is loaded?
2. **Dataset exploration:** What can the user investigate?
3. **Dataset analysis:** Answer questions using the application's existing statistics and graph algorithms.

The AI should first understand the dataset, then use its tools to answer questions.

# **1. First, introduce an abstract dataset description**

I would not expose RDF terminology as the primary language to the LLM.

Your application can internally work with RDF, but the AI should see a more general model:

- Entity types instead of RDF classes.
- Entities instead of RDF instances.
- Attributes instead of datatype properties.
- Relationships instead of object properties.
- Cardinalities and frequencies as statistical information.

For example, your application could produce the following description after loading a dataset.

**Dataset overview**

Loaded

# **510**

Entities in 2 entity types

**Entity types**

**Student**

## **500**

Attributes

| Attribute | Data type | Cardinality |
| --------- | --------- | ----------- |
| name      | string    | 500         |
| age       | integer   | 480         |
| email     | string    | 495         |

Relationships

\- attends → Vorlesung (1–5 per student)

**Vorlesung**

## **10**

Attributes

| Attribute | Data type | Cardinality |
| --------- | --------- | ----------- |
| title     | string    | 10          |
| semester  | string    | 10          |

Relationships

\- attended by ← Student

**Relationship summary**

Student → Vorlesung\
**1–5**

Example schema description, with illustrative values.

The important detail is that you don't need to send every entity to the LLM.

You send a compact structural description of the dataset.

The LLM can infer:

> This dataset describes students, university courses and the relationships between students and courses. It appears to represent course attendance.

This is exactly the sort of understanding you want.

One caveat: the AI should distinguish between what is explicitly known and what it infers. For example, a relationship called `attends` may suggest attendance, but it doesn't necessarily mean completed attendance or successful participation.

# **2. How to communicate this to an MCP chat**

MCP has several mechanisms for exactly this purpose. They have different roles.

| MCP mechanism       | What you should use it for                                               |
| ------------------- | ------------------------------------------------------------------------ |
| Server instructions | Explain the general purpose and operating principles of your application |
| Resources           | Provide the dataset description and statistical metadata                 |
| Tools               | Allow the AI to investigate and analyze the data                         |
| Prompts             | Provide predefined analysis workflows                                    |
| Skills              | Reusable, more extensive instructions, where supported                   |

This distinction is important.

## **A. Server instructions: explain what your application does**

MCP supports server-level instructions during initialization.

This is probably the first thing you should implement.

You can give the AI instructions such as:

**Example: RDF viewer server instructions**

Conceptual instruction text sent by your MCP server

You are connected to a data exploration application.

The application loads structured datasets and provides statistical information about their contents.

The data consists of entity types, entities, attributes and relationships.

Your responsibilities:

1. Understand the dataset structure before answering questions.
2. Use the dataset overview to understand what the data represents.
3. Explain the dataset in domain-oriented language, not technical RDF terminology.
4. Use the available analysis tools to answer questions.
5. Prefer exact application statistics over estimates.
6. Clearly distinguish observed facts from interpretations.
7. When appropriate, return structured results that can be displayed as tables or visualizations.

When a new dataset is available, retrieve its dataset overview before performing analysis.

The server instructions are particularly useful because they apply across the tools exposed by your server. OpenAI's MCP integration documentation also explicitly describes this mechanism.&#x20;

[image](https://www.google.com/s2/favicons?domain=https://developers.openai.com\&sz=32)

OpenAI Developers+1



One limitation: instructions are guidance, not an enforcement mechanism. A client or model might not follow them exactly. Therefore, important requirements should also be reflected in tool descriptions and application logic.

## **B. Resources: provide the actual dataset description**

This is where I would put your statistical information.

For example, your MCP server could expose:

```
dataset://current/overview
dataset://current/entity-types
dataset://current/relationships
dataset://current/statistics
```

The first resource could contain:

```
{
  "dataset_name": "University data",
  "description": "Unknown",
  "statistics": {
    "entity_count": 510,
    "entity_type_count": 2,
    "relationship_count": 1200
  },
  "entity_types": [
    {
      "id": "Student",
      "label": "Student",
      "count": 500,
      "attributes": [
        {
          "name": "name",
          "data_type": "string",
          "count": 500,
          "min_cardinality": 1,
          "max_cardinality": 1
        },
        {
          "name": "age",
          "data_type": "integer",
          "count": 480,
          "min_cardinality": 0,
          "max_cardinality": 1
        }
      ]
    },
    {
      "id": "Vorlesung",
      "label": "Vorlesung",
      "count": 10,
      "attributes": [
        {
          "name": "title",
          "data_type": "string",
          "count": 10,
          "min_cardinality": 1,
          "max_cardinality": 1
        }
      ]
    }
  ],
  "relationships": [
    {
      "name": "attends",
      "source_type": "Student",
      "target_type": "Vorlesung",
      "count": 1200
    }
  ]
}
```

This is illustrative JSON, not an MCP protocol message.

The important advantage of resources is that you can expose a relatively large dataset description without putting everything into the tool definitions.

You can also separate structural information from more detailed statistics.

For example, the AI first reads the overview and only requests detailed information about a particular entity type when necessary.

MCP resources can also support subscriptions and update notifications. This is useful if the loaded dataset changes.&#x20;

[image](https://www.google.com/s2/favicons?domain=https://github.com\&sz=32)

GitHub+1



## **C. Tools: let the AI investigate the dataset**

Now comes the actual interaction.

I would initially provide a small number of tools.

| Tool                          | Purpose                                          |
| ----------------------------- | ------------------------------------------------ |
| `get_dataset_overview`        | Retrieve the complete structural summary         |
| `get_entity_type_details`     | Inspect attributes and relationships of one type |
| `get_relationship_statistics` | Inspect relationship frequencies                 |
| `query_entities`              | Retrieve matching entities                       |
| `analyze_relationships`       | Calculate aggregations over relationships        |

Notice that these are domain-independent tools.

They don't need to know whether the underlying data is RDF, JSON, XML or a relational database.

For example, the tool:

`analyze_relationships`

could accept:

```
{
  "source_type": "Student",
  "relationship": "attends",
  "target_type": "Vorlesung",
  "group_by": "target",
  "aggregation": "count",
  "order": "descending",
  "limit": 10
}
```

The result:

```
{
  "groups": [
    {
      "entity": "Vorlesung 1",
      "count": 180
    },
    {
      "entity": "Vorlesung 2",
      "count": 145
    },
    {
      "entity": "Vorlesung 3",
      "count": 120
    }
  ]
}
```

The LLM can now answer:

> Vorlesung 1 is the most frequently attended course, with 180 student-course relationships.

This is much better than asking the LLM to retrieve hundreds of students and count them itself.

**Important:** the tool should count distinct students if the intended metric is the number of students attending a course. Counting relationship triples is not necessarily equivalent if duplicate or qualified relationships are possible.

## **D. Prompts: define reusable workflows**

MCP prompts are especially interesting for your use case.

You could provide prompts such as:

- `understand_dataset`
- `analyze_dataset`
- `find_relationship_patterns`
- `compare_entity_types`
- `identify_data_quality_issues`

For example, the `understand_dataset` prompt could tell the AI:

```
Analyze the currently loaded dataset.

1. Retrieve the dataset overview.
2. Identify the main entity types.
3. Examine the relationships between them.
4. Determine the likely domain of the dataset.
5. Explain the dataset in understandable language.

Do not invent domain semantics.
Distinguish observed structure from inferred meaning.

Present a short summary and mention interesting
questions that could be investigated further.
```

There is one important MCP distinction:

**Prompts are normally user-invoked templates, not automatically executed instructions.**

A compatible client might expose them as slash commands or a prompt menu.&#x20;

[image](https://www.google.com/s2/favicons?domain=https://github.com\&sz=32)

GitHub+1



For automatic behavior after loading, I would use server instructions and dataset resources instead.

# **3. An interesting new possibility: Skills over MCP**

There is actually a relatively new MCP extension that is directly relevant to your question.

It is called **Skills over MCP**, identified by `io.modelcontextprotocol/skills`.

It was accepted as a finalized extension in September 2026.&#x20;

[image](https://www.google.com/s2/favicons?domain=https://github.com\&sz=32)

GitHub+1



It allows an MCP server to distribute Agent Skills to compatible AI clients.

A skill is essentially a directory containing a `SKILL.md` file and optionally supporting documentation and examples.

For your RDF viewer, you could provide:

```
skills/
  data-exploration/
    SKILL.md
    references/
      data-model.md
      statistics.md

  relationship-analysis/
    SKILL.md
    references/
      aggregations.md

  data-quality/
    SKILL.md
```

For example, `data-exploration/SKILL.md`:

```
---
name: data-exploration
description: Understand and explore datasets
---

# Data exploration

You are working with a structured data exploration
application.

## Data model

The application works with:

- Entity types
- Entities
- Attributes
- Relationships

The underlying storage technology is not important
for interpreting the data.

## Workflow

When exploring a new dataset:

1. Retrieve the dataset overview.
2. Identify the entity types.
3. Identify important relationships.
4. Infer the likely domain.
5. Explain the dataset to the user.

Always distinguish facts from assumptions.

## Analysis

Use application tools for statistical calculations.
Do not estimate counts from incomplete samples.
```

The important advantage is that you can distribute the skill together with your MCP server.

The AI client can discover the skill, retrieve it and use its instructions.

Documentation: [Skills over MCP specification](https://skills.extensions.modelcontextprotocol.io/specification/beta/skills?utm_source=chatgpt.com) and [Official GitHub repository](https://github.com/modelcontextprotocol/ext-skills?utm_source=chatgpt.com).

One qualification: this is an optional extension. Support in MCP clients and Rust SDKs is still evolving. I would not make your basic functionality depend on it.

For maximum compatibility, use:

- Server instructions for essential behavior.
- Resources for dataset metadata.
- Tool descriptions for individual operations.
- Skills for richer workflows where supported.

# **4. How should the first interaction after loading a dataset work?**

This is an important detail.

There is a difference between an MCP server providing information and an MCP server initiating a conversation.

MCP does not generally let a server simply insert an unsolicited assistant message into every connected chat.

The host application controls the conversation.

I would therefore implement the following workflow.

1. **User loads a dataset**

   Your application calculates statistics and creates a dataset profile.
2. **MCP server exposes the new profile**

   The resource `dataset://current/overview` becomes available.
3. **Chat retrieves the overview**

   The server instructions tell the AI to retrieve it before answering.
4. **AI explains the dataset**

   The assistant gives the user an understandable summary.
5. **User starts exploring**

   Natural-language questions invoke the appropriate tools.

If you control the chat application, you can go one step further: when the dataset is loaded, send an application event to the chat component and initiate a new assistant turn.

For an external chat client, you cannot assume this behavior. Resource subscriptions and update notifications can inform a client that data has changed, but do not automatically guarantee that it will start a new conversation.

I would also expose a tool:

```
get_current_dataset_overview
```

This gives the AI a reliable way to retrieve the current dataset even if resource discovery or subscriptions are not supported.

# **5. Example: the complete interaction you want**

Let's put the pieces together.

**AI data assistant**

Dataset connected

ASSISTANT · AFTER LOADING

I have analyzed the structure of your dataset.

It contains students and university courses (*Vorlesungen*).

There are 500 students and 10 courses. Students are connected to courses through an attendance relationship.

This appears to be a dataset describing course participation at a university.

You can investigate which courses are most popular, which students attend the most courses, or how participation is distributed.

USER

Which courses are most popular?

Tool: analyze_relationships

**Most popular courses**

Number of distinct students per course

| Course      | Students |
| ----------- | -------- |
| Vorlesung 1 | 180      |
| Vorlesung 2 | 145      |
| Vorlesung 3 | 120      |
| Vorlesung 4 | 95       |
| Vorlesung 5 | 80       |

Result calculated from dataset

ASSISTANT

Vorlesung 1 is the most popular course, with 180 students.

Would you like to see the distribution of course attendance?

Ask a follow-up question...

Illustrative conversation; the numbers are not actual dataset results.

Notice that the AI doesn't need to know SPARQL, RDF or the internal graph implementation to answer this question.

It understands the meaning of the data and delegates the calculation to your application.

# **6. One important addition to your existing statistics**

I would consider extending your current statistics slightly.

You already collect:

- Instance counts by type.
- Attribute types and cardinalities.
- Reference counts.
- Reference types.
- Class names.

I would add a few relationship statistics that make natural-language questions easier.

| Statistic             | Example        | Enables questions                       |
| --------------------- | -------------- | --------------------------------------- |
| Distinct source count | 500 students   | How many students participate?          |
| Distinct target count | 10 courses     | How many courses exist?                 |
| References per source | Average 2.4    | How many courses does a student attend? |
| References per target | Average 120    | How popular are courses?                |
| Min/max cardinality   | 1–5 courses    | Which students attend the most courses? |
| Distribution          | Histogram      | How is participation distributed?       |
| Most frequent targets | Top 10 courses | Which courses are most popular?         |
| Isolated entities     | 15 students    | Which students attend no courses?       |

These statistics can be calculated once after loading the dataset, rather than repeatedly by the LLM.

However, you don't need to calculate every possible aggregation in advance. A generic aggregation tool can calculate them on demand.

Also distinguish between:

- Number of relationship records.
- Number of distinct source entities.
- Number of distinct target entities.

They are not necessarily the same.

# **7. My proposed initial MCP interface**

I would keep the first implementation very small.

Recommended MVP

| MCP component       | Implementation                        |
| ------------------- | ------------------------------------- |
| Server instructions | General data exploration instructions |
| Resource            | `dataset://current/overview`          |
| Resource            | `dataset://current/entity-types`      |
| Resource            | `dataset://current/relationships`     |
| Tool                | `get_current_dataset_overview`        |
| Tool                | `get_entity_type_details`             |
| Tool                | `analyze_relationships`               |
| Tool                | `query_entities`                      |
| Prompt              | `understand_dataset`                  |
| Prompt              | `explore_relationships`               |
| Skill               | `data-exploration`                    |

I would implement the dataset profile in Rust as an independent structure:

```
struct DatasetProfile {
    name: Option<String>,
    entity_types: Vec<EntityTypeProfile>,
    relationships: Vec<RelationshipProfile>,
    statistics: DatasetStatistics,
}

struct EntityTypeProfile {
    id: String,
    label: String,
    count: usize,
    attributes: Vec<AttributeProfile>,
}

struct AttributeProfile {
    name: String,
    data_type: String,
    count: usize,
    min_cardinality: usize,
    max_cardinality: usize,
}

struct RelationshipProfile {
    id: String,
    source_type: String,
    target_type: String,
    count: usize,
}
```

This profile is independent of the RDF implementation and can eventually be generated from JSON, XML or other structured data sources too.

## **8. One final architectural recommendation**

I would actually separate two kinds of knowledge:

**1. Static application knowledge**

Provided by instructions / skills

- What the application does.
- What entity types and relationships mean.
- How to interpret statistics.
- How to use tools.
- Which analysis workflows are available.

**2. Dynamic dataset knowledge**

Provided by resources / tools

- Current dataset.
- Entity types and counts.
- Attributes and cardinalities.
- Relationship structure.
- Statistical distributions.
- Current analysis results.

This separation is particularly valuable if you later introduce different kinds of data sources.

The application knowledge remains unchanged, while the dataset description changes every time a user loads a new file.

**My strongest recommendation:** Start with server instructions, one dataset overview resource and three or four analytical tools. Test this with Claude Desktop or another MCP client before building anything more complicated. You will quickly discover whether the dataset description is understandable enough for an LLM to answer meaningful questions.

And importantly, you can test all of this without implementing an embedded chat or changing your egui interface at all.