import networkx as nx
import community as community_louvain
import matplotlib.pyplot as plt

# Create a sample graph (Zachary's Karate Club is a classic community detection benchmark)
use_karate = True

if use_karate:
    G = nx.karate_club_graph()
    print(f"edges {G.edges()}")
else:
    edges = [
        (0,2),(0,3),(0,5),
            (1,2),(1,4),(1,7),
            (2,4),(2,5),(2,6),
            (3,7),
            (4,10),
            (5,7),(5,11),
            (6,7),(6,11),
            (8,9),(8,10),(8,11),(8,14),(8,15),
            (9,12),(9,14),
            (10,11),(10,12),(10,13),(10,14),
            (11,13)
    ]

    # Create an undirected graph
    G = nx.Graph()
    G.add_edges_from(edges)

# Compute the best partition using the Louvain algorithm
partition = community_louvain.best_partition(G)

# Print the community assignment for each node
print("Node -> Community mapping:")
for node, comm in partition.items():
    print(f"Node {node}: Community {comm}")

# Draw the graph with nodes colored by their community
pos = nx.spring_layout(G)
cmap = plt.cm.get_cmap("viridis", max(partition.values()) + 1)

nx.draw_networkx_nodes(G, pos, partition.keys(),
                       node_size=300,
                       cmap=cmap,
                       node_color=list(partition.values()))
nx.draw_networkx_edges(G, pos, alpha=0.5)
nx.draw_networkx_labels(G, pos)

plt.title("Communities detected by Louvain algorithm")
plt.show()