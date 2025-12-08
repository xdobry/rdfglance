use std::{collections::VecDeque, iter, usize};

use egui::{Pos2, Rect as ERect};
use crate::dbgorth;
use crate::layoutalg::ortho::channels::{ChannelPort, ChannelPortType};
use crate::uistate::layout::Edge;

use super::channels::{RChannel, build_channels};
use super::routing_slots::{GraphEdgeRouting};

/**
 * Create routing graph
 * 
 * 1) Create channel so it are boxes between node boxes.
 *   - There are vertical and horizontal channels.
 *   - merge channel together
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
    Port(usize,usize,Side), // node_idx, channel_idx, side: The side is the side of the node where the port is located, so not the side of the channel
    Node(usize),
}


#[derive(Debug, PartialEq)]
pub struct ChannelOrientation {
    pub channel_idx: usize,
    pub orientation: Orientation,
}

impl RNodeType {
    pub fn get_channel_ref(&self) -> Option<ChannelOrientation> {
        match self {
            RNodeType::Port(_node_id, channel_id, side) => {
                Some(ChannelOrientation { channel_idx: *channel_id, orientation: side.orientation() })
            },
            _ => None,
        }
    }
    pub fn get_channel_id(&self, orientation: Orientation) -> ChannelOrientation {
        match self {
            RNodeType::Port(_node_id, channel_id, side) => {
                ChannelOrientation {
                    channel_idx: *channel_id,
                    orientation: side.orientation(),
                }
            },
            RNodeType::BendPoint(vertical, horizontal) => {
                match orientation {
                    Orientation::Vertical => ChannelOrientation {
                        channel_idx: *vertical,
                        orientation: Orientation::Vertical,
                    },
                    Orientation::Horizontal => ChannelOrientation {
                        channel_idx: *horizontal,
                        orientation: Orientation::Horizontal,
                    },
                }
            },
            _ => {
                panic!("RNodeType::get_channel_id called on non channel node");
            },
        }
    }    
    pub fn new_channel_orientation(&self, channel_orientation: &ChannelOrientation) -> Option<ChannelOrientation> {
        match self {
            RNodeType::Port(_node_id, channel_id, side) => {
                let orientation = side.orientation();
                if channel_orientation.channel_idx == *channel_id && orientation == channel_orientation.orientation {
                    return None;
                }
                Some(ChannelOrientation { channel_idx: *channel_id, orientation })
            },
            RNodeType::BendPoint(vertical, horizontal) => {
                let (channel_id, orientation) = match channel_orientation.orientation {
                    Orientation::Vertical => (*vertical, Orientation::Vertical),
                    Orientation::Horizontal => (*horizontal, Orientation::Horizontal),
                };
                if channel_orientation.channel_idx == channel_id && orientation == channel_orientation.orientation {
                    return None;
                } 
                Some(ChannelOrientation { channel_idx: channel_id, orientation: orientation.opposite() })
            },
            _ => None,
        }
    }
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

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Orientation {
    Vertical,
    Horizontal,
}

impl Orientation {
    pub fn opposite(&self) -> Orientation {
        match self {
            Orientation::Vertical => Orientation::Horizontal,
            Orientation::Horizontal => Orientation::Vertical,
        }
    }
}

/**
 * Abstract edge route is a route between two nodes in routing graph
 * It contains only node indices in routing graph
 * It does not contain any port or channel slot information
 * 
 * With exceptions each node is point in the line segment that is final edge (that is orthogonal polygonal line)
 * There are always add least 3 points (port, channel, port)
 * For each bend (channel crossing) there is additional point
 *                channel
 *      -----------------------------
 *      | -port0                    | - port1
 * ------------                 -----------
 *    Node 0                        Node 1
 * 
 *  The route has 3 nodes but in this case there are 4 points in final polygonal line
 *  If bend points (channel crossings) are present the number of points is equal to number of nodes in the route
 *                         |------
 *                    +----| port1 Node 1
 *                    |    |-------
 *          bend      |
 *      --------------+ channel crossing 
 *      | -port0
 * ------------
 *    Node 0
 * 
 *  In this case the route has 5 nodes (port0, channel-v0, bend point(0,0), channel-h0, port1) and 5 points in final polygonal line
 *  And one bend direction (UpLeft). So for channel-v0 (vertical channel 0) then connector Side is Top. For channel-h0 (horizontal channel 0) the connector Side is Left.
 */ 
