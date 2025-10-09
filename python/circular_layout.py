import networkx as nx
import csv
import matplotlib.pyplot as plt
import math
import itertools
import random
from typing import List, Tuple, Dict, Iterable

# Evaluate problem of optimal ordering for cicural layout using different methods
# The optimal criteria is edge length
# The finding of optimal order by bruce force is n! so it is possible only till 8-10 nodes

# Open points
# 1) For sparse graph - the graph could be devided into sub graphs using articulation points. Find components first and handle it seperatelly
#    The problem is that connecting edge is not using im optimization
# 2) Use genetic algrithms (also for minimizing crossing points)
#    - gain function is clear
#    - mutation (just swaping edges)
#    - how combine ordering, find best common suborders and preserve them
# 3) Try sequential placement (greedy)
#    - how start (choose start node) - should be central
#    - choose the longest path first
#    - can the fidler vactor be usefull to choose the branch
#
# Finding
# TSP - can not find the optima
# 2-opt (swapping 2 nodes randomly) - can improve but need good start
#

def load_example_data():
    G = nx.DiGraph()     # Directed graph

    # Open and read the CSV
    with open("edges.csv", newline="") as f:
        reader = csv.DictReader(f)  # Reads columns by name: "source", "target"
        for row in reader:
            src = int(row["source"])
            tgt = int(row["target"])
            G.add_edge(src, tgt)

    print(f"Loaded {G.number_of_nodes()} nodes and {G.number_of_edges()} edges")
    return G

def edges_len(G, pos, nodes, debug=False):
    edges_len = 0.0
    for node_idx, node in enumerate(nodes):
        from_pos = pos[node_idx]
        for edge in G.edges(node):
            to_idx = nodes.index(edge[1])
            to_pos = pos[to_idx]
            distance = math.sqrt((to_pos[0] - from_pos[0])**2 + (to_pos[1] - from_pos[1])**2)
            if debug:
                print(f"{node}-{edge[1]} {distance}")
            edges_len += distance
    return edges_len

def get_tsp_graph(G,penalty: float = 20.0):
    nodes = list(G.nodes())
    tsp_graph = nx.Graph()

    for u in nodes:
        for v in nodes:
            if u == v:
                continue
            if G.has_edge(u, v) or G.has_edge(v, u):
                dist = 1.0
            else:
                dist = penalty
            tsp_graph.add_edge(u, v, weight=dist)

    return tsp_graph

def optimal_order_using_tsp(G):
    tsp_G = get_tsp_graph(G)
    print("start tsp")
    order = nx.approximation.traveling_salesman.christofides(tsp_G, weight="weight")
    #order = nx.approximation.traveling_salesman.greedy_tsp(tsp_G, weight="weight")
    print("end tsp")
    order.pop()
    return order

def optimal_order(G,pos_list):
    min = 100000
    min_order = None
    i = 0

    for order in itertools.permutations(range(1,len(G))):
        if i % 1000 == 0:
            print(f"iter {i}")
        i = i+1
        forder = (0,) + order
        elen = edges_len(G,pos_list,forder)
        if elen<min:
            min = elen
            min_order = forder
    return min_order

def draw_graph(G,pos,ax,title):
    nx.draw_networkx_nodes(G, pos, node_size=300,ax=ax)
    nx.draw_networkx_edges(G, pos, alpha=0.5,ax=ax)
    nx.draw_networkx_labels(G, pos, alpha=0.5,ax=ax)
    ax.set_title(title)

def draw_graph_order(G,pos_list,order,ax,title):
    opos = {}
    for i,n in enumerate(order):
        opos[n] = pos_list[i]
    draw_graph(G,opos,ax,title)

def circular_distance_by_index(i: int, j: int, n: int) -> int:
    """Shortest circular distance (#steps) between positions i and j on n-cycle."""
    d = abs(i - j)
    return min(d, n - d)

def circular_cost(order: List, edges: Iterable[Tuple], n: int) -> float:
    """
    Compute the true circular layout cost for 'order'.
    order : list of nodes (length n)
    edges : iterable of (u, v) or (u, v, w)
    n : number of nodes
    """
    pos = {node: idx for idx, node in enumerate(order)}
    total = 0.0
    for e in edges:
        if len(e) == 3:
            u, v, w = e
            weight = w
        else:
            u, v = e
            weight = 1.0
        i, j = pos[u], pos[v]
        dist = circular_distance_by_index(i, j, n)
        total += weight * dist
    return total

def node_cost(order, node, adj_map):
    total = 0.0
    edge_pos = order.index(node)
    n = len(order)
    for edge in adj_map[node]:
        total += circular_distance_by_index(edge_pos,order.index(edge),n)
    return total

