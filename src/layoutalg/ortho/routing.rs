use std::{collections::VecDeque, usize};

use egui::{Pos2, Rect as ERect, Vec2};

/**
 * Create routing graph
 * 
 * 1) Create channel so it are boxes between node boxes.
 *   - There are vertical and horizontal channels.
 *   - merge channel together
 *   - create channel represenatives (lines in middle of channel)
 * 
 *  How create channels
 *   1) Find outbox rect (bounding box of all node boxes with some margin)
 *   2) Use sweep line to go from left to right. Create x-points sorted ()
 */

pub struct RoutingGraph {
    /**
     * nodes are stored in specifc order
     * - first original nodes (the same order as boxes)
     * - ports, 4 ports per node
     * - vertical channels
     * - horizontal channels
     * - bend points
     * 
     * There are specific method to get node info for port, node or channel index
     */
    pub nodes: Vec<RNode>,
    pub nodes_len: usize,
    pub vchannels: Vec<RChannel>,
    pub hchannels: Vec<RChannel>,
}


#[derive(Debug)]
pub enum RNodeType {
    BendPoint(usize,usize), // vertical channel index, horizontal channel index
    Port(usize,Side),
    Node(usize),
    Channel(usize,Orientation),
}
pub struct REdge {
    pub from: usize,
    pub to: usize,
}



#[derive(Debug)]
pub struct RNode {
    pub node_type: RNodeType,
    pub neighbors: Vec<usize>,
}

impl RNode {
    pub fn from_type(node_type: RNodeType) -> Self {
        RNode {
            node_type: node_type,
            neighbors: Vec::new(),
        }
    }

}

#[derive(Clone, PartialEq, Eq, Copy, Debug)]
pub enum Side {
    Right,
    Left,
    Top,
    Bottom,
}

impl Side {
    pub fn opposite(&self) -> Side {
        match self {
            Side::Right => Side::Left,
            Side::Left => Side::Right,
            Side::Top => Side::Bottom,
            Side::Bottom => Side::Top,
        }
    }

    pub fn is_opposite(&self,side: Side) -> bool {
        match self {
            Side::Right => side == Side::Left,
            Side::Left => side == Side::Right,
            Side::Top => side == Side::Bottom,
            Side::Bottom => side == Side::Top,
        }
    }

