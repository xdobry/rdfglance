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

# tool: take_screen_shot()

state: not implemented

parameter: target_file_localtion

it should make the png screen of the whole application.


```
if ui.button("Take screenshot").clicked() {
    ui.ctx().request_screenshot(|image| {
        let result = ::image::save_buffer(
            "screenshot.png",
            image.as_raw_rgba(),
            image.size[0] as u32,
            image.size[1] as u32,
            ::image::ColorType::Rgba8,
        );

        if let Err(err) = result {
            eprintln!("Screenshot failed: {err}");
        }
    });
}
```


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

## tool: switch_display

state: not implemented

parameter: display_type enum according to DisplayType

It should switch app diaplay to the types defined by DisplayType enum
So set RdfGlanceApp.display_type to according enum value


## tool: wg_add_nodes

state: implemented

Add node to working graph bei iri (shorten iri) list
There is NodeAction::AddNode, add new AddNodes so many iri-indexes can be handled

## tool: wg_add_type_instances

state: implemented

Add all instances of a type to the working graph, optionally filtered by data properties.

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
  - visible_edge_count and hidden_edge_count
  - per predicate: total, visible, and hidden edge counts
  - hidden_predicates: currently hidden predicate IRIs

## tool: wg_set_hidden_predicates

state: implemented

Replace the hidden predicate list used by the app. An empty list makes all predicates visible.

## tool: wg_clean

state: implemented

remove all data from graph

## tool: wg_run_algorithm

state: implemented

Run the selected statistic algorithm on the working graph. The `algorithm`
parameter uses the same enum as the `statistic` parameter of
`get_statistics_data`. Return row count and available statistic kinds only;
retrieve concrete values with `get_statistics_data`.


## tool: wg_reset_statistics

state: implemented

## tool: statistics_info

state: implemented

Show the available row count and statistic kinds. Does not return concrete
values; use `get_statistics_data` for those.

## tool: get_statistics_data

state: implemented

Return concrete per-node statistic values, sorted by value.

parameters:
  - statistic: shared enum also used by `wg_run_algorithm`
  - order: highest|lowest (highest is default)
  - limit: maximum number of values (100 is default)


## tool: export_statistics_data

state: not implemented

Export statistics data to csv. Same as existing csv statistic export.

# App Zustand abfragen

## tool: get_app_status

state: not defined

- was wird angezeigt
- was wurde selektiert
- vielleicht einstellungen (geladenen Datensätze usw)

## tool: export_screenshot

# Visual Query

Run visual query from agent.
Pass query structure as json (corresponding to TableQuery).
Sonst sehr ählich zu table (type_instances)
Sortieren, Export (CSV), Range, Columns


## tool: query_run

state: to be implemented

parameter: json and (cleaned from ui) version TableQuery. So use shorten iri for types and predicates.
For columns transport only visible columns if (empty all column).
The app should build visual query, run it and switch to visual query tab.
The TableQuery need to be checked if all types and predicates (for the type so use type_index) exists
If errors are found the error message should describe the exact reason

return - 

row_count: int
columns (tuples of type and predicate)

## tool: query_info

state: to be implemented

return 
row_count: int
column: (type of type and predicate)
query: the same structure as input parameter from TableQuery


## tool: query_reset

state: to be implemented

reset the query and drop all result

## tool: query_export

state: to be implemented

Return query result table as csv. If not query exists return error that query_run must be used first

## tool: query_get_range

state: to be implemented

Return the row range from the query result (see also get_type_instance_range)




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

 
