import networkx as nx
# https://en.wikipedia.org/wiki/Zachary%27s_karate_club

G = nx.karate_club_graph()

with open("sample-rdf-data/karate_graph.ttl", "w") as f:
    f.write("@prefix ex: <http://rdfglance.karate_club#> .\n")
    f.write("@prefix foaf: <http://xmlns.com/foaf/0.1/> .\n\n")
    f.write("# Zachary's Karate Club graph see  https://en.wikipedia.org/wiki/Zachary%27s_karate_club\n")
    f.write("# Used to test community detection with louvain algorithm\n\n")

    for node_id, node in G.nodes(data=True):
        print(node_id)
        print(node)
        f.write(f"ex:{node_id} a ex:Person ; ex:club \"{node.get("club")}\" .\n")

    for u, v in G.edges():
        f.write(f"ex:{u} foaf:knows ex:{v} .\n")        