    pub fn orientation(&self) -> Orientation {
        match self {
            Side::Right | Side::Left => Orientation::Vertical,
            Side::Top | Side::Bottom => Orientation::Horizontal,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Orientation {
    Vertical,
    Horizontal,
}

/**
 * Abstract edge route is a route between two nodes in routing graph
 * It contains only node indices in routing graph
 * It does not contain any port or channel slot information
 */
pub struct AbstractEdgeRoute {
    from: usize,
    to: usize,
    route: Vec<usize>
}

pub struct EdgeRoute {
    abstract_route: usize,
    slots: Vec<u16>,
}

pub struct GraphEdgeRouting {
    pub edge_routes: Vec<EdgeRoute>,
    pub port_slots: Vec<u16>,
    pub vchannel_slots: Vec<u16>,
    pub hchannel_slots: Vec<u16>,    
}

impl RoutingGraph {
    pub fn add_edge(&mut self, from: usize, to: usize) {
        self.nodes[from].neighbors.push(to);
        self.nodes[to].neighbors.push(from);
    }

    pub fn port_start_node(&self, node_idx: usize) -> usize {
        node_idx * 4 + self.nodes_len
    }

    fn channel_node_idx(&self, channel_idx: usize, orientation: Orientation) -> usize {
        let channel_start = self.nodes_len + self.nodes_len * 4;
        match orientation {
            Orientation::Vertical => channel_start + channel_idx,
            Orientation::Horizontal => self.vchannels.len() + channel_start + channel_idx,
        }
    }

    fn channel(&self, channel_idx: usize, orientation: Orientation) -> &RChannel {
        match orientation {
            Orientation::Vertical => &self.vchannels[channel_idx],
            Orientation::Horizontal => &self.hchannels[channel_idx],
        }
    }
}

/* One need for different AreaLimit to build a channel rect */
struct AreaLimit {
    coord: f32, 
    min: f32, 
    max: f32, 
    node_id: usize,
}


#[derive(Debug)]
struct Port {
    node_id: usize,
    side: Side,
}

impl Port {
    pub fn position(&self, node_rect: &ERect) -> Pos2 {
        match self.side {
            Side::Right => Pos2::new(node_rect.max.x, node_rect.center().y),
            Side::Left => Pos2::new(node_rect.min.x, node_rect.center().y),
            Side::Top => Pos2::new(node_rect.center().x, node_rect.min.y),
            Side::Bottom => Pos2::new(node_rect.center().x, node_rect.max.y),
        }
    }
    pub fn slot_position(&self, node_rect: &ERect, slot: u16, total_slots: u16) -> Pos2 {
        match self.side {
            Side::Right => {
                let spacing = node_rect.height() / (total_slots as f32 + 1.0);
                Pos2::new(node_rect.max.x, node_rect.min.y + spacing * (slot as f32 + 1.0))
            },
            Side::Left => {
                let spacing = node_rect.height() / (total_slots as f32 + 1.0);
                Pos2::new(node_rect.min.x, node_rect.min.y + spacing * (slot as f32 + 1.0))
            },
            Side::Top => {
                let spacing = node_rect.width() / (total_slots as f32 + 1.0);
                Pos2::new(node_rect.min.x + spacing * (slot as f32 + 1.0), node_rect.min.y)
            },
            Side::Bottom => {
                let spacing = node_rect.width() / (total_slots as f32 + 1.0);
                Pos2::new(node_rect.min.x + spacing * (slot as f32 + 1.0), node_rect.max.y)
            },
        }
    }
}

#[derive(Debug)]
struct RChannel {
    rect: ERect,
    orientation: Orientation,
    ports: Vec<Port>,
}

impl RChannel {
    pub fn from_min_max(min: Pos2, max: Pos2, orientation: Orientation) -> Self {
        let rect = ERect::from_min_max(min, max);
        RChannel {
            rect,
            orientation,
            ports: Vec::new(),
        }
    }

    pub fn merge_v(&mut self, other: &mut RChannel) {       
        self.rect.set_top(self.rect.top().min(other.rect.top()));
        self.rect.set_bottom(self.rect.bottom().max(other.rect.bottom()));
        self.rect.set_left(self.rect.left().max(other.rect.left()));
        self.rect.set_right(self.rect.right().min(other.rect.right()));
        self.ports.append(&mut other.ports);
    }

    pub fn merge_h(&mut self, other: &mut RChannel) {       
        self.rect.set_left(self.rect.left().min(other.rect.left()));
        self.rect.set_right(self.rect.right().max(other.rect.right()));
        self.rect.set_top(self.rect.top().max(other.rect.top()));
        self.rect.set_bottom(self.rect.bottom().min(other.rect.bottom()));
        self.ports.append(&mut other.ports);
    }

    pub fn point_on_representative(&self, port_pos: Pos2) -> Pos2 {
        match self.orientation {
            Orientation::Vertical => {
                Pos2::new(self.rect.center().x, port_pos.y)
            },
            Orientation::Horizontal => {
                Pos2::new(port_pos.x, self.rect.center().y)
            },
        }
    }

    pub fn slot_position(&self, port_pos: Pos2, slot: u16, total_slots: u16) -> Pos2 {
        match self.orientation {
            Orientation::Vertical => {
                let spacing = self.rect.width() / (total_slots as f32 + 1.0);
                Pos2::new(self.rect.min.x + spacing * (slot as f32 + 1.0), port_pos.y)
            },
            Orientation::Horizontal => {
                let spacing = self.rect.height() / (total_slots as f32 + 1.0);
                Pos2::new(port_pos.x, self.rect.min.y + spacing * (slot as f32 + 1.0))
            },
        }
    }
}

struct OppositePort {
    node_id_ls: usize,
    node_id_gt: usize,
    channel_idx: usize,
    side_ls: Side,
    side_gt: Side,
}

fn build_channels(boxes: &[ERect]) -> (Vec<RChannel>,Vec<RChannel>) {
    let bounding_box = boxes.iter().fold(
        ERect::from_min_max(egui::pos2(f32::INFINITY, f32::INFINITY), egui::pos2(f32::NEG_INFINITY, f32::NEG_INFINITY)),
        |acc, b| acc.union(*b)
    );
    
    let mut limits_vl = Vec::new();
    let mut limits_vr = Vec::new();
    let mut limits_ht = Vec::new();
    let mut limits_hb = Vec::new();
    for (i, b) in boxes.iter().enumerate() {
        limits_vl.push(AreaLimit{coord: b.min.x, min: b.min.y, max: b.max.y, node_id: i});
        limits_vr.push(AreaLimit{coord: b.max.x, min: b.min.y, max: b.max.y, node_id: i});
        limits_ht.push(AreaLimit{coord: b.min.y, min: b.min.x, max: b.max.x, node_id: i});
        limits_hb.push(AreaLimit{coord: b.max.y, min: b.min.x, max: b.max.x, node_id: i});
    }
    let margin: f32 = 10.0;
    let bounding_box = bounding_box.expand(margin);
    limits_vl.push(AreaLimit{coord: bounding_box.max.x, min: bounding_box.min.y, max: bounding_box.max.y, node_id: usize::MAX});
    limits_vr.push(AreaLimit{coord: bounding_box.min.x, min: bounding_box.min.y, max: bounding_box.max.y, node_id: usize::MAX});
    limits_ht.push(AreaLimit{coord: bounding_box.max.y, min: bounding_box.min.x, max: bounding_box.max.x, node_id: usize::MAX});
    limits_hb.push(AreaLimit{coord: bounding_box.min.y, min: bounding_box.min.x, max: bounding_box.max.x,  node_id: usize::MAX});

    limits_vl.sort_by(|a,b| a.coord.partial_cmp(&b.coord).unwrap());
    limits_vr.sort_by(|a,b| a.coord.partial_cmp(&b.coord).unwrap());
    limits_ht.sort_by(|a,b| a.coord.partial_cmp(&b.coord).unwrap());
    limits_hb.sort_by(|a,b| a.coord.partial_cmp(&b.coord).unwrap());
    
    let mut vchannels: Vec<RChannel> = Vec::new();

    // Find vertical channels based of left limits
    for right in limits_vl.iter() {
        // find to
        // search first area that intersect line vertical lien from left edge corner
        'next_area: for top in limits_hb.iter().rev() {
            if top.coord > right.min {
                continue;
            }
            if right.coord >= top.min && right.coord <= top.max {
                // find bottom
                for bottom in limits_ht.iter() {
                    if bottom.coord < right.max {
                        continue;
                    }
                    if right.coord >= bottom.min && right.coord <= bottom.max {
                        // find left
                        for left in limits_vr.iter().rev() {
                            if left.coord > right.coord {
                                continue;
                            }
                            // check if left min and max intersect with top and bottom
                            if left.min <= bottom.coord && top.coord <= left.max {
                                let mut channel = RChannel::from_min_max(
                                    egui::pos2(left.coord, top.coord),
                                    egui::pos2(right.coord, bottom.coord),
                                    Orientation::Vertical
                                );
                                if right.node_id != usize::MAX {
                                    let port = Port {
                                        node_id: right.node_id,
                                        side: Side::Left,
                                    };
                                    channel.ports.push(port);
                                }
                                let mut merged = false;
                                for mc in vchannels.iter_mut() {
                                    if mc.rect.intersects(channel.rect) {
                                        mc.merge_v(&mut channel);
                                        merged = true;
                                        break;
                                    }
                                }
                                if !merged {
                                    vchannels.push(channel);
                                }
                                break 'next_area;
                            }
                        }
                    }
                }
            }
        }
    }
    // Find vertical channels based of left limits
    // TODO: We could optimize it if the left corner is in the right area so the port can be already added in run above
    for left in limits_vr.iter() {
        // find to
        // search first area that intersect line vertical lien from left edge corner
        'next_area: for top in limits_hb.iter().rev() {
            if top.coord > left.min {
                continue;
            }
            if left.coord >= top.min && left.coord <= top.max {
                // find bottom
                for bottom in limits_ht.iter() {
                    if bottom.coord < left.max {
                        continue;
                    }
                    if left.coord >= bottom.min && left.coord <= bottom.max {
                        // find left
                        for right in limits_vl.iter() {
                            if left.coord > right.coord {
                                continue;
                            }
                            // check if left min and max intersect with top and bottom
                            if right.min <= bottom.coord && top.coord <= right.max {
                                let mut channel = RChannel::from_min_max(
                                    egui::pos2(left.coord, top.coord),
                                    egui::pos2(right.coord, bottom.coord),
                                    Orientation::Vertical
                                );
                                if left.node_id != usize::MAX {
                                    let port = Port {
                                        node_id: left.node_id,
                                        side: Side::Right,
                                    };
                                    channel.ports.push(port);
                                }
                                let mut merged = false;
                                for mc in vchannels.iter_mut() {
                                    if mc.rect.intersects(channel.rect) {
                                        mc.merge_v(&mut channel);
                                        merged = true;
                                        break;
                                    }
                                }
                                if !merged {
                                    vchannels.push(channel);
                                }
                                break 'next_area;
                            }
                        }
                    }
                }
            }
        }
    }

    let mut hchannels: Vec<RChannel> = Vec::new();

    for bottom in limits_ht.iter() {
        // find to
        // search first area that intersect line vertical line from left edge corner
        'next_area: for left in limits_vr.iter().rev() {
            if left.coord > bottom.min {
                continue;
            }
            if bottom.coord >= left.min && bottom.coord <= left.max {
                // find right
                for right in limits_vl.iter() {
                    if right.coord < bottom.max {
                        continue;
                    }
                    if bottom.coord >= right.min && bottom.coord <= right.max {
                        // find top
                        for top in limits_hb.iter().rev() {
                            if top.coord > bottom.coord {
                                continue;
                            }
                            // check if left min and max intersect with top and bottom
                            if top.min <= right.coord && left.coord <= top.max {
                                let mut channel = RChannel::from_min_max(
                                    egui::pos2(left.coord, top.coord),
                                    egui::pos2(right.coord, bottom.coord),
                                    Orientation::Horizontal
                                );
                                if bottom.node_id != usize::MAX {
                                    let port = Port {
                                        node_id: bottom.node_id,
                                        side: Side::Top,
                                    };
                                    channel.ports.push(port);
                                }
                                let mut merged = false;
                                for mc in hchannels.iter_mut() {
                                    if mc.rect.intersects(channel.rect) {
                                        mc.merge_h(&mut channel);
                                        merged = true;
                                        break;
                                    }
                                }
                                if !merged {
                                    hchannels.push(channel);
                                }
                                break 'next_area;
                            }
                        }
                    }
                }
            }
        }
    }

    for top in limits_hb.iter() {
        // find to
        // search first area that intersect line vertical line from left edge corner
        'next_area: for left in limits_vr.iter().rev() {
            if left.coord > top.min {
                continue;
            }
            if top.coord >= left.min && top.coord <= left.max {
                // find right
                for right in limits_vl.iter() {
                    if right.coord < top.max {
                        continue;
                    }
                    if top.coord >= right.min && top.coord <= right.max {
                        // find bottom
                        for bottom in limits_ht.iter() {
                            if top.coord > bottom.coord {
                                continue;
                            }
                            // check if left min and max intersect with top and bottom
                            if bottom.min <= right.coord && left.coord <= bottom.max {
                                let mut channel = RChannel::from_min_max(
                                    egui::pos2(left.coord, top.coord),
                                    egui::pos2(right.coord, bottom.coord),
                                    Orientation::Horizontal
                                );
                                if top.node_id != usize::MAX {
                                    let port = Port {
                                        node_id: top.node_id,
                                        side: Side::Bottom,
                                    };
                                    channel.ports.push(port);
                                }                                
                                let mut merged = false;
                                for mc in hchannels.iter_mut() {
                                    if mc.rect.intersects(channel.rect) {
                                        mc.merge_h(&mut channel);
                                        merged = true;
                                        break;
                                    }
                                }
                                if !merged {
                                    hchannels.push(channel);
                                }
                                break 'next_area;
                            }
                        }
                    }
                }
            }
        }
    }

    (vchannels, hchannels)
}


