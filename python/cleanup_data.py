from rdflib import Graph, Namespace, URIRef, Literal
from rdflib.namespace import RDF, RDFS, FOAF, XSD

# ------------------------
# 1. Load TTL Data
# ------------------------
g = Graph()
g.parse("sample-rdf-data/programming_languages.ttl", format="ttl")
prefixes = dict(g.namespaces())

print(f"Loaded {len(g)} triples from data.ttl")

def add_missing_types(g, predicate, object_type):       
    # ------------------------
    # 2. SPARQL Query
    # ------------------------
    query = f"""
    PREFIX dbo: <http://dbpedia.org/ontology/>
    PREFIX dbp: <http://dbpedia.org/property/>
    PREFIX dbr: <http://dbpedia.org/resource/>
    PREFIX yago: <http://dbpedia.org/class/yago/>
    SELECT distinct ?pl
    WHERE {{
        ?l {predicate} ?pl.
        FILTER isIRI(?pl)
        filter not exists {{?pl a {object_type}}}
    }}
    """

    print(f"Adding missing types for {predicate} with class {object_type}")

    for row in g.query(query):
        print(f"object: {row.pl}")
        g.add((row.pl, RDF.type, URIRef(object_type)))


add_missing_types(g, "dbo:influencedBy", "dbo:ProgrammingLanguage")
add_missing_types(g, "dbp:paradigm", "yago:Paradigm113804375")
add_missing_types(g, "dbo:designer", "dbo:Person")

g.serialize("sample-rdf-data/programming_languages2.ttl", format="ttl")
print("Updated graph saved to sample-rdf-data/programming_languages2.ttl")