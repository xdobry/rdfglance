import networkx as nx
import matplotlib.pyplot as plt

def draw_graph(G,pos,ax,title):
    nx.draw_networkx_nodes(G, pos, node_size=300,ax=ax)
    nx.draw_networkx_edges(G, pos, alpha=0.5,ax=ax)
    nx.draw_networkx_labels(G, pos, alpha=0.5,ax=ax)
    ax.set_title(title)

fig, axes = plt.subplots(1, 2, figsize=(10, 5))

G = nx.karate_club_graph()
pos = nx.spring_layout(G)

draw_graph(G,pos,axes[0],"spring")

pos_spec = nx.spectral_layout(G)
draw_graph(G,pos_spec,axes[1],"spectral")


plt.show()