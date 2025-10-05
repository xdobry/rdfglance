from rdflib import Graph, Namespace, URIRef, Literal
from rdflib.namespace import RDF, RDFS, FOAF, XSD
import time

# ------------------------
# 1. Load TTL Data
# ------------------------
start = time.time()

g = Graph()
g.parse("../olympics.ttl", format="ttl")

end = time.time()
print("Execution time:", end - start, "seconds")

# Executiontime 72 seconds