pub fn create_routing_graph(boxes: &[ERect]) -> RoutingGraph {
    let (vchannels, hchannels) = build_channels(boxes);
    let mut cross_points: Vec<(usize,usize,Pos2)> = Vec::new();
    let mut rnodes: Vec<RNode> = Vec::new();
    let mut redges: Vec<REdge> = Vec::new();
    for (node_id, _b) in boxes.iter().enumerate() {
        let rnode = RNode::from_type(RNodeType::Node(node_id));
        rnodes.push(rnode);
    }
    for (node_id, _b) in boxes.iter().enumerate() {
        rnodes.push(RNode::from_type(RNodeType::Port(node_id, Side::Right)));
        redges.push(REdge{from: node_id, to: rnodes.len()-1});
        rnodes.push(RNode::from_type(RNodeType::Port(node_id, Side::Left)));
        redges.push(REdge{from: node_id, to: rnodes.len()-1});
        rnodes.push(RNode::from_type(RNodeType::Port(node_id, Side::Top)));
        redges.push(REdge{from: node_id, to: rnodes.len()-1});
        rnodes.push(RNode::from_type(RNodeType::Port(node_id, Side::Bottom)));
        redges.push(REdge{from: node_id, to: rnodes.len()-1});
    }
    let vchannel_offset = rnodes.len();
    for (channel_idx,channel) in vchannels.iter().enumerate() {
        let rnode = RNode::from_type(RNodeType::Channel(channel_idx,Orientation::Vertical));
        rnodes.push(rnode);
        let channel_node_idx = rnodes.len() -1;
        for port in &channel.ports {
            let port_rnode_idx = port.node_id * 4 + (port.side as usize) + boxes.len();
            redges.push(REdge{from: channel_node_idx, to: port_rnode_idx});
        }
    }
    let hchannel_offset = rnodes.len();
    for (channel_idx,channel) in hchannels.iter().enumerate() {
        let rnode = RNode::from_type(RNodeType::Channel(channel_idx,Orientation::Horizontal));
        rnodes.push(rnode);
        let channel_node_idx = rnodes.len() -1;
        for port in &channel.ports {
            let port_rnode_idx = port.node_id * 4 + (port.side as usize) + boxes.len();
            redges.push(REdge{from: channel_node_idx, to: port_rnode_idx});
        }
    }
    for (vindex,vchannel) in vchannels.iter().enumerate() {
        for (hindex, hchannel) in hchannels.iter().enumerate() {
            if vchannel.rect.intersects(hchannel.rect) {
                let intersection = vchannel.rect.intersect(hchannel.rect);
                cross_points.push((vindex,hindex, intersection.center()));
                let rnode = RNode::from_type(RNodeType::BendPoint(vindex,hindex));
                rnodes.push(rnode);
                let bend_node_idx = rnodes.len() - 1;
                redges.push(REdge{from: bend_node_idx, to: vchannel_offset + vindex});
                redges.push(REdge{from: bend_node_idx, to: hchannel_offset + hindex});
            }
        }
    }
    let mut routing_grpah = RoutingGraph {
        nodes: rnodes,
        nodes_len: boxes.len(),
        vchannels,
        hchannels
    };
    for edge in redges {
        routing_grpah.add_edge(edge.from, edge.to);
    }
    routing_grpah
}

/**
 * Computes abstract routes for all possible edges
 * The routes are abstract because they have no port and channel slots assigned
 */
pub fn route_edges(routing_graph: &RoutingGraph, edges: &[(usize,usize)]) -> Vec<AbstractEdgeRoute> {
    let mut edges_routes: Vec<AbstractEdgeRoute> = Vec::new();
    let mut edges: Vec<(usize,usize)> = edges.into_iter()
        .filter(|(from,to)| *from != *to)
        .map(|(from,to)| {
        if from > to {
            (*to, *from)
        } else {
            (*from, *to)
        }
    }).collect();
    edges.sort_unstable_by(|(from_a,_to_a),(from_b,_to_b)| {
        from_a.cmp(from_b)
    });
    edges.dedup();
    let mut opposite_ports : Vec<OppositePort> = Vec::new();

    fn collect_opposite_ports(channels: &[RChannel], opposite_ports: &mut Vec<OppositePort>) {
        for (channel_id, channel) in channels.iter().enumerate() {
            for port_a in channel.ports.iter() {
                for port_b in channel.ports.iter() {
                    if port_a.node_id < port_b.node_id && port_a.side.is_opposite(port_b.side) {
                        let op = OppositePort {
                            node_id_ls: port_a.node_id,
                            node_id_gt: port_b.node_id,
                            channel_idx: channel_id,
                            side_ls: port_a.side,
                            side_gt: port_b.side,
                        };
                        opposite_ports.push(op);
                    }
                }
            }
        }
    }
    collect_opposite_ports(&routing_graph.vchannels, &mut opposite_ports);
    collect_opposite_ports(&routing_graph.hchannels, &mut opposite_ports);
    opposite_ports.sort_unstable_by(|a,b| 
        a.node_id_ls.cmp(&b.node_id_ls).then(a.node_id_gt.cmp(&b.node_id_gt)));

    let mut targets: Vec<usize> = Vec::new();
    let mut last_from = usize::MAX;
    for (from, to) in edges.iter() {
        if last_from != *from {
            if !targets.is_empty() {
                route_edges_from(routing_graph, last_from, &targets, &opposite_ports, &mut edges_routes);
            }
            last_from = *from;
            targets.clear();
        }
        targets.push(*to);
    }
    route_edges_from(routing_graph, last_from, &targets, &opposite_ports, &mut edges_routes);
    edges_routes.sort_unstable_by(|a,b| a.from.cmp(&b.from).then(a.to.cmp(&b.to)));
    edges_routes
}