pub struct AbstractEdgeRoute {
    pub from: usize,
    pub to: usize,
    pub route: Vec<usize>,
    // For each bend point in the route there is a bend direction
    // It is useful to compute the side of channel connectors later
    pub bend_directions: Vec<BendDirection>
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BendDirection {
    UpRight,
    UpLeft,
    DownRight,
    DownLeft,
}

impl BendDirection {
    pub fn side_for_orientation(&self, orientation: Orientation) -> Side {
        match orientation {
            Orientation::Vertical => {
                match self {
                    BendDirection::UpRight | BendDirection::DownRight => Side::Right,
                    BendDirection::UpLeft | BendDirection::DownLeft => Side::Left,
                }
            },
            Orientation::Horizontal => {
                match self {
                    BendDirection::UpRight | BendDirection::UpLeft => Side::Top,
                    BendDirection::DownRight | BendDirection::DownLeft => Side::Bottom,
                }
            }
        }
    }
}

impl RoutingGraph {
    pub fn add_edge(&mut self, from: usize, to: usize) {
        self.nodes[from].neighbors.push(to);
        self.nodes[to].neighbors.push(from);
    }

    pub fn channel(&self, channel_idx: usize, orientation: Orientation) -> &RChannel {
        match orientation {
            Orientation::Vertical => &self.vchannels[channel_idx],
            Orientation::Horizontal => &self.hchannels[channel_idx],
        }
    }

    pub fn channel_idx(&self, channel_idx: usize, orientation: Orientation) -> usize {
        match orientation {
            Orientation::Vertical => channel_idx,
            Orientation::Horizontal => self.vchannels.len() + channel_idx,
        }
    }

    pub fn bend_iterator(&self, channel_idx: usize, orientation: Orientation) -> impl Iterator<Item=usize> {
        let mut start_idx = self.nodes_len + self.nodes_len * 4;
        iter::from_fn(move || {
            while start_idx < self.nodes.len() {
                let idx = start_idx;
                let node_type = &self.nodes[idx].node_type;
                match node_type {
                    RNodeType::BendPoint(v_idx,h_idx) => {
                        start_idx += 1;
                        match orientation {
                            Orientation::Vertical if *v_idx == channel_idx => return Some(*h_idx),
                            Orientation::Horizontal if *h_idx == channel_idx => return Some(*v_idx),
                            _ => {}
                        }
                    },
                    _ => {
                        start_idx = self.nodes.len();
                    }
                }
            }
            None
        })
    }

