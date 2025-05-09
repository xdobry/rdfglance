# General Information

At this time the application may not be self-explained. So here some screenshot with additional information.

First you should load some rdf data. The application supports ttl (turtle) and rdf/xml formats.
There is some sample rdf data in the [sample-rdf-data](../sample-rdf-data/programming_languages.ttl) directory

After it you may chose 3 possible note tabs: Table, Visual Graph and Browser
The good point is to start is table view.

# Table Tab

![screenshot](screeshots/table.png)

RDFGlance sort all nodes by types.
First you can see some statistics of RDF Date.

Then you can see the list of all types with some statistic
- count of all instances of this type
- count of unique data properties of all instances of this type
- count of unique object properties of all instances of this type
- count of unique referenced object properties of all instances of this type

Remember that in rdfs an instance can have multiple types. 

After selecting the type you can see all instances as a table.
You can sort the instances by some data property.
The out/in columns shown the count of outgoing and ingoing edges (object properties).
By clicking of the out/in cell you can browser all references in pop-up windows.

You can click on the cell to see the whole value of the data property or other values of the same predicate.
Remember in rdf one node can have multiple objects of same predicate. 

# Browser Tab 

![screenshot](screeshots/browser.png)

In the browser you can see all properties of one node.

Following information are show:

- type of the node (as label). You can click the type to change to table view and see all instances of clicked type
- all data properties
- all object properties (references)
- all objects that reference this node (referenced by)

# Visual Graph

![screenshot](screeshots/visual-graph.png)

Visual graph shows nodes and edges (relations) as visual graph.
It is intended to manipulate the relations (hide, unhide, expand) to discover relations
in the data.

You can manipulate the graph by clicking the nodes.
Double click will expand all outgoing or incoming relation of the node.

On the right side you can see all data properties and relation of the node.
You can also use the relation buttons to expand chosen relation.
You may hide or unhide some relaiton type or expand some relation type for all visible nodes.


