from collections import deque, defaultdict

# Graph as adjacency list
#  0 -- 1 -- 3 --  4
#  \-- 2 --/
graph = {
    '0': ['1', '2'],
    '1': ['0', '3'],
    '2': ['0','3'],
    '3': ['1','2','4'],
    '4': ['3'],
}

def brandes_bfs(source, graph):
    # Step 1 — Initialization
    d = {v: -1 for v in graph}                  # distances
    sigma = {v: 0 for v in graph}               # # shortest paths
    P = {v: [] for v in graph}                  # predecessors

    d[source] = 0
    sigma[source] = 1

    Q = deque()                                 # BFS queue
    S = []                                      # stack for later processing

    Q.append(source)

    # Step 2 — BFS
    while Q:
        v = Q.popleft()
        S.append(v)

        for w in graph[v]:
            # If w is found for the first time
            if d[w] < 0:
                Q.append(w)
                d[w] = d[v] + 1

            # If the shortest path to w is via v
            if d[w] == d[v] + 1:
                sigma[w] += sigma[v]
                P[w].append(v)

    return S, d, sigma, P


# Run BFS from source 'A'
S, d, sigma, P = brandes_bfs('0', graph)

# Show results
print("Stack S (BFS visitation order):", S)
print("\nDistances (d):")
for v in d:
    print(f"{v}: {d[v]}")

print("\nShortest path counts (sigma):")
for v in sigma:
    print(f"{v}: {sigma[v]}")

print("\nPredecessors (P):")
for v in P:
    print(f"{v}: {P[v]}")

def dependency_accumulation(S, P, sigma, source, Cb):
    # δ[v] will store dependencies for each node
    delta = {v: 0 for v in P}

    # Process nodes in reverse BFS order
    while S:
        w = S.pop()  # take the last visited node
        for v in P[w]:  # for each predecessor of w
            # Formula from Brandes:
            # δ[v] += (σ[v] / σ[w]) * (1 + δ[w])
            delta[v] += (sigma[v] / sigma[w]) * (1 + delta[w])

        # Don't add the source to its own centrality
        if w != source:
            Cb[w] += delta[w]
    return Cb

Cb = {v: 0 for v in P}

# Run Step 2
Cb = dependency_accumulation(S.copy(), P, sigma, 'A', Cb)

print("Betweenness after source A contribution:")
for v, score in Cb.items():
    print(f"{v}: {score}")

# compute the whole cb for all nodes again

def compute_betweenness_centrality(graph):
    Cb = {v: 0 for v in graph}
    for v in graph:
        S, d, sigma, P = brandes_bfs(v, graph)
        dependency_accumulation(S.copy(), P, sigma, v, Cb)
    return Cb

Cb = compute_betweenness_centrality(graph)
for v, score in Cb.items():
    print(f"{v}: {score}")