    pub fn channel_by_id(&self, channel_idx: usize) -> &RChannel {
        if channel_idx >= self.vchannels.len() {
            &self.vchannels[channel_idx - self.vchannels.len()]
        } else {
            &self.vchannels[channel_idx]
        }
    }
}




#[derive(Debug)]
pub struct NodePort {
    pub node_id: usize,
    pub side: Side,
}

impl NodePort {
    pub fn position(&self, node_rect: &ERect) -> Pos2 {
        match self.side {
            Side::Right => Pos2::new(node_rect.max.x, node_rect.center().y),
            Side::Left => Pos2::new(node_rect.min.x, node_rect.center().y),
            Side::Top => Pos2::new(node_rect.center().x, node_rect.min.y),
            Side::Bottom => Pos2::new(node_rect.center().x, node_rect.max.y),
        }
    }
    pub fn channel_position(&self, node_rect: &ERect) -> f32 {
        match self.side {
            Side::Right | Side::Left=> node_rect.center().y,
            Side::Top | Side::Bottom => node_rect.center().x,
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
struct OppositePort {
    node_id_ls: usize,
    node_id_gt: usize,
    channel_idx: usize,
    side_ls: Side,
    side_gt: Side,
}


pub fn create_routing_graph(boxes: &[ERect]) -> RoutingGraph {
    let (mut vchannels, mut hchannels) = build_channels(boxes);
    let mut cross_points: Vec<(usize,usize,Pos2)> = Vec::new();
    let mut rnodes: Vec<RNode> = Vec::new();
    let mut redges: Vec<REdge> = Vec::new();
    for (node_id, _b) in boxes.iter().enumerate() {
        let rnode = RNode::from_type(RNodeType::Node(node_id));
        rnodes.push(rnode);
    }
    for (channel_idx,channel) in vchannels.iter_mut().enumerate() {
        for port in channel.ports.iter_mut() {
            match port.port_type {
                ChannelPortType::NodePort{node_id, side} => {
                    rnodes.push(RNode::from_type(RNodeType::Port(node_id, channel_idx, side)));
                    redges.push(REdge{from: node_id, to: rnodes.len()-1});
                    port.rnode_id = rnodes.len() - 1;
                }
                _ => {}
            }
        }
    }
    for (channel_idx,channel) in hchannels.iter_mut().enumerate() {
        for port in channel.ports.iter_mut() {
            match port.port_type {
                ChannelPortType::NodePort{node_id, side} => {
                    rnodes.push(RNode::from_type(RNodeType::Port(node_id, channel_idx, side)));
                    redges.push(REdge{from: node_id, to: rnodes.len()-1});
                    port.rnode_id = rnodes.len()-1;
                },
                _ => {}
            }
        }
    }
    for (vindex,vchannel) in vchannels.iter_mut().enumerate() {
        for (hindex, hchannel) in hchannels.iter_mut().enumerate() {
            if vchannel.rect.intersects(hchannel.rect) {
                let intersection = vchannel.rect.intersect(hchannel.rect);
                let bend_node_idx = rnodes.len();
                vchannel.ports.push(ChannelPort {
                    position: intersection.center().y,
                    port_type: ChannelPortType::Bend{ channel_id: hindex },
                    rnode_id: bend_node_idx,
                });
                hchannel.ports.push(ChannelPort {
                    position: intersection.center().x,
                    port_type: ChannelPortType::Bend{ channel_id: vindex },
                    rnode_id: bend_node_idx
                });
                cross_points.push((vindex,hindex, intersection.center()));
                let rnode = RNode::from_type(RNodeType::BendPoint(vindex,hindex));
                rnodes.push(rnode);
                // redges.push(REdge{from: bend_node_idx, to: vchannel_offset + vindex});
                // redges.push(REdge{from: bend_node_idx, to: hchannel_offset + hindex});
            }
        }
    }
    // First create edges along the channels, because they straight forward should be visited first
    vchannels.iter_mut().for_each(|c| {
        c.ports.sort_unstable_by(|a,b| a.position.partial_cmp(&b.position).unwrap());
        if c.ports.len() >= 2 {
            let mut from_rnode_idx = c.ports[0].rnode_id;
            for port in c.ports.iter().skip(1) {
                let to_rnode_idx = port.rnode_id;
                redges.push(REdge{from: from_rnode_idx, to: to_rnode_idx});
                from_rnode_idx = to_rnode_idx;
            }
        }
    });
    hchannels.iter_mut().for_each(|c| {
        c.ports.sort_unstable_by(|a,b| a.position.partial_cmp(&b.position).unwrap());
        if c.ports.len() >= 2 {
            let mut from_rnode_idx = c.ports[0].rnode_id;
            for port in c.ports.iter().skip(1) {
                let to_rnode_idx = port.rnode_id;
                redges.push(REdge{from: from_rnode_idx, to: to_rnode_idx});
                from_rnode_idx = to_rnode_idx;
            }
        }
    });

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
pub fn route_edges(routing_graph: &RoutingGraph, edges: &[Edge], boxes: &[ERect]) -> Vec<AbstractEdgeRoute> {
    let mut edges_routes: Vec<AbstractEdgeRoute> = Vec::new();
    let mut edges: Vec<(usize,usize)> = edges.into_iter()
        .filter(|edge| edge.from != edge.to)
        .map(|edge| {
        if edge.from > edge.to {
            (edge.to, edge.from)
        } else {
            (edge.from, edge.to)
        }
    }).collect();
    edges.sort_unstable_by(|(from_a,to_a),(from_b,to_b)| {
        from_a.cmp(from_b).then(to_a.cmp(to_b))
    });
    edges.dedup();
    let mut targets: Vec<usize> = Vec::new();
    let mut last_from = usize::MAX;
    for (from, to) in edges.iter() {
        if last_from != *from {
            if !targets.is_empty() {
                route_edges_from(routing_graph, last_from, &targets, &mut edges_routes, boxes);
            }
            last_from = *from;
            targets.clear();
        }
        targets.push(*to);
    }
    route_edges_from(routing_graph, last_from, &targets, &mut edges_routes, boxes);
    edges_routes.sort_unstable_by(|a,b| a.from.cmp(&b.from).then(a.to.cmp(&b.to)));
    edges_routes
}

fn route_edges_from(routing_graph: &RoutingGraph, from: usize, to: &[usize], edges_routes: &mut Vec<AbstractEdgeRoute>, boxes: &[ERect]) {
    // make bfs from from node to all to nodes.
    let mut visited: Vec<bool> = vec![false; routing_graph.nodes.len()];
    let mut predecessor: Vec<usize> = vec![usize::MAX; routing_graph.nodes.len()];
    let mut queue: VecDeque<(usize,Orientation)> = VecDeque::new();
    visited[from] = true;
    for &n in routing_graph.nodes[from].neighbors.iter() {
        let n_route = &routing_graph.nodes[n];
        match n_route.node_type {
            RNodeType::Port(_node_id, _channel_id, side) => {
                let orientation = side.orientation();
                predecessor[n] = from;
                queue.push_back((n,orientation));
            },
            _ => {
                panic!("invalid routing graph structure, node should be connected only to ports");
            }
        }
    }
    let mut to_find = to.len();
    // This could be optimized be using A* search with heuristic
    // The heuristic could be vector distance between current and target node (because we are in 2D space)
    while let Some((node_idx,orientation)) = queue.pop_front() {
        visited[node_idx] = true;
        let node = routing_graph.nodes.get(node_idx).unwrap();
        match node.node_type {
            RNodeType::Node(node_index) if node_index != from => {
                if to.contains(&node_index) {
                    // found route
                    let mut route: Vec<usize> = Vec::new();
                    let mut current = node_idx;
                    while let Some(&prev) = predecessor.get(current) {
                        current = prev;
                        if prev == from {
                            break;
                        }
                        route.push(current);
                    }
                    route.reverse();
                    /*
                    println!("route from {} to {} before cleanup",from, node_idx);
                    for r in route.iter() {
                        let node = &routing_graph.nodes[*r];
                        println!(" elem {:?}",node.node_type);
                    }
                     */
                    remove_no_bend_edges(&mut route, routing_graph);
                    /*
                    println!("route from {} to {}",from, node_idx);
                    for r in route.iter() {
                        let node = &routing_graph.nodes[*r];
                        println!(" elem {:?}",node.node_type);
                    }
                     */

                    let bend_directions: Vec<BendDirection> = compute_bend_directions(&route, routing_graph, boxes);
                    edges_routes.push(AbstractEdgeRoute {
                        from,
                        to: node_index,
                        route,
                        bend_directions,
                    });
                    // If already all routes found exit
                    to_find -= 1;
                    if to_find == 0 {
                        break;
                    }
                }
            },
            _ => {
                let mut was_skip = false;
                let current_channel_orientation = node.node_type.get_channel_id(orientation);
                for target in node.neighbors.iter() {
                    if !visited[*target] {
                        let target_node = &routing_graph.nodes[*target];
                        // First chose the straight channel continuation for bend nodes
                        let visit_first = match target_node.node_type {
                            RNodeType::BendPoint(_,_) | RNodeType::Port(_,_,_) => {
                                let new_channel_orientation = target_node.node_type.get_channel_id(orientation);
                                current_channel_orientation.channel_idx == new_channel_orientation.channel_idx
                            },
                            _ => { true }
                        };
                        if visit_first { 
                            visited[*target] = true;
                            predecessor[*target] = node_idx;
                            queue.push_back((*target,orientation));
                        } else {
                            was_skip = true;
                        }
                    }
                }
                if was_skip {
                    let orientation = orientation.opposite();
                    for target in node.neighbors.iter() {
                        if !visited[*target] {
                            visited[*target] = true;
                            predecessor[*target] = node_idx;
                            queue.push_back((*target,orientation));
                        }
                    }
                }
            }            
        }
    }
    if to_find != 0 {
        panic!("not all routes found");
    }
}

/*
route from 1 to 4 before cleanup

 elem Port(1, 1, Right) V
 elem Port(3, 1, Left)  V
 elem BendPoint(1, 3) 
 elem BendPoint(1, 2)
 elem Port(3, 2, Top)
 elem Port(9, 2, Bottom)
 elem BendPoint(2, 2)
 elem Port(4, 2, Left)

 Should be filtered to

 elem Port(1, 1, Right) Vertical
 elem BendPoint(1, 2) Horizontal
 elem BendPoint(2, 2) Vertical
 elem Port(4, 2, Left) Vertical (no change)

*/
fn remove_no_bend_edges(route: &mut Vec<usize>, routing_graph: &RoutingGraph) {
    if route.len()>=2 {
        // Remove bend point and ports on same channel. So we have only one line segment per channel change
        let mut channel_orientation = routing_graph.nodes[route[0]].node_type.get_channel_ref().unwrap();
        let mut idx = 1;
        loop {
            if idx >= route.len()-1 {
                break;
            }
            // if the element is port just remove it
            let current_node = &routing_graph.nodes[route[idx]];
            if matches!(current_node.node_type, RNodeType::Port(_,_,_)) {
                // port remove
                route.remove(idx);
                continue;
            }
            let current_channel_orientation = current_node.node_type.get_channel_id(channel_orientation.orientation);
            let next_node = &routing_graph.nodes[route[idx+1]];
            let next_channel_orientation = next_node.node_type.get_channel_id(channel_orientation.orientation);
            if current_channel_orientation == next_channel_orientation {
                // same channel remove current node
                route.remove(idx);
                continue;
            }
            // so it is real bend, we need to switch on next channel and orientation
            channel_orientation = current_node.node_type.get_channel_id(channel_orientation.orientation.opposite());
            idx += 1;
        }
    }
}

fn compute_bend_directions(route: &Vec<usize>, routing_graph: &RoutingGraph, boxes: &[ERect]) -> Vec<BendDirection> {
    let mut bend_directions: Vec<BendDirection> = Vec::new();
    // To compute bend direction we need to look for each point before and after the bend
    // This point is either another bend and port in the channel
    if route.len() <= 2 {
        return bend_directions;
    }
    let mut current_pos = 1; // the first bend point can be at index 1
    let port_node = &routing_graph.nodes[route[0]];
    let mut last_pos: Pos2 = Pos2::ZERO;
    let mut last_orientation = Orientation::Vertical;
    match port_node.node_type {
        RNodeType::Port(node_id, channel_id, side) => {
            let port = NodePort { node_id: node_id, side: side };
            let rect = &boxes[node_id];
            let port_pos = port.position(rect);
            last_orientation = side.orientation();
            let channel = routing_graph.channel(channel_id, last_orientation);
            last_pos = channel.point_on_representative(port_pos);
        },
        _ => {
            panic!("invalid route (does not start with port)")
        }
    }
    let next_point = | npos: usize | -> Pos2 {
        let next_node = &routing_graph.nodes[route[npos]];
        match &next_node.node_type {
            RNodeType::Port(node_id,channel_id, side ) => {
                let channel = routing_graph.channel(*channel_id, side.orientation());
                let port = NodePort { node_id: *node_id, side: *side };
                let rect = &boxes[*node_id];
                let port_pos = port.position(rect);
                channel.point_on_representative(port_pos)
            },
            RNodeType::BendPoint(vindex,hindex) => {
                let vchannel_rect = routing_graph.vchannels[*vindex].rect;                
                let hchannel_rect = routing_graph.hchannels[*hindex].rect;              
                vchannel_rect.intersect(hchannel_rect).center()                
            },
            _ => {
                panic!("invalid route (expected port or bend point)")
            }
        }
    };
    loop {
        let bend_node = &routing_graph.nodes[route[current_pos]];
        match &bend_node.node_type {
            RNodeType::BendPoint(vindex,hindex) => {
                let next_pos = next_point(current_pos + 1);
                let bend_dir = bend_direction(last_pos, next_pos, last_orientation);
                last_orientation = last_orientation.opposite();
                bend_directions.push(bend_dir);
                let vchannel_rect = routing_graph.vchannels[*vindex].rect;                
                let hchannel_rect = routing_graph.hchannels[*hindex].rect;              
                last_pos = vchannel_rect.intersect(hchannel_rect).center();                
            },
            _ => {
                break;
            }
        }
        current_pos += 1;
    }
    bend_directions
}

// Orientation of first channel orientation from from_pos to to_pos
//
//     from_pos
//      |                    - Orientation: vertical
//      +---------to_pos
//
// bend is UpRight
// 
fn bend_direction(from_pos: Pos2, to_pos: Pos2, orientation: Orientation) -> BendDirection {
    let relative = to_pos - from_pos;
    match orientation {
        Orientation::Horizontal => {
            if relative.x >= 0.0 && relative.y <= 0.0 {
                BendDirection::UpLeft
            } else if relative.x < 0.0 && relative.y <= 0.0 {
                BendDirection::UpRight
            } else if relative.x >= 0.0 && relative.y > 0.0 {
                BendDirection::DownLeft
            } else {
                BendDirection::DownRight
            }
        }
        Orientation::Vertical => {
            if relative.x >= 0.0 && relative.y <= 0.0 {
                BendDirection::DownRight
            } else if relative.x < 0.0 && relative.y <= 0.0 {
                BendDirection::DownLeft
            } else if relative.x >= 0.0 && relative.y > 0.0 {
                BendDirection::UpRight
            } else {
                BendDirection::UpLeft
            }
        }
    }
}


/**
 * Only used for debugging purposes to get actual points from abstract routes
 */
pub fn map_abstract_routes(routing_graph: &RoutingGraph, boxes: &[ERect], routes: &Vec<AbstractEdgeRoute>) -> Vec<Vec<Pos2>> {
    routes.iter().map(|route| {
        let mut points: Vec<Pos2> = Vec::new();
        let mut is_first_port = true;
        for node_idx in route.route.iter() {
            let node = &routing_graph.nodes[*node_idx];
            match &node.node_type {
                RNodeType::Port(node_index, channel_id, side) => {
                    // channel to port connection
                    let rect = &boxes[*node_index];
                    let port = NodePort { node_id: *node_index, side: *side };
                    let port_pos = port.position(rect);
                    let channel = routing_graph.channel(*channel_id, side.orientation());
                    let channel_point = channel.point_on_representative(port_pos);
                    if is_first_port {
                        points.push(port_pos);
                        points.push(channel_point);                       
                        is_first_port = false;
                    } else {
                        points.push(channel_point);                       
                        points.push(port_pos);
                    }
                },
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

pub fn map_routes_to_segments(routing_graph: &RoutingGraph, boxes: &[ERect], routes: &Vec<AbstractEdgeRoute>, edge_routing: &GraphEdgeRouting) -> Vec<Vec<Pos2>> {
    let mut all_segments: Vec<Vec<Pos2>> = Vec::new();
    for edge_route in edge_routing.edge_routes.iter() {
        let abstract_route = &routes[edge_route.abstract_route];
        let mut points: Vec<Pos2> = Vec::new();
        let mut was_port = false;
        let mut last_channel: Option<(usize,Orientation)> = None;
        for (point_idx, rnode_idx) in abstract_route.route.iter().enumerate() {
            let node = &routing_graph.nodes[*rnode_idx];
            match &node.node_type {
                RNodeType::Port(node_index, channel_id, side) => {
                    // channel to port connection
                    let rect = &boxes[*node_index];
                    let port = NodePort { node_id: *node_index, side: *side };
                    let port_pos = port.slot_position(rect, edge_route.port_slots[point_idx], edge_routing.port_slots[*node_index * 4 + (*side as usize)]);
                    let orientation = side.orientation();
                    let total_channel_slots = match orientation {
                        Orientation::Vertical => edge_routing.channel_slots[*channel_id],
                        Orientation::Horizontal => edge_routing.channel_slots[*channel_id+routing_graph.vchannels.len()],
                    };
                    let last_channel_slot = if was_port {
                        edge_route.channel_slots[point_idx-1]
                    } else {
                        edge_route.channel_slots[point_idx]
                    };
                    let channel = routing_graph.channel(*channel_id, orientation);
                    let channel_point: Pos2 = channel.slot_position(port_pos, last_channel_slot, total_channel_slots);
                    dbgorth!("end port aroute={}-{} channel-slot:{}", abstract_route.from, abstract_route.to, last_channel_slot );
                    if was_port {
                        points.push(channel_point);
                        points.push(port_pos);
                    } else {
                        points.push(port_pos);
                        points.push(channel_point);
                        was_port = true;
                        last_channel = Some((*channel_id,orientation));
                    }
                },
                RNodeType::BendPoint(vindex,hindex) => {
                    if let Some((last_channel_id,orientation)) = last_channel.take() {
                        let channel_slot = edge_route.channel_slots[point_idx -1];
                        let (crossing_channel_id, vslot, hslot) = if orientation == Orientation::Vertical {
                            (*hindex, channel_slot, edge_route.channel_slots[point_idx])
                        } else {
                            (*vindex, edge_route.channel_slots[point_idx], channel_slot)
                        };
                        dbgorth!("edge route vindex={} hindex={} last_channel_slot={} last_channel_id={} vslot={} hslot={}", vindex, hindex, channel_slot, last_channel_id, vslot, hslot);
                        let vtotal_channel_slots = edge_routing.channel_slots[*vindex];
                        let vchannel = &routing_graph.vchannels[*vindex];
                        let v_cross_point = vchannel.slot_position(Pos2::ZERO, vslot, vtotal_channel_slots);

                        let htotal_channel_slots = edge_routing.channel_slots[*hindex + routing_graph.vchannels.len()];
                        let hchannel = &routing_graph.hchannels[*hindex];
                        let h_cross_point = hchannel.slot_position(Pos2::ZERO, hslot, htotal_channel_slots);

                        let cross_point = Pos2::new(v_cross_point.x, h_cross_point.y);
                        points.push(cross_point);
                        last_channel = Some((crossing_channel_id,orientation.opposite()));
                    }                   
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

    use crate::layoutalg::ortho::routing_slots::{calculate_edge_routes, create_channel_connectors};

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
        assert!(!routing_graph.nodes.is_empty());
        assert!(!routing_graph.vchannels.is_empty());
        assert!(!routing_graph.hchannels.is_empty());
        for (node_idx, node) in routing_graph.nodes.iter().enumerate() {
            println!("Node:{} - {:?}", node_idx, node);
        }
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

    #[test]
    fn 
    ing() -> Result<(), Box<dyn std::error::Error>> {
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
        ].iter().map(|(from,to)| Edge{from:*from, to:*to, predicate: 0, bezier_distance: 0.0}).collect::<Vec<_>>();

        let routes = route_edges(&routing_graph, &edges, &boxes);
        for route in routes.iter() {
            println!("route for edge {}->{}", route.from, route.to);
            // We should have only easy routes with no bends in this test
            assert!(route.route.len() == 2);
            for node_idx in route.route.iter() {
                let node = &routing_graph.nodes[*node_idx];
                println!(" idx: {} {:?}",*node_idx, node);
            }
        }
        assert_eq!(routes.len(), edges.len());
        
        let mut channel_connectors = create_channel_connectors(&routing_graph, &boxes);
        let graph_edge_routes = calculate_edge_routes(&routing_graph, &mut channel_connectors, &edges, &routes, &boxes);
        assert_eq!(graph_edge_routes.edge_routes.len(), edges.len());

        let route_segments = map_routes_to_segments(&routing_graph, &boxes, &routes, &graph_edge_routes);
        assert_eq!(route_segments.len(), edges.len());

        let out_dir = PathBuf::from("target/test-output");
        fs::create_dir_all(&out_dir)?;
        let rects = test_rects();
        let svg_path = out_dir.join("graph_routes.svg");
        let root = SVGBackend::new(&svg_path, (200, 200)).into_drawing_area();
        root.fill(&WHITE)?;

        draw_rects(&root, &rects, &BLACK)?;

        for segments in route_segments {
            // There should be at least 2 point per route
            assert!(segments.len() >= 2);
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

    #[test]
    fn test_bend_direction() {
        //  (0,60)-------------------------- (110,60)
        // |        2         |     1        |
        // |                  |              |
        //  ----------------(10,120)----------
        // |        4          |     3       |
        // |                   |             |
        //  (0,150)-------------------------- (110,150)
        
        let dir = bend_direction(egui::pos2(10.0,120.0), egui::pos2(110.0,60.0), Orientation::Horizontal);
        assert_eq!(dir, BendDirection::UpLeft);
        let dir = bend_direction(egui::pos2(110.0,60.0), egui::pos2(10.0,120.0), Orientation::Horizontal);
        assert_eq!(dir, BendDirection::DownRight);
        let dir = bend_direction(egui::pos2(10.0,120.0), egui::pos2(110.0,150.0), Orientation::Horizontal);
        assert_eq!(dir, BendDirection::DownLeft);
        let dir = bend_direction(egui::pos2(10.0,120.0), egui::pos2(0.0,150.0), Orientation::Horizontal);
        assert_eq!(dir, BendDirection::DownRight);
        let dir = bend_direction(egui::pos2(10.0,120.0), egui::pos2(0.0,60.0), Orientation::Horizontal);
        assert_eq!(dir, BendDirection::UpRight);

        let dir = bend_direction(egui::pos2(10.0,120.0), egui::pos2(110.0,60.0), Orientation::Vertical);
        assert_eq!(dir, BendDirection::DownRight);
        let dir = bend_direction(egui::pos2(110.0,60.0), egui::pos2(10.0,120.0), Orientation::Vertical);
        assert_eq!(dir, BendDirection::UpLeft);
        let dir = bend_direction(egui::pos2(10.0,120.0), egui::pos2(110.0,150.0), Orientation::Vertical);
        assert_eq!(dir, BendDirection::UpRight);
        let dir = bend_direction(egui::pos2(10.0,120.0), egui::pos2(0.0,150.0), Orientation::Vertical);
        assert_eq!(dir, BendDirection::UpLeft);
        let dir = bend_direction(egui::pos2(10.0,120.0), egui::pos2(0.0,60.0), Orientation::Vertical);
        assert_eq!(dir, BendDirection::DownLeft);
    }


}