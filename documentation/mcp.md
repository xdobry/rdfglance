# RDFGlance and MCP

## What is MCP, and what does it add to RDFGlance?

The [Model Context Protocol (MCP)](https://modelcontextprotocol.io/) is an open
standard that lets AI clients connect to external tools and data sources. With
RDFGlance's MCP server, a compatible chat assistant can inspect the dataset
currently open in RDFGlance and use its tables and node browser in response to
natural-language requests.

This is useful when you want to ask questions about a dataset without first
writing queries or manually searching through tables. The assistant can read
the dataset profile, find types and properties, filter instances, and inspect
nodes. RDFGlance remains responsible for displaying the data; MCP connects the
assistant to selected RDFGlance features.

## What the server provides

- `get_dataset_profile` and the `dataset://current/profile` resource provide
  dataset counts, types, properties, and reference information.
- `show_type_instances` opens a type's instances in the table view and can
  apply filters to data properties.
- `get_type_instance_range` reads rows from the most recently shown table
  (at most 100 per call).
- `show_node` opens a node in the browse view; `get_node_info` returns its
  types, properties, and references.

The server operates on the dataset open in the RDFGlance desktop app. It is
not a general-purpose SPARQL endpoint.

## Build and configure with Claude Code

Build the MCP executable from the repository root:

```powershell
cargo build --package rdf-glance-desktop-mcp --release
```

Create a project-level `.mcp.json` in the folder where you run Claude Code.
Replace the example paths with the actual locations on your computer:

```json
{
  "mcpServers": {
    "rdfglance": {
      "type": "stdio",
      "command": "C:\\path\\to\\rdfglance\\target\\release\\rdf-glance-mcp.exe",
      "args": [
        "C:\\path\\to\\rdfglance\\sample-rdf-data\\programming_languages.ttl"
      ]
    }
  }
}
```

The file argument is optional. When provided, RDFGlance opens that dataset
when the client starts the server; otherwise, use the dataset already open in
RDFGlance. Claude Code may ask you to approve a project-level MCP server
before connecting. See the [Claude Code MCP guide](https://docs.anthropic.com/en/docs/claude-code/mcp)
for details.

## Configure in VS Code

VS Code uses `.vscode/mcp.json` and a `servers` property rather than the
`.mcp.json` / `mcpServers` format above. Use this equivalent configuration,
again replacing the example paths:

```json
{
  "servers": {
    "rdfglance": {
      "type": "stdio",
      "command": "C:\\path\\to\\rdfglance\\target\\release\\rdf-glance-mcp.exe",
      "args": [
        "C:\\path\\to\\rdfglance\\sample-rdf-data\\programming_languages.ttl"
      ]
    }
  }
}
```

See the [VS Code MCP documentation](https://code.visualstudio.com/docs/copilot/chat/mcp-servers)
for connecting to MCP servers and configuring other clients.

## Example conversation

For example, with a dataset loaded, a user could ask:

> **User:** What kinds of things are in this dataset?
>
> **Assistant:** I’ll check the dataset profile.
> *(Calls `get_dataset_profile` and summarizes the types and counts it returns.)*
>
> **User:** Show me the first ten instances of the programming-language type.
>
> **Assistant:** I found the type in the profile and will open its instances.
> *(Calls `show_type_instances` with the type IRI from the profile, then calls
> `get_type_instance_range` for rows 0–10.)*
>
> **Assistant:** The table is open in RDFGlance. Here are the first ten rows
> and the properties available in the results.

The assistant should use the type IRI from the profile, not guess an identifier
from its display label. It can also use filters or inspect a particular node
when asked.

## Other launch options

The MCP server uses stdio by default. You can also run it directly:

```powershell
.\target\release\rdf-glance-mcp.exe
```

To use Streamable HTTP instead:

```powershell
.\target\release\rdf-glance-mcp.exe --transport streamable-http --host 127.0.0.1 --port 8080
```

The HTTP endpoint is `/mcp` and has no authentication. Bind to `127.0.0.1`
unless other machines are meant to access it; binding to `0.0.0.0` exposes it
on all network interfaces.

To test the stdio transport with the MCP Inspector (with a debug build):

```powershell
npx @modelcontextprotocol/inspector .\target\debug\rdf-glance-mcp.exe
```