fn route_edges_from(routing_graph: &RoutingGraph, from: usize, to: &[usize], opposite_ports: &[OppositePort], edges_routes: &mut Vec<AbstractEdgeRoute>) {
    // make bfs from from node to all to nodes.
    let mut visited: Vec<bool> = vec![false; routing_graph.nodes.len()];
    let mut predecessor: Vec<usize> = vec![usize::MAX; routing_graph.nodes.len()];
    let port_start_idx = routing_graph.port_start_node(from);
    let mut queue: VecDeque<usize> = VecDeque::new();
    for i in 0..4 {
        queue.push_back(port_start_idx+i);
        predecessor[port_start_idx+i] = from;
    }
    let mut to_find = to.len();
    // find direct connection first (on same channel and exactly opposite ports)
    for to_idx in to.iter() {
        let search_result = opposite_ports.binary_search_by(|op| {
            op.node_id_ls.cmp(&from).then(op.node_id_gt.cmp(to_idx))
        });
        if let Ok(op_idx) = search_result {
            let op = &opposite_ports[op_idx];
            let channel_node_idx = routing_graph.channel_node_idx(op.channel_idx, op.side_ls.orientation());
            let from_port_idx = from *4 + (op.side_ls as usize) + routing_graph.nodes_len;
            let to_port_idx = op.node_id_gt *4 + (op.side_gt as usize) + routing_graph.nodes_len;
            let route = vec![from_port_idx, channel_node_idx, to_port_idx];
            edges_routes.push(AbstractEdgeRoute {
                from,
                to: *to_idx,
                route,
            });
            to_find -= 1;
            let target_port_start_idx = routing_graph.port_start_node(*to_idx);
            for i in 0..4 {
                visited[target_port_start_idx+i] = true;
            }
        }
    }
    if to_find == 0 {
        return;
    }
    // This could be optimized be using A* search with heuristic
    // The heuristic could be vector distance between current and target node (because we are in 2D space)
    while let Some(node_idx) = queue.pop_front() {
        if visited[node_idx] {
            continue;
        }
        visited[node_idx] = true;
        let node = routing_graph.nodes.get(node_idx).unwrap();
        match node.node_type {
            RNodeType::Port(node_index,_side) if node_index != from => {
                if to.contains(&node_index) {
                    // found route
                    let mut route: Vec<usize> = Vec::new();
                    let mut current = node_idx;
                    while let Some(&prev) = predecessor.get(current) {
                        route.push(current);
                        current = prev;
                        if prev == from {
                            break;
                        }
                    }
                    route.reverse();
                    edges_routes.push(AbstractEdgeRoute {
                        from,
                        to: node_index,
                        route,
                    });
                    // If already all routes found exit
                    to_find -= 1;
                    if to_find == 0 {
                        break;
                    }
                    // mark all ports of this node as visited
                    let target_port_start_idx = routing_graph.port_start_node(node_index);
                    for i in 0..4 {
                        visited[target_port_start_idx+i] = true;
                    }
                }
            },
            _ => {
                for target in node.neighbors.iter() {
                    if !visited[*target] {
                        predecessor[*target] = node_idx;
                        queue.push_back(*target);
                    }
                }
            }
        }
    }
}

pub fn calculate_edge_routes(routing_graph: &RoutingGraph, edges: &[(usize,usize)], routes: &[AbstractEdgeRoute]) -> GraphEdgeRouting {
    let mut ports_slots = vec![0u16; routing_graph.nodes_len * 4];
    let mut vchannel_slots = vec![0u16; routing_graph.vchannels.len()];
    let mut hchannel_slots = vec![0u16; routing_graph.hchannels.len()];
    
    let mut edge_routes: Vec<EdgeRoute> = Vec::new();
    for (e_from,e_to) in edges.iter() {
        let node_min = if e_from < e_to { *e_from } else { *e_to };
        let node_max = if e_from < e_to { *e_to } else { *e_from };
        let route_idx = routes.binary_search_by(|r| {
            r.from.cmp(&node_min).then(r.to.cmp(&node_max))
        });
        if let Ok(route_idx) = route_idx {
            let route = &routes[route_idx];           
            let mut slots = vec![0; route.route.len()];
            for (point_idx, node_idx) in route.route.iter().enumerate() {
                let node = &routing_graph.nodes[*node_idx];
                match &node.node_type {
                    RNodeType::Port(node_index, side) => {
                        let port_idx = node_index *4 + (*side as usize);
                        slots[point_idx] = ports_slots[port_idx];
                        ports_slots[port_idx] += 1;
                    },
                    RNodeType::Channel(channel_idx, orientation) => {
                        match orientation {
                            Orientation::Vertical => {
                                slots[point_idx] = vchannel_slots[*channel_idx];
                                vchannel_slots[*channel_idx] += 1;
                            },
                            Orientation::Horizontal => {
                                slots[point_idx] = hchannel_slots[*channel_idx];
                                hchannel_slots[*channel_idx] += 1;
                            },
                        }
                    },
                    _ => {}
                }
            }
            edge_routes.push(EdgeRoute {
                abstract_route: route_idx,
                slots: slots,
            });
        } else {
            panic!("No route found for edge from {} to {}", e_from, e_to);
        }
    }
    GraphEdgeRouting {
        edge_routes: edge_routes,
        port_slots: ports_slots,
        vchannel_slots: vchannel_slots,
        hchannel_slots: hchannel_slots,
    }
}

