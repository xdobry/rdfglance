import networkx as nx
import community as community_louvain
import matplotlib.pyplot as plt

import louvain

if __name__ == "__main__":
    use_karate = True

    if use_karate:
        complex_edges = [(0, 1), (0, 2), (0, 3), (0, 4), (0, 5), (0, 6), (0, 7), (0, 8), (0, 10), (0, 11), 
                         (0, 12), (0, 13), (0, 17), (0, 19), (0, 21), (0, 31), (1, 2), (1, 3), (1, 7), (1, 13), 
                         (1, 17), (1, 19), (1, 21), (1, 30), (2, 3), (2, 7), (2, 8), (2, 9), (2, 13), (2, 27), 
                         (2, 28), (2, 32), (3, 7), (3, 12), (3, 13), (4, 6), (4, 10), (5, 6), (5, 10), (5, 16), 
                         (6, 16), (8, 30), (8, 32), (8, 33), (9, 33), (13, 33), (14, 32), (14, 33), (15, 32), 
                         (15, 33), (18, 32), (18, 33), (19, 33), (20, 32), (20, 33), (22, 32), (22, 33), 
                         (23, 25), (23, 27), (23, 29), (23, 32), (23, 33), (24, 25), (24, 27), (24, 31), 
                         (25, 31), (26, 29), (26, 33), (27, 33), (28, 31), (28, 33), (29, 32), (29, 33), 
                         (30, 32), (30, 33), (31, 32), (31, 33), (32, 33)]
    else:        
        complex_edges = [
            (0,2),(0,5),(0,3),
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

    all_nodes = set()
    for e in complex_edges:
        all_nodes.add(e[0])
        all_nodes.add(e[1])
    nodes_len = len(all_nodes)


    structure = louvain.Structure(nodes_len, complex_edges)
    structure.louvain(1.0, True)
    print("Origin communities complex:", structure.origin_nodes_community)

    G = nx.Graph()
    G.add_edges_from(complex_edges)

    # Compute the best partition using the Louvain algorithm
    partition = community_louvain.best_partition(G,randomize=False,resolution=1.0)

    print(f"partition {partition}")

    # Draw the graph with nodes colored by their community
    pos = nx.spring_layout(G)
    cmap = plt.cm.get_cmap("viridis", max(partition.values()) + 1)

    fig, axes = plt.subplots(1, 2, figsize=(10, 5))

    nx.draw_networkx_nodes(G, pos, partition.keys(),
                        node_size=300,
                        cmap=cmap,
                        alpha=0.5,
                        node_color=list(partition.values()),ax=axes[0])
    nx.draw_networkx_edges(G, pos, alpha=0.2,ax=axes[0])
    nx.draw_networkx_labels(G, pos, alpha=0.5,ax=axes[0])

    axes[0].set_title("Communities detected by Louvain algorithm")
    my_partition = {}
    for i in range(0,len(structure.origin_nodes_community)):
        my_partition[i] = structure.origin_nodes_community[i]
    print(f"my partition {my_partition}")

    my_cmap = plt.cm.get_cmap("viridis", max(my_partition.values()) + 1)

    nx.draw_networkx_nodes(G, pos, my_partition.keys(),
                        node_size=300,
                        cmap=my_cmap,
                        alpha=0.5,
                        node_color=list(my_partition.values()),ax=axes[1])
    nx.draw_networkx_edges(G, pos, alpha=0.2,ax=axes[1])
    nx.draw_networkx_labels(G, pos, alpha=0.5, ax=axes[1])

    axes[1].set_title("Own algorithm")

    print(f"modularity louvain {community_louvain.modularity(partition,G)}")
    print(f"modularity own {community_louvain.modularity(my_partition,G)}")

    
    plt.tight_layout()
    plt.show()