def two_opt_improve(order: List, edges: Iterable[Tuple], max_iter: int = 1000) -> List:
    """
    Simple 2-opt style local search that tries swapping two nodes (positions)
    and keeps swaps that reduce the true circular cost.
    """
    adj_map = {}
    for u, v in edges:
        adj_map.setdefault(u, []).append(v)
        adj_map.setdefault(v, []).append(u) 

    n = len(order)
    best_order = list(order)
    # best_cost = circular_cost(best_order, edges, n)
    improved = True
    it = 0
    while improved and it < max_iter:
        improved = False
        it += 1
        # randomize scan to escape deterministic cycles
        positions = list(range(n))
        random.shuffle(positions)
        for i in positions:
            for j in range(i+1, n):
                # swap positions i and j
                curr_cost = node_cost(best_order,i,adj_map) + node_cost(best_order,j,adj_map)
                cand = list(best_order)
                cand[i], cand[j] = cand[j], cand[i]
                # cand_cost = circular_cost(cand, edges, n)
                cand_cost = node_cost(cand,i,adj_map) + node_cost(cand,j,adj_map)
                if cand_cost < curr_cost - 1e-12:
                    best_order = cand
                    improved = True
                    # break to restart scanning from new improved order
                    break
            if improved:
                break
    return best_order

def spectral_order(G):
    fiedler_vector = nx.fiedler_vector(G)
    spectral_order = [node for _, node in sorted(zip(fiedler_vector, G.nodes()))]
    return spectral_order

def seq_placement(G):
    adj_map, min_node = gen_adj_start_node(G)
    # order = [n for n in graph_iterator(adj_map,min_node,set())]
    visited = set()
    stack = [min_node]
    order = []

    while stack:
        node = stack.pop()
        if node not in visited:
            visited.add(node)
            order.append(node)
            for n in adj_map[node]:
                if n not in visited:
                    stack.append(n)

    return order


def graph_iterator(adj_map,node,visited):
    yield node
    visited.add(node)
    for n in adj_map[node]:
        if n not in visited:
            yield from graph_iterator(adj_map,n,visited)

def all_dfs_orders_my(adj_map,start):
    """
    All posible dfs deep first search
    Because of back references the generated proposal are not unique
    but for sparse diagram much smaller than all order permutations

    the amount is multiply of all factor degress 

    Perhaps can be reduced by not trying all permutations of posible edges but only trying each edge as first position once

    But I suppose that the optimal order is one of the dfs orders (to be proved but seems to be obvious by looking at it)
    """
    def inner_diff(visited, stack):
        order = []
        while stack:
            node = stack.pop()
            if node not in visited:
                visited.add(node)
                order.append(node)
                targets = [n for n in adj_map[node] if n not in visited]
                if len(targets)<=1:
                    for n in adj_map[node]:
                        stack.append(n)
                else:
                    for nstack in itertools.permutations(targets):
                        sub_stack = stack.copy() + list(nstack)
                        for rest_order in inner_diff(visited.copy(),sub_stack):
                            yield order + rest_order
                    return
        yield order
    
    return list(inner_diff(set(), [start])) 

def best_seq_placement(G):
    adj_map, min_node = gen_adj_start_node(G)
    edges = G.edges()
    
    min = 100000
    min_order = None
    i = 0
    nodes_len = len(G)

    for order in all_dfs_orders_my(adj_map, min_node):
        elen = circular_cost(order, edges, nodes_len)
        print(f"order {order} cost {elen}")
        if elen<min:
            min = elen
            min_order = order
    return min_order


def random_dfs(adj_map, start_node):
    visited = set()
    stack = [start_node]
    order = []
    while stack:
        node = stack.pop()
        if node not in visited:
            visited.add(node)
            order.append(node)
            targets = adj_map[node]
            for n in random.sample(targets,len(targets)):
                if n not in visited:
                    stack.append(n)
    return order

def gen_adj_start_node(G):
    edges = G.edges()
    adj_map = {}
    for u, v in edges:
        adj_map.setdefault(u, []).append(v)
        adj_map.setdefault(v, []).append(u)
    min_node = None
    min_neighbors = None
    for node , neighbors in adj_map.items():
        if min_node is None or len(neighbors)<min_neighbors:
            min_node = node
            min_neighbors = len(neighbors)
            if min_neighbors == 1:
                break
    return adj_map, min_node

def circular_distance_by_index(i: int, j: int, n: int) -> float:
    """Helper for circular distance between two indices."""
    d = abs(i - j)
    return min(d, n - d)