/**
 * Only used for debugging purposes to get actual points from abstract routes
 */
pub fn map_abstract_routes(routing_graph: &RoutingGraph, boxes: &[ERect], routes: &Vec<AbstractEdgeRoute>) -> Vec<Vec<Pos2>> {
    routes.iter().map(|route| {
        let mut points: Vec<Pos2> = Vec::new();
        let mut start_port: Option<Port> = None;
        let mut channel: Option<(usize,Orientation)> = None;
        for node_idx in route.route.iter() {
            let node = &routing_graph.nodes[*node_idx];
            match &node.node_type {
                RNodeType::Port(node_index, side) => {
                    if let Some((channel_id,orientation)) = channel.take() {
                        // channel to port connection
                        let rect = &boxes[*node_index];
                        let port = Port { node_id: *node_index, side: *side };
                        let port_pos = port.position(rect);
                        let channel = routing_graph.channel(channel_id, orientation);
                        let channel_point = channel.point_on_representative(port_pos);
                        points.push(channel_point);                       
                        points.push(port_pos);
                    } else {
                        start_port = Some(Port { node_id: *node_index, side: *side })
                    }
                },
                RNodeType::Channel(channel_id,orientation ) => {
                    if let Some(start_port) = start_port.take() {
                        // port to channel connection
                        let rect = &boxes[start_port.node_id];
                        let port_pos = start_port.position(rect);
                        points.push(port_pos);
                        let channel = routing_graph.channel(*channel_id, *orientation);
                        let channel_point = channel.point_on_representative(port_pos);
                        points.push(channel_point);
                    }
                    channel = Some((*channel_id,*orientation));
                }
                RNodeType::BendPoint(vindex,hindex) => {
                    let vchannel = &routing_graph.vchannels[*vindex];
                    let hchannel = &routing_graph.hchannels[*hindex];
                    let irect = vchannel.rect.intersect(hchannel.rect);
                    points.push(irect.center());
                },
                _ => {}
            }
        }
        points
    }).collect::<Vec<_>>()
}

pub fn map_routes_to_segements(routing_graph: &RoutingGraph, boxes: &[ERect], routes: &Vec<AbstractEdgeRoute>, edge_routing: &GraphEdgeRouting) -> Vec<Vec<Pos2>> {
    let mut all_segments: Vec<Vec<Pos2>> = Vec::new();
    for edge_route in edge_routing.edge_routes.iter() {
        let abstract_route = &routes[edge_route.abstract_route];
        let mut points: Vec<Pos2> = Vec::new();
        let mut start_port: Option<Port> = None;
        let mut channel: Option<(usize,Orientation)> = None;
        for (point_idx, rnode_idx) in abstract_route.route.iter().enumerate() {
            let node = &routing_graph.nodes[*rnode_idx];
            match &node.node_type {
                RNodeType::Port(node_index, side) => {
                    if let Some((channel_id,orientation)) = channel.take() {
                        // channel to port connection
                        let rect = &boxes[*node_index];
                        let port = Port { node_id: *node_index, side: *side };
                        let port_pos = port.slot_position(rect, edge_route.slots[point_idx], edge_routing.port_slots[*node_index * 4 + (*side as usize)]);
                        let channel = routing_graph.channel(channel_id, orientation);
                        let total_channel_slots = match orientation {
                            Orientation::Vertical => edge_routing.vchannel_slots[channel_id],
                            Orientation::Horizontal => edge_routing.hchannel_slots[channel_id],
                        };
                        let channel_point = channel.slot_position(port_pos, edge_route.slots[point_idx-1], total_channel_slots);
                        points.push(channel_point);                       
                        points.push(port_pos);
                    } else {
                        start_port = Some(Port { node_id: *node_index, side: *side })
                    }
                },
                RNodeType::Channel(channel_id,orientation ) => {
                    if let Some(start_port) = start_port.take() {
                        // port to channel connection
                        let rect = &boxes[start_port.node_id];
                        let port_pos = start_port.slot_position(rect, edge_route.slots[point_idx -1], edge_routing.port_slots[start_port.node_id *4 + (start_port.side as usize)]);
                        points.push(port_pos);
                        let channel = routing_graph.channel(*channel_id, *orientation);
                        let total_channel_slots = match orientation {
                            Orientation::Vertical => edge_routing.vchannel_slots[*channel_id],
                            Orientation::Horizontal => edge_routing.hchannel_slots[*channel_id],
                        };
                        let channel_point = channel.slot_position(port_pos, edge_route.slots[point_idx], total_channel_slots);
                        points.push(channel_point);
                    }
                    channel = Some((*channel_id,*orientation));
                }
                RNodeType::BendPoint(vindex,hindex) => {
                    let vchannel = &routing_graph.vchannels[*vindex];
                    let hchannel = &routing_graph.hchannels[*hindex];
                    let irect = vchannel.rect.intersect(hchannel.rect);
                    points.push(irect.center());
                },
                _ => {}
            }
        }
        all_segments.push(points);
    }
    all_segments
}


