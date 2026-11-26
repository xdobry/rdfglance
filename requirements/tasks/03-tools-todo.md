# implemented

- `get_dataset_profile`: Liefert Statistiken, Typen und Eigenschaften des Datensatzes.
- `show_type_instances`: Zeigt Instanzen eines Typs, optional nach Daten-Properties gefiltert.
- `get_type_instance_range`: Liest bis zu 100 Zeilen der zuletzt angezeigten Typentabelle.
- `show_node`: Zeigt einen Knoten anhand seiner IRI.
- `get_node_info`: Liefert Typen, Eigenschaften und Referenzen für einen oder mehrere Knoten.

# general idea

- every tool can pass sugestions, limitation for futher actions
- nach dem Laden (wenn sehr viele Elemente)
- zu viele Elemente in working graph (Layout ausblenden)
- brauche ich type_instances wenn ich query habe (type instances is einfacher)
- load_data (content in body)


# tool: load_data

state: implemented

pass as argument the file location.
Load from file it might be all supported rdf files. Search in code for all supported formats

# tool: reset_data

state: implemented

call RdfGlanceApp.clean_data() 

# tool: search 

state: implemented

- open 
- description: full text search for all literals.
- paremters: 
  - search_text: string
  - max_result: int

It should return list of nodes
iri, types, property (in which value was found), value

It is not implemented in the app.
The idea is to search directly in indexes for text (So short string and long string) indexes and after it 
go throw all nodes and find which nodes use the indexes. It should use rayon parallisation. for this 2 steps if possible

# WorkingGraph Tool

Working graph correspond to app RdfGlanceApp.visible_nodes So it is the visual grap cache that is used also to run graph algorithms
on it.

## tool: wg_add_nodes

state: implemented

Add node to working graph bei iri (shorten iri) list
There is NodeAction::AddNode, add new AddNodes so many iri-indexes can be handled

## tool: wg_expand

state: implemented

expand nodes.
parameter:
  - nodes (iris) - in empty all iris
  - types (iris) - expand by type
  - predicates (list or iris and direction tuples) - of empty all predicates. For example {predicate: "influencedBy" direction: standard|reverse}

It correspond already to existing functionality in system

## tool: wg_info

state: implemented

give info about existing nodes in visual graph
result:
  - node_count: int
  - edge_count: int
  - types: list of all types with count of instances
  - predicates: list of all existing edge predictes with count of edges

## tool: wg_clean

state: implemented

remove all data from graph

## tool: wg_run_algorith

state: implemented

run algorithm on working graph - 


## tool: wg_reset_statistics

state: implemented

# App Zustand abfragen

## tool: get_app_status

- was wird angezeigt
- was wurde selektiert
- vielleicht einstellungen (geladenen Datensätze usw)

## tool: export_screenshot

# Visual Query

Run visual query from agent.
Pass query structure as json (corresponding to TableQuery).
Sonst sehr ählich zu table (type_instances)
Sortieren, Export (CSV), Range, Columns


## tool: run_query

state: to be defined

# Type Instances

state: to be defined

  - Add column visibility
  - Add column sort

## tool: type_instances_info

Show the state of type instances (what type is selected, what columns, how many rows)

state: to be defined

  - what are current data
  - in tables (columns, rows, type-iri)

  ## tool: type_instances_export

  state: not implemented

  export table to csv as file

  # 