def circular_cost_crossing(order: List, edges: Iterable[Tuple], n: int) -> float:
    """
    Compute the circular layout cost for 'order':
    - edge length (shorter is better)
    - +1 penalty per crossing
    """
    pos = {node: idx for idx, node in enumerate(order)}
    total = 0.0

    # Convert edges to unified (u,v,weight) form
    edge_list = []
    for e in edges:
        if len(e) == 3:
            u, v, w = e
        else:
            u, v, w = e[0], e[1], 1.0
        edge_list.append((u, v, w))
        total += w * circular_distance_by_index(pos[u], pos[v], n)

    # --- Edge crossing penalty ---
    crossings = 0
    for i in range(len(edge_list)):
        u1, v1, _ = edge_list[i]
        a, b = sorted((pos[u1], pos[v1]))
        for j in range(i + 1, len(edge_list)):
            u2, v2, _ = edge_list[j]
            c, d = sorted((pos[u2], pos[v2]))
            # edges (a,b) and (c,d) cross if endpoints alternate
            if (a < c < b < d) or (c < a < d < b):
                crossings += 1

    total += crossings  # 1 unit penalty per crossing
    return total

def crossover(parent1, parent2):
    """Order crossover (OX) for permutations."""
    size = len(parent1)
    a, b = sorted(random.sample(range(size), 2))
    child = [None] * size
    child[a:b] = parent1[a:b]
    fill = [x for x in parent2 if x not in child]
    j = 0
    for i in range(size):
        if child[i] is None:
            child[i] = fill[j]
            j += 1
    return child

def mutate(individual, mutation_rate=0.1):
    """Swap two elements with a certain probability."""
    ind = individual[:]
    for i in range(len(ind)):
        if random.random() < mutation_rate:
            j = random.randrange(len(ind))
            ind[i], ind[j] = ind[j], ind[i]
    return ind

def select(population, fitnesses):
    """Tournament selection."""
    k = 3
    selected = random.sample(list(zip(population, fitnesses)), k)
    selected.sort(key=lambda x: x[1], reverse=False)
    return selected[0][0]

def genetic_opt(G, 
    population_size=100,
    generations=200,
    crossover_rate=0.8,
    mutation_rate=0.1,):
    adj_map, start_node = gen_adj_start_node(G)
    edges = G.edges()
    nodes_len = len(G)
    population = [random_dfs(adj_map,start_node) for _ in range(population_size)]

    for gen in range(generations):
        # Evaluate fitness
        fitnesses = [circular_cost_crossing(ind,edges,nodes_len) for ind in population]
        best = min(zip(population, fitnesses), key=lambda x: x[1])

        print(f"Gen {gen}: best fitness = {best[1]:.4f}")

        # --- Create next generation
        new_population = []
        while len(new_population) < population_size:
            parent1 = select(population, fitnesses)
            parent2 = select(population, fitnesses)
            # Crossover
            if random.random() < crossover_rate:
                child = crossover(parent1, parent2)
            else:
                child = parent1[:]
            # Mutation
            child = mutate(child, mutation_rate)
            new_population.append(child)

        population = new_population

    # Return the best solution found
    fitnesses = [circular_cost_crossing(ind,edges,nodes_len) for ind in population]
    best = min(zip(population, fitnesses), key=lambda x: x[1])
    return best
    

G = load_example_data()
pos = nx.circular_layout(G)
pos_list = []
for n in list(G):
    pos_list.append(pos[n])

nodelist = list(range(len(G)))
print(f"edges_len {edges_len(G,pos,nodelist,True)}")

fig, axes = plt.subplots(1, 7, figsize=(10, 5))

draw_graph(G,pos,axes[0],"random layout")

min_order = optimal_order(G, pos_list)
print(f"min_order {min_order} {edges_len(G,pos_list,min_order,True)}")
draw_graph_order(G,pos_list,min_order,axes[1],"Optimal")

tsp_order = optimal_order_using_tsp(G)
print(f"tsp order {tsp_order} {edges_len(G,pos_list,tsp_order)}")
draw_graph_order(G,pos_list,tsp_order,axes[2],"tsp solution")

#improved_order = two_opt_improve([0,1,2,3,4,5,6,7], G.edges())
improved_order = two_opt_improve(tsp_order, G.edges())
print(f"rand gen {improved_order} {edges_len(G,pos_list,improved_order)}")
draw_graph_order(G,pos_list,improved_order,axes[3],"2-opt")

H = G.to_undirected()
print(f"articulation points {list(nx.articulation_points(H))}")

s_order = spectral_order(H)
print(f"spectral order {s_order} {edges_len(G,pos_list,s_order)}")
draw_graph_order(G,pos_list,s_order,axes[4],"spectral order")

place_order = best_seq_placement(G)
print(f"place order {place_order} {edges_len(G,pos_list,place_order)}")
draw_graph_order(G,pos_list,place_order,axes[5],"placed")

gen_order = genetic_opt(G)[0]
print(f"genetic order {place_order} {edges_len(G,pos_list,gen_order)}")
draw_graph_order(G,pos_list,gen_order,axes[6],"genetic")

plt.show()



