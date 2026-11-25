# implemented

- `get_dataset_profile`: Liefert Statistiken, Typen und Eigenschaften des Datensatzes.
- `show_type_instances`: Zeigt Instanzen eines Typs, optional nach Daten-Properties gefiltert.
- `get_type_instance_range`: Liest bis zu 100 Zeilen der zuletzt angezeigten Typentabelle.
- `show_node`: Zeigt einen Knoten anhand seiner IRI.
- `get_node_info`: Liefert Typen, Eigenschaften und Referenzen für einen oder mehrere Knoten.



# tool: load_data

state: open

pass as argument the file location.
Load from file it might be all supported rdf files. Search in code for all supported formats

# tool: reset_data

state: open

call RdfGlanceApp.clean_data() 

# tool: search 

state: open, not implement yet

- open 
- description: full text search for all literals.
- paremters: 
  - search_text: string
  - include iris: boolean
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

state: open

Add node to working graph bei iri (shorten iri) list
There is NodeAction::AddNode, add new AddNodes so many iri-indexes can be handled

## tool: wg_expand

state: open

expand nodes.
parameter:
  - nodes (iris) - in empty all iris
  - types (iris) - expand by type
  - predicates (list or iris and direction tuples) - of empty all predicates. For example {predicate: "influencedBy" direction: standard|reverse}

It correspond already to existing functionality in system

## tool: wg_info

state: open

give info about existing nodes in visual graph
result:
  - node_count: int
  - edge_count: int
  - types: list of all types with count of instances
  - predicates: list of all existing edge predictes with count of edges

# tool: wg_clean

state: open

remove all data from graph

# tool: wg_run_algorith

state: open

run algorithm on working graph - 


# tool: wg_reset_statistics

state: open

# RunQuery

state: to be defined

# Show Table

state: to be defined


  - Add column visibility
  - Add column sort

# GetInstancesInfo

state: to be defined


  - what are current data
  - in tables (columns, rows, type-iri)
