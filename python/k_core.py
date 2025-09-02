from collections import defaultdict

def compute_k_core(nodes, edges):
    # Build adjacency list
    adj = defaultdict(set)
    for u, v in edges:
        adj[u].add(v)
        adj[v].add(u)

    degree = {node: len(adj[node]) for node in nodes}
    core_number = dict()

    # Nodes not yet processed
    remaining = set(nodes)

    changed = True
    k = 1
    while remaining:
        changed = True
        while changed:
            changed = False
            to_remove = []
            for node in remaining:
                if degree[node] < k:
                    to_remove.append(node)
            if to_remove:
                changed = True
                for node in to_remove:
                    remaining.remove(node)
                    core_number[node] = k - 1
                    for neighbor in adj[node]:
                        if neighbor in remaining:
                            degree[neighbor] -= 1
        k += 1

    # Remaining nodes get highest k-core
    for node in remaining:
        core_number[node] = k - 1

    return core_number

# Example graph
nodes = [0,1,2,3,4,5,6]
edges = [
    (0,1), (0,2), (1,2),  # triangle → 3-core
    (2,3), (3,4), (4,5),  # chain → 2-core
    (5,6),                  # leaf → 1-core
]

k_core = compute_k_core(nodes, edges)

# Print results
print("Node : k-core centrality")
for node in sorted(nodes):
    print(f"{node} : {k_core[node]}")