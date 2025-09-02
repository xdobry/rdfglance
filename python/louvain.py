

from collections import defaultdict
import random
from typing import List, Dict
import math

# This is own implementation of Louvain
# I have tried many python or rust libraries, but either they was naive implmented and slow or
# do not compute right modularity
# The implementation was initially inspired by gephi but later rewritten for rust
# and adapted the q_delta calculation to really match the modularity delta changes
#
# The rust implementation was written after the python and is compatible
# The python implementation has very detailed tests for modularity and q_calculation


class Community:
    def __init__(self, id: int, node: int):
        self.id = id
        self.nodes = [node]
        self.total_degree = 0.0

    def add_node(self, node):
        self.nodes.append(node)

    def remove_node(self, node):
        self.nodes.remove(node)

class Structure:
    def __init__(self, nodes_len,edges: List[tuple[int,int]]):
        self.communities: List[Community] = [Community(i,i) for i in range(0,nodes_len)]
        # node to node connections (undirected)
        self.edges : Dict[int,(int,float)] = defaultdict(list)
        self.orig_edges = defaultdict(list)
        self.m = len(edges) * 2.0
        self.node_community = []
        self.node_selfreference = []
        self.origin_nodes_community = []
        self.last_modularity = None
        for e in edges:
            self.edges[e[1]].append((e[0],1.0))
            self.edges[e[0]].append((e[1],1.0))
            self.orig_edges[e[1]].append((e[0],1.0))
            self.orig_edges[e[0]].append((e[1],1.0))
        for i in range(0, nodes_len):
            self.node_community.append(i)
            self.origin_nodes_community.append(i)
            self.node_selfreference.append(0.0)

        self.init_caches(nodes_len)
    
    def init_caches(self, nodes_len):
        self.node_degrees = []
        for node_index in range(0, nodes_len):
            sum_weights = self.node_selfreference[node_index]
            for _neighbor, weight in self.edges[node_index]:
                sum_weights += weight
            self.node_degrees.append(sum_weights)

        for community in self.communities:
            community.total_degree = self.community_total_degree_compute(community.id)

        self.node_communities_weights = []
        for node_index in range(0, nodes_len):
            comp_weights = self.node_communities_compute(node_index)
            self.node_communities_weights.append(comp_weights)
    
    def louvain(self, currentResolution=1.0, randomize = True):
        someChange = True
        while someChange:
            someChange = False
            localChange = True
            while localChange:
                localChange = False
                nodes = list(range(len(self.communities)))
                random.shuffle(nodes)
                for node_index in nodes:
                    bestCommunity = self.updateBestCommunity(node_index, currentResolution)
                    print(f"Node {node_index} best community {bestCommunity}")
                    if bestCommunity and bestCommunity != self.node_community[node_index]:
                        print(f"Moving node {node_index} from community {self.node_community[node_index]} to {bestCommunity} {self.communities[bestCommunity].nodes}")
                        self.moveNodeTo(node_index, bestCommunity)
                        modularity = self.compute_modularity()
                        print(f"modularity: {modularity} orig_modularity {self.compute_orig_modularity()}")
                        if self.last_modularity and self.last_modularity>modularity:
                            raise Exception("wrong modularity hase became smaller")
                        self.last_modularity = modularity
                        localChange = True
                someChange = localChange or someChange

            print(f"Final communities: {self.node_community}")
            if someChange:
                self.merge_nodes()
                print(f"mapping {self.origin_nodes_community}")
                print(f"merged edges {self.edges}")
                print(f"self references {self.node_selfreference}")
                print(f"node2communities {self.node_communities_weights}")
                #break
                #if len(self.node_community)<=4:
                #    break

    def debug(self):
        for c in self.communities:
            print(f"Community {c.id}: nodes={c.nodes}")

    def updateBestCommunity(self, node_index: int, resolution: float) -> int:
        best = 0
        bestCommunity = None
        for community_index, shared_degree in self.node_communities(node_index).items():
            if shared_degree>0.0:
                qValue = self.q(node_index, community_index, shared_degree, resolution)
                print(f" node: {node_index} community {community_index} q={qValue}")
                if qValue > best:
                    best = qValue
                    bestCommunity = community_index
        return bestCommunity
    
    def node_communities(self, node_index: int) -> Dict[int,float]:
        return self.node_communities_weights[node_index]
    
    def node_communities_compute(self, node_index: int) -> Dict[int,float]:
        communities = dict()
        for neighbor, weight in self.edges[node_index]:
            community_index = self.node_community[neighbor]
            if community_index not in communities:
                communities[community_index] = weight
            else:
                communities[community_index] += weight
        return communities
    
    def node_degree(self, node_index: int) -> float:
        return self.node_degrees[node_index]
    
    def community_total_degree_compute(self, community_index: int) -> int:
        sum = 0
        for node_index in self.communities[community_index].nodes:
            sum += self.node_degree(node_index)
        return sum

    def community_total_degree(self, community_index: int) -> int:
        #return self.community_total_degree_compute(community_index)
        return self.communities[community_index].total_degree

    def shared_degree(self, node_index: int, community_index: int) -> int:
        # number of edges from node to community
        sum = 0.0
        for neighbor, weight in self.edges[node_index]:
            if self.node_community[neighbor] == community_index:
                sum += weight
        return sum

    def q(self, node_index: int, community_index: int, shared_degree: float, resolution: float) -> float:
        # the formular is 
        # deleta_q = resolution * d_ij/m - (d_i*d_j)/(2*m*m)
        # deleta_q = (resolution*d_ij - (d_i*d_j)/(2*m))/m
        # d_ij = number of edges from node to community
        # d_i = degree of node
        # d_j = total degree of community

        current_community = self.node_community[node_index]
        if current_community == community_index:
            if len(self.communities[community_index].nodes) == 1:
                return 0.0
            else:
                d_i = self.node_degree(node_index)
                # we simulate the case that the node is removed from current community
                # so the community total degree is reduced by d_i
                d_j = self.community_total_degree(community_index) - d_i
                d_ij = shared_degree * 2.0
                return (resolution*d_ij-(d_i*d_j)/(self.m * 0.5))/(self.m)
        else:
            d_i = self.node_degree(node_index)
            d_j = self.community_total_degree(community_index)
            d_ij = shared_degree * 2.0
            #print(f" d_i {d_i} d_j {d_j} d_ij {d_ij} m {self.m} self_reference {self.node_selfreference[node_index]} node_index {node_index}")
            return (resolution*d_ij-(d_i*d_j)/(self.m * 0.5))/(self.m)

    def moveNodeTo(self, node_index: int, community: int):
        old_community = self.node_community[node_index]
        node_degree = self.node_degree(node_index)
        self.communities[old_community].remove_node(node_index)
        self.communities[old_community].total_degree -= node_degree
        self.communities[community].add_node(node_index)
        self.communities[community].total_degree += node_degree
        for neighbor, weight in self.edges[node_index]:
            if old_community in self.node_communities_weights[neighbor]:
                self.node_communities_weights[neighbor][old_community] -= weight
                if self.node_communities_weights[neighbor][old_community] <= 0.0:
                    del self.node_communities_weights[neighbor][old_community]
            if community in self.node_communities_weights[neighbor]:
                self.node_communities_weights[neighbor][community] += weight
            else:
                self.node_communities_weights[neighbor][community] = weight
        
        self.node_community[node_index] = community


        # remove connection to old community
        #nodes_len = len(self.node_communities_weights)
        #self.node_communities_weights = []
        #for node_index in range(0, nodes_len):
        #    comp_weights = self.node_communities_compute(node_index)
        #    self.node_communities_weights.append(comp_weights)


    def merge_nodes(self):
        # We need new length of nodes, which is number of not empty communities
        # after it the list of edges between communities
        # we need to map between old community id and new community id
        community_id_map : map[int,int] = dict()
        new_community_count = 0
        for c in self.communities:
            if len(c.nodes) > 0:
                community_id_map[c.id] = new_community_count
                new_community_count += 1
        new_communities = []
        new_edges = {}
        new_node_selfreference = []
        m = 0.0
        for community_id, new_community_id in community_id_map.items():
            c = self.communities[community_id]
            c.id = new_community_id
            edges_for_community = {}
            new_communities.append(c)
            self_reference = 0.0
            for node in c.nodes:
                for neighbor, weight in self.edges[node]:
                    neighbor_community = self.node_community[neighbor]
                    neighbor_community_new = community_id_map[neighbor_community]
                    if neighbor_community_new in edges_for_community:
                        edges_for_community[neighbor_community_new] += weight
                    else:
                        edges_for_community[neighbor_community_new] = weight
                self_reference += self.node_selfreference[node]
            new_edges[new_community_id] = []
            c.nodes = [new_community_id]
            for neighbor_community, weight in edges_for_community.items():
                m += weight
                if neighbor_community == new_community_id:
                    self_reference += weight
                else:
                    new_edges[new_community_id].append((neighbor_community, weight))
            new_node_selfreference.append(self_reference)

        self.communities = new_communities
        self.node_selfreference = new_node_selfreference

        for i in range(0, len(self.origin_nodes_community)):
            new_community_old_id = self.node_community[self.origin_nodes_community[i]]
            self.origin_nodes_community[i] = community_id_map[new_community_old_id]

        self.edges = new_edges
        self.m = m
        self.node_community = []
        for i in range(0, new_community_count):
            self.node_community.append(i)

        self.init_caches(new_community_count)            
        print(f"Merged to new {new_community_count} communities")

    def compute_modularity(self, resolution=1.0):
        """
        Compute modularity of the current partition in self.node_community.
        Works for weighted, undirected graphs, including self-loops.
        """
        # total edge weight
        m = self.m * 0.5
        if m == 0:
            return 0.0

        # degree of each node (including self-loop)
        k = self.node_degrees

        # community -> list of nodes
        communities = defaultdict(list)
        for node, comm in enumerate(self.node_community):
            communities[comm].append(node)

        Q = 0.0
        for nodes in communities.values():
            # sum of weights of internal edges
            in_weight = 0.0
            tot_degree = 0.0
            for u in nodes:
                tot_degree += k[u]
                for v, w in self.edges.get(u, []):
                    if self.node_community[v] == self.node_community[u]:
                        in_weight += w
                # include self-loop if any
                in_weight += self.node_selfreference[u]

            # each internal edge counted twice (u->v and v->u), so divide by 2
            in_weight /= 2.0

            Q += in_weight / m - resolution * (tot_degree / (2*m))**2

        return Q
    
    def compute_orig_modularity(self, resolution=1.0):
        m = 0.0

        # community -> list of nodes
        communities = defaultdict(list)
        for node, comm in enumerate(self.origin_nodes_community):
            current_com = self.node_community[comm]
            communities[current_com].append(node)

        k = {}
        for node, edges in self.orig_edges.items():
            node_degree = len(edges) * 1.0
            k[node] = node_degree
            m += node_degree

        m = m * 0.5

        Q = 0.0
        for nodes in communities.values():
            # sum of weights of internal edges
            in_weight = 0.0
            tot_degree = 0.0
            for u in nodes:
                tot_degree += k[u]
                for v, w in self.orig_edges.get(u, []):
                    if self.node_community[self.origin_nodes_community[v]] == self.node_community[self.origin_nodes_community[u]]:
                        in_weight += w

            # each internal edge counted twice (u->v and v->u), so divide by 2
            in_weight /= 2.0

            Q += in_weight / m - resolution * (tot_degree / (2*m))**2

        return Q
                

def louvain(nodes_len,edges: List[tuple[int,int]],currentResolution=1.0):
    # Initial partition: each node in its own community
    structure = Structure(nodes_len, edges)
    return structure.louvain(currentResolution)

# ---------- demonstration on a small graph ----------
if __name__ == "__main__":
    # simple graph with two communities: {0,1,2} and {3,4,5}
    if True:
        edges = [
            (0,1),(0,2),
            (2,3),
            (3,4),(3,5),
            (4,5)
        ]
        #louvain(6, edges)
        print("Louvain optimization demo")
        structure = Structure(6, edges)
        structure.debug()
        assert structure.node_degree(0) == 2.0
        assert structure.node_degree(1) == 1.0
        assert structure.node_degree(3) == 3.0
        assert structure.community_total_degree(0) == 2.0
        assert structure.community_total_degree(1) == 1.0
        assert structure.community_total_degree(3) == 3.0
        assert structure.shared_degree(0,2) == 1.0
        my_node_communities = structure.node_communities(0)
        assert my_node_communities[2] == 1.0
        assert structure.shared_degree(2,4) == 0.0
        my_node_communities = structure.node_communities(0)
        assert my_node_communities.keys() == {1,2}
        assert structure.q(0,1,my_node_communities[1],1.0) > 0.0
        assert structure.node_communities(2).keys() == {0,3}
        assert structure.node_communities(0).keys() == {1,2}
        assert structure.node_communities(1).keys() == {0}
        structure.moveNodeTo(1,0)
        assert structure.node_community[1] == 0
        assert len(structure.communities[0].nodes) == 2
        assert len(structure.communities[1].nodes) == 0
        assert structure.node_degree(1) == 1
        assert structure.community_total_degree(0) == 3
        assert structure.community_total_degree(1) == 0
        assert structure.shared_degree(2,0) == 1   
        assert structure.node_communities(2).keys() == {0,3}
        #print(f"node_communities 0: {structure.node_communities(0)}")
        assert structure.node_communities(0).keys() == {0, 2}
        assert structure.node_communities(1).keys() == {0}
        assert len(structure.communities) == 6
        old_m = structure.m
        structure.merge_nodes()
        new_m = structure.m
        print(f"m {new_m} m_old {old_m}")
        assert old_m == new_m
        assert len(structure.communities) == 5
        assert structure.origin_nodes_community == [0,0,1,2,3,4]
        assert len(structure.edges[0]) == 1
        assert len(structure.edges[1]) == 2
        assert structure.node_selfreference[0] == 2.0
        assert structure.communities[0].nodes == [0]
        assert structure.communities[1].nodes == [1]

        #print(f"node degree {structure.node_degree(0)}")
        #assert structure.node_degree(0) == 3.0
        modularity = structure.compute_modularity()
        print(f"modularity {modularity}")
        my_node_communities = structure.node_communities(1)      
        q_delta = structure.q(1,0,my_node_communities[0],1.0)
        structure.moveNodeTo(1,0)
        modularity_new = structure.compute_modularity()
        modularity_new_orig = structure.compute_orig_modularity()
        assert math.isclose(modularity_new, modularity_new_orig)
        print(f"node_selfreference {structure.node_selfreference}")
        print(f"modularity {modularity} new_modularity {modularity_new} q_delta {q_delta} diff {modularity_new - modularity}")
        assert math.isclose(modularity_new,modularity + q_delta)

        print("Restart 1")
        structure = Structure(6, edges)

        modularity = structure.compute_modularity()
        my_node_communities = structure.node_communities(0)
        q_delta = structure.q(0,1,my_node_communities[1],1.0)
        structure.moveNodeTo(0,1)
        modularity_new = structure.compute_modularity()
        print(f"modularity {modularity} new_modularity {modularity_new} q_delta {q_delta} diff {modularity_new - modularity}")
        assert math.isclose(modularity_new,modularity + q_delta)
        my_node_communities = structure.node_communities(2)
        assert my_node_communities.keys() == {1,3}
        assert my_node_communities[1] == 1.0
        assert my_node_communities[3] == 1.0
        assert structure.q(2,3,my_node_communities[3],1.0) > 0.0
        my_node_communities = structure.node_communities(0)
        assert my_node_communities.keys() == {1,2}
        assert my_node_communities[1] == 1.0
        assert my_node_communities[2] == 1.0
        my_node_communities = structure.node_communities(1)
        assert my_node_communities.keys() == {1}
        assert my_node_communities[1] == 1.0

        print("Restart 2")
        structure = Structure(6, edges)
        structure.louvain(1.0)
        assert len(structure.communities) == 2
        print("Origin communities:", structure.origin_nodes_community)

    print("Restart 3")
    complex_edges = [
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
    structure = Structure(16, complex_edges)
    structure.louvain(0.414, False)
    print("Origin communities complex:", structure.origin_nodes_community)

