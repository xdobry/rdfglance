# Open Points

* Show referenced object in place (ui collapsing area)
* Load object from sparql endpoint
    * load models from named graph
    * show and select named graph
* Load labels from types and props (show iris as tool tip)
* Copy iri to clip board
* Show property arrays
* Show formated boolean, number and language string

# Open Points Visual Graph

* show nodes
   * V frame
   * rect at position
   * V click event
   * V marked (outline)
   * V drag
   * V double click
   * pop down menu
      * dismiss
      * show relations
   * V show relations
   * set up layout factors
* V open nodes (per context)
* V close nodes
* V expand all visible nodes
* show detail side bar with functionality
  * hide node
  * close all nodes of this type
  * close all other
  * expand
  * expand relation
  * show that hidden
  * select label attribute 
* relation
  * hide relation
  * hide all relations of this type
* highlight all edges from and to selected
* safe visible and cache state
* show cache statistics
* purge cache
* show all in cache
* load all into cache
* load all avaiable data
* sparql construct

# Model Support

* Load all found ontologies
* Load models with classes, properties


#######################
ToDo
- zoom
- V choose properties for instance display
- V select references to expand in detail property view
- V filter out unwanted languages during loading (optional, configuration)
- V sortierung für typen
- label für typen
- V sortierung für instanzen
- V filter für instanzen
- schnelles laden merke letzte iri
- auswahl der properties für instanz table
- V sortierung der properties für table
- abschneiden der properties wenn mehrere zeilen
- label management (lade labels aus iri, lade ontology)
- V show iri short
- handle many types (performance see yago)
- handle instances without type (special empty type?)
- graph actions
    - hide disconnected
    - hide not direct connected
- V relations - hide/show relations
- expand (partial layout)
- layout (animated layout)
- V change layout parmeters (relation force, node force)
- V expand all references of type
- search connection between nodes (open and search)
- visual graph 
   - V hide labels
   - show label/iri on hoover

# Own Grid
- move with mouse
- move columns
- show index of cell
(DB Browser) 
- paging/navigaition


# Functionality

Table
- export, import (csv, json, rdf)

# First Release
- (disabled) loading per sparql endpoint (connect to known databases)
- V layout animation (check box)
- prefix manager
   - automatically create prefixes
- visual-graph show references with labels
  - node:context hide referenced
- V handle objects without class
- V handle rdf lists
- V handle blank nodes
- V move graph using mouse drag
- V resize table cells
- V compute layout ausschalten wenn zu wenig bewegung
- V control-F focus on search
- limit count of shown nodes (show warning)

- zoom for graph view
- hint user how use graph view (double click and expand)
- resize iri column
- filter for types
- expand nodes (add in circle to the existing expanding node to simplify layout)
- windows icon
- windows no cmd terminal window
- serialization

#####################################
- sprachen
- type/predicate iri labels
- prefix manager
- list
- browser
- sparql browser
- kann die App eine Viewer für dbdata and co. benutzt werden

- (wenn immer prefix manager benutzt wird, dann braucht man keine iri short)
- labels nachladen kann man als eine Aktion (wenn sparql)

###########################
#DBPedia

- Lade nich zu viel (Welche Props sollen geladen werden)
 (z.B nicht alle Typen)
- Ist es in named graph unterteilt
- nur eine Sprache laden
- nur bestimmte verweise laden
(Das alles Konfigurierbar)

###############################
#Zustand Abspeichern

- Serialisierung (Ist es schneller als RDF)
- Auch zustand der Graphen