#[cfg(test)]
mod tests {
    use std::{fs, path::PathBuf, vec};

    use super::*;
    use plotters::{coord::Shift, prelude::*};

    fn test_rects() -> Vec<ERect> {
        vec![
            ERect::from_center_size(egui::pos2(20.0, 20.0), egui::vec2(30.0, 10.0)),
            ERect::from_center_size(egui::pos2(70.0, 22.0), egui::vec2(30.0, 10.0)),
            ERect::from_center_size(egui::pos2(20.0, 38.0), egui::vec2(25.0, 10.0)),
            ERect::from_center_size(egui::pos2(70.0, 40.0), egui::vec2(35.0, 10.0)),
            ERect::from_center_size(egui::pos2(40.0, 60.0), egui::vec2(55.0, 10.0)),
        ]
    }

    #[test]
    fn test_create_routing_graph() {
        let boxes = vec![
            ERect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(100.0, 100.0)),
            ERect::from_min_max(egui::pos2(150.0, 150.0), egui::pos2(250.0, 250.0)),
        ];
        let routing_graph = create_routing_graph(&boxes);
        // assert_eq!(routing_graph.nodes.len(), 0);
    }

    fn draw_rects(root: &DrawingArea<SVGBackend,Shift>,rects: &[ERect],color: &RGBColor) -> Result<(), Box<dyn std::error::Error>>{
        for (r, rect) in rects.iter().enumerate() {
            let top_left = (rect.min.x as i32, rect.min.y as i32);
            let bottom_right = (rect.max.x as i32, rect.max.y as i32);
            root.draw(&Rectangle::new(
                [top_left, bottom_right],
                ShapeStyle::from(color).stroke_width(1),
            ))?;
            let style = TextStyle::from(("sans-serif", 10).into_font()).color(color);
            let text = format!("{}",r);
            root.draw_text(&text, &style, (rect.min.x as i32, rect.min.y as i32))?;
        }
        Ok(())
    }

    fn draw_channels(root: &DrawingArea<SVGBackend,Shift>,channels: &[RChannel],color: &RGBColor) -> Result<(), Box<dyn std::error::Error>>{
        for channel in channels {
            let top_left = (channel.rect.min.x as i32, channel.rect.min.y as i32);
            let bottom_right = (channel.rect.max.x as i32, channel.rect.max.y as i32);
            root.draw(&Rectangle::new(
                [top_left, bottom_right],
                ShapeStyle::from(color).stroke_width(1),
            ))?;
            let center = channel.rect.center();
            let style = TextStyle::from(("sans-serif", 10).into_font()).color(color);
            let text = format!("{}",channel.ports.len());
            root.draw_text(&text, &style, (center.x as i32, center.y as i32))?;
        }
        Ok(())
    }

    #[test]
    fn test_build_channels() -> Result<(), Box<dyn std::error::Error>> {
        let boxes = test_rects();
        let (vchannels,hchannels) = build_channels(&boxes);
        assert!(!vchannels.is_empty());
        assert!(!hchannels.is_empty());

        let out_dir = PathBuf::from("target/test-output");
        fs::create_dir_all(&out_dir)?;
        let rects = test_rects();
        let svg_path = out_dir.join("channel_boxes.svg");
        let root = SVGBackend::new(&svg_path, (200, 200)).into_drawing_area();
        root.fill(&WHITE)?;

        draw_channels(&root, &vchannels, &RED)?;
        draw_channels(&root, &hchannels, &BLUE)?;
        draw_rects(&root, &rects, &BLACK)?;

        assert_eq!(vchannels.len(), 3);
        assert_eq!(hchannels.len(), 4);

        Ok(())
    }

    #[test]
    fn test_edge_routing() -> Result<(), Box<dyn std::error::Error>> {
        let boxes = test_rects();


        let routing_graph = create_routing_graph(&boxes);
        assert!(!routing_graph.nodes.is_empty());

        for (idx, c) in routing_graph.vchannels.iter().enumerate() {
            println!("V Channel: {} {:?}", idx, c);
        }
        for (idx, c) in routing_graph.hchannels.iter().enumerate() {
            println!("H Channel: {} {:?}", idx, c);
        }
        for (node_idx, node) in routing_graph.nodes.iter().enumerate() {
            println!(" idx: {} {:?}",node_idx, node);
        }

        let edges = vec![
            (0,1),
            (0,3),
            (0,4),
            (2,3),
            (1,4),
            (1,3),
            (2,4),
        ];

        let routes = route_edges(&routing_graph, &edges);
        for route in routes.iter() {
            println!("route for edge {}->{}", route.from, route.to);
            for node_idx in route.route.iter() {
                let node = &routing_graph.nodes[*node_idx];
                println!(" idx: {} {:?}",*node_idx, node);
            }
        }
        assert_eq!(routes.len(), edges.len());
        let graph_edge_routes = calculate_edge_routes(&routing_graph, &edges, &routes);
        assert_eq!(graph_edge_routes.edge_routes.len(), edges.len());

        let route_segments = map_routes_to_segements(&routing_graph, &boxes, &routes, &graph_edge_routes);
        assert_eq!(route_segments.len(), edges.len());

        let out_dir = PathBuf::from("target/test-output");
        fs::create_dir_all(&out_dir)?;
        let rects = test_rects();
        let svg_path = out_dir.join("graph_routes.svg");
        let root = SVGBackend::new(&svg_path, (200, 200)).into_drawing_area();
        root.fill(&WHITE)?;

        draw_rects(&root, &rects, &BLACK)?;

        for segments in route_segments {
            let points = segments.iter().map(|p| (p.x as i32, p.y as i32)).collect::<Vec<_>>();
            root.draw(&PathElement::new(
                    points,
                    ShapeStyle::from(&GREEN).stroke_width(1),
                )
            )?;
        }
        Ok(())
    }

    #[test]
    fn test_edge_route_slots() -> Result<(), Box<dyn std::error::Error>> {
        let rects = vec![
            ERect::from_min_max(egui::pos2(20.0, 20.0), egui::pos2(50.0, 30.0)),
            ERect::from_min_max(egui::pos2(60.0, 20.0), egui::pos2(100.0, 30.0)),
            ERect::from_min_max(egui::pos2(110.0, 20.0), egui::pos2(130.0, 30.0)),
            ERect::from_min_max(egui::pos2(30.0, 60.0), egui::pos2(70.0, 70.0)),
            ERect::from_min_max(egui::pos2(80.0, 60.0), egui::pos2(130.0, 70.0)),
        ];

        let out_dir = PathBuf::from("target/test-output");
        fs::create_dir_all(&out_dir)?;
        let svg_path = out_dir.join("routes_slots.svg");
        let root = SVGBackend::new(&svg_path, (200, 200)).into_drawing_area();
        root.fill(&WHITE)?;

        draw_rects(&root, &rects, &BLACK)?;

        let routing_graph = create_routing_graph(&rects);
        let port_start_idx = routing_graph.port_start_node(0);

        // We build own abstract routes that onle use one channel to better test
        // slot sorting and assigment
        let abstract_routes: Vec<AbstractEdgeRoute> = vec![
            AbstractEdgeRoute { 
                from: 0, to: 1, route: vec![
                    port_start_idx + Side::Bottom as usize,
                    routing_graph.channel_node_idx(1, Orientation::Horizontal),
                    port_start_idx + 4 + Side::Bottom as usize,
                ] 
            },
            AbstractEdgeRoute { 
                from: 0, to: 2, route: vec![
                    port_start_idx + Side::Bottom as usize,
                    routing_graph.channel_node_idx(1, Orientation::Horizontal),
                    port_start_idx + 8 + Side::Bottom as usize,
                ] 
            },
            AbstractEdgeRoute { 
                from: 1, to: 2, route: vec![
                    port_start_idx + 4 + Side::Bottom as usize,
                    routing_graph.channel_node_idx(1, Orientation::Horizontal),
                    port_start_idx + 8 + Side::Bottom as usize,
                ] 
            }
        ];

        let segmetns = map_abstract_routes(&routing_graph, &rects, &abstract_routes);

        for segments in segmetns.iter() {
            let points = segments.iter().map(|p| (p.x as i32, p.y as i32)).collect::<Vec<_>>();
            root.draw(&PathElement::new(
                    points,
                    ShapeStyle::from(&YELLOW).stroke_width(1),
                )
            )?;
        }

        let edges = vec![
            (0,1),
            (0,2),
            (0,2),
            (1,2),
        ];

        let graph_edge_routes = calculate_edge_routes(&routing_graph, &edges, &abstract_routes);
        assert_eq!(graph_edge_routes.edge_routes.len(), edges.len());

        let route_segments = map_routes_to_segements(&routing_graph, &rects, &abstract_routes, &graph_edge_routes);
        assert_eq!(route_segments.len(), edges.len());

        for segments in route_segments.iter() {
            let points = segments.iter().map(|p| (p.x as i32, p.y as i32)).collect::<Vec<_>>();
            root.draw(&PathElement::new(
                    points,
                    ShapeStyle::from(&RED).stroke_width(1),
                )
            )?;
        }


        Ok(())
    }

    #[test]
    fn test_plotters() -> Result<(), Box<dyn std::error::Error>> {
        let out_dir = PathBuf::from("target/test-output");
        fs::create_dir_all(&out_dir)?;
        let rects = test_rects();
        let svg_path = out_dir.join("test_output.svg");
        let root = SVGBackend::new(&svg_path, (200, 200)).into_drawing_area();
        root.fill(&WHITE)?;

        for rect in &rects {
            let top_left = (rect.min.x as i32, rect.min.y as i32);
            let bottom_right = (rect.max.x as i32, rect.max.y as i32);
            root.draw(&Rectangle::new(
                [top_left, bottom_right],
                ShapeStyle::from(&BLACK).stroke_width(1),
            ))?;
        }

        Ok(())
    }
}