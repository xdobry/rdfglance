use std::ops::Range;

use egui::{Pos2, Rect as ERect, emath::easing::cubic_in};

use crate::{dbgorth, layoutalg::ortho::{channels::ChannelPortType, route_sorting::TopologyRouting, routing::{BendDirection, NodePort}}, uistate::layout::Edge};

use super::routing::{Orientation, Side, RoutingGraph, RNodeType, AbstractEdgeRoute, RNode};

// Channel connector can be either port or bend (right and left for vertical, top and bottom for horizontal)
// The connector are ordered along the channel. I have index to compute circular gab between connectors 
// The is needed to compute ordering of edges
//
//    AL     PL2    BL  PL3  CL
//    -------------------------
//    AR     PR0    BR       CR
// 
//    AL - Left Bend with Channel A
//    PR0 - Port Right with Node 0  
// 
//    Circular order: CR, BR, PR0, AR, AL, PL2, BL, CL, PR3
//    Order CR, CL, PL3, BR, BL, PR0, PL2, AR, AL (Right has priority)
//    
//    All leg (segment of edge that are in middle of channel, parallel to channel) 
//    has start end end point to channel connector and are assigned to edge
#[derive(Debug)]
pub struct ChannelConnector {
    slots: u16,
    connector_type: ChannelConnectorType,
    connector_side: ChannelConnectorSide,
    circular_index: usize,
    pos: f32,
}

#[derive(Debug)]
pub struct ChannelConnectors {
    connectors: Vec<ChannelConnector>,
    channel_offsets: Vec<usize>,
}

impl ChannelConnectors {
    fn add_connectors(&mut self, connectors: Vec<ChannelConnector>) {
        self.channel_offsets.push(self.connectors.len());
        self.connectors.extend(connectors);
    }
    pub fn connector_range(&self, channel_idx: usize) -> &[ChannelConnector] {
        let start = self.channel_offsets[channel_idx];
        let end = if channel_idx + 1 < self.channel_offsets.len() {
            self.channel_offsets[channel_idx + 1]
        } else {
            self.connectors.len()
        };
        &self.connectors[start..end]
    }
    fn connector_range_mut(&mut self, channel_idx: usize) -> &mut [ChannelConnector] {
        let start = self.channel_offsets[channel_idx];
        let end = if channel_idx + 1 < self.channel_offsets.len() {
            self.channel_offsets[channel_idx + 1]
        } else {
            self.connectors.len()
        };
        &mut self.connectors[start..end]
    }
    fn circular_distance(from_idx: usize, to_idx: usize, connectors: &[ChannelConnector]) -> u16 {
        let circular_idx_from = connectors[from_idx].circular_index;
        let circular_idx_to = connectors[to_idx].circular_index;
        let diff = circular_idx_from.abs_diff(circular_idx_to);
        if diff > connectors.len()/2 {
            (connectors.len()-diff) as u16
        } else {
            diff as u16
        }
    }
}

#[derive(Debug)]
enum ChannelConnectorType {
    Port(usize), // node id
    Bend(usize), // crossing channel id
}

#[derive(Debug,PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
enum ChannelConnectorSide {
    RightOrButton,
    LeftOrTop,
}

impl ChannelConnectorSide {
    fn from_side(side: Side) -> Self {
        match side {
            // Switch because channel orientation is different than port side
            Side::Right | Side::Bottom => ChannelConnectorSide::LeftOrTop,
            Side::Left | Side::Top => ChannelConnectorSide::RightOrButton,
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
struct ChannelLeg {
    start_connector: usize,
    end_connector: usize,
    edge_idx: usize,
    route_start_idx: usize,
    route_channel_idx: usize,
    route_end_idx: usize,
    circular_distance: u16,
    port_sides: PortSides,
    route_order: i32,
    // The ordering of lag in channel according to global route ordering
    // The global route ordering is top (direction left to right) or left (direction top to button), so always in the coordinate direction
    is_global_order: bool,
}

impl Ord for ChannelLeg {
    /**
     * The stacking ordering of legs in channel
     * It is different for global ordering. Because in the local ordering BothRighOrBottom are taking slots from another side of free slots range
     *     +--------+
     *     | +---+  |
     *     | |   |  |
     *     0 1 
     * Consider vertical channel. In local order is 1 before 0 because it takes slots from right range (so it takes 1 first)
     * In global order is 0 above 1. Because 0 is more towards 0.
     */
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.port_sides.cmp(&other.port_sides)
            .then(
                match self.port_sides {
                    PortSides::BothLeftOrTop | PortSides::BothRightOrBottom  => self.circular_distance.cmp(&other.circular_distance),
                    PortSides::ChangeUp => {
                        let (self_min,self_max) = if self.start_connector < self.end_connector {
                            (self.start_connector, self.end_connector)
                        } else {
                            (self.end_connector, self.start_connector)
                        };
                        let (other_min,other_max) = if other.start_connector < other.end_connector {
                            (other.start_connector, other.end_connector)
                        } else {
                            (other.end_connector, other.start_connector)
                        };
                        self_min.cmp(&other_min).then(self_max.cmp(&other_max))
                    },
                    PortSides::ChangeDown => {
                        let (self_min,self_max) = if self.start_connector < self.end_connector {
                            (self.start_connector, self.end_connector)
                        } else {
                            (self.end_connector, self.start_connector)
                        };
                        let (other_min,other_max) = if other.start_connector < other.end_connector {
                            (other.start_connector, other.end_connector)
                        } else {
                            (other.end_connector, other.start_connector)
                        };
                        other_max.cmp(&self_max).then(other_min.cmp(&self_min))
                    }
                }               
            .then(
                match self.port_sides {
                    PortSides::BothLeftOrTop | PortSides::ChangeUp | PortSides::ChangeDown => self.route_order.cmp(&other.route_order),
                    PortSides::BothRightOrBottom => other.route_order.cmp(&self.route_order),
                }
            )
        )
    }
}

impl PartialOrd for ChannelLeg {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl ChannelLeg {
    /**
     * global orders of legs in one channel.
     * It is almost the same order as in local order trait cmp but in exception of BothRightOrBottom legs
     */
    fn leg_relative_order(&self, other: &Self) -> std::cmp::Ordering {
        if self.port_sides == other.port_sides {
            match self.port_sides {
                PortSides::BothRightOrBottom => {
                    // Here the exception see picture in cmp method trait
                    self.cmp(other).reverse()
                },
                _ => {
                    self.cmp(other)
                }
            }
        } else {
            self.port_sides.stack_order().cmp(&other.port_sides.stack_order())
        }
    }
}

struct LegOrderState {
    current_is_global: bool,
}

impl LegOrderState {
    pub fn new(start_point: Pos2, end_point: Pos2) -> Self {
        Self {
            current_is_global: if start_point.x < end_point.x {
                true
            } else {
                if start_point.x == end_point.x {
                    start_point.y < end_point.y
                } else {
                    false
                }
            },
        }
    }
    // gives the ordering of current leg according to new bend direction
    // 
    //      +--------0 
    //      |  +-----1
    //      |  |
    //      0  1
    //  It is the DownRight bend. For this bend the order is not changing. So if is was global (toward 0,0) it still stay this
    //  The orientation or direction of the route is not important for order changing
    // 
    //   0-----+
    //   1--+  |
    //      |  |
    //      0  1
    //  For DownLeft the order is changing
    // 
    //  We need canonize the route direction and flip the values if the route is going against the global direction (so we will start with negative ordering)
    //  It depends of start and end point of route
    //  The standard direction is left to right (and down)
    pub fn is_global_order(&mut self, bend: BendDirection) -> bool {
        let mut is_global = match bend {
            BendDirection::UpRight | BendDirection::DownLeft => false,
            BendDirection::UpLeft | BendDirection::DownRight => true,
        };
        let last_current_global = self.current_is_global;
        if !self.current_is_global {
            is_global = !is_global;
        }
        self.current_is_global = is_global;
        last_current_global
    }

    pub fn is_current_global(&self) -> bool {
        self.current_is_global
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Copy, Clone)]
enum PortSides {
    BothLeftOrTop,
    ChangeUp, // Ports change the sites and the direction is Up when you go from left to right side (or top to down, always +) 
    ChangeDown, // Order of ChangeUp and ChangeDown legs is not important for crossing to each other, so they are separated
    BothRightOrBottom,
}

impl PortSides {
    fn stack_order(self) -> u16 {
        match self {
            PortSides::BothLeftOrTop => 0,
            PortSides::ChangeUp => 1,
            PortSides::ChangeDown => 2,
            PortSides::BothRightOrBottom => 3,
        }
    }
    
    fn from_sides(side_a: Side, side_b: Side, connector_a: usize, connector_b: usize) -> Self {
        assert_eq!(side_a.orientation(), side_b.orientation(), "sides must have same orientation");
        if side_a == side_b {
            match side_a {
                Side::Right | Side::Bottom => PortSides::BothRightOrBottom,
                Side::Left | Side::Top => PortSides::BothLeftOrTop,
            }
        } else {
            match (side_a, side_b) {
                (Side::Left, Side::Right) | (Side::Top, Side::Bottom) => {
                    if connector_a < connector_b {
                        PortSides::ChangeDown
                    } else {
                        PortSides::ChangeUp
                    }
                },
                (Side::Right, Side::Left) | (Side::Bottom, Side::Top) => {
                    if connector_a > connector_b {
                        PortSides::ChangeDown
                    } else {
                        PortSides::ChangeUp
                    }
                },
                _ => panic!("invalid side combination"),
            }
        }
    }
    fn pack_order(&self) -> ChannelConnectorSide {
        match self {
            PortSides::BothLeftOrTop | PortSides::ChangeDown | PortSides::ChangeUp => ChannelConnectorSide::LeftOrTop,
            PortSides::BothRightOrBottom  => ChannelConnectorSide::RightOrButton,
        }
    }
}

pub struct EdgeRoute {
    pub abstract_route: usize,
    pub edge_idx: usize,
    pub port_slots: Vec<u16>,
    pub channel_slots: Vec<u16>,
}

pub struct GraphEdgeRouting {
    pub edge_routes: Vec<EdgeRoute>,
    pub port_slots: Vec<u16>,
    // first vertical then horizontal channels
    // see channel_gid in RoutingGraph
    pub channel_slots: Vec<u16>,
}

struct RouteIterator<'a> {
    pos: usize,
    route_ids: &'a Vec<usize>
}

impl<'a> RouteIterator<'a> {
    fn from_route(route_ids: &'a Vec<usize>) -> Self {
        Self {
            pos: 0,
            route_ids
        }
    }
}

impl RouteIterator<'_> {   
    fn next_route<'a>(&mut self, routing_graph: &'a RoutingGraph) -> &'a RNode {
        let route_id = self.route_ids[self.pos];
        self.pos += 1;
        &routing_graph.nodes[route_id]
    }
    fn next_port(&mut self, routing_graph: &RoutingGraph) -> (usize, usize, Side) {
        let route_node = self.next_route(routing_graph);
        match route_node.node_type {
            RNodeType::Port(node_id, channel_id, side) => (node_id, channel_id, side),
            _ => {
                panic!("expect port node in route");
            }
        }
    }
    fn last_bend_channel(&self, routing_graph: &RoutingGraph, orientation: Orientation) -> usize {
        let bend_pos = self.pos-2;
        let route_id = self.route_ids[bend_pos];
        let route = &routing_graph.nodes[route_id];
        match route.node_type {
            RNodeType::BendPoint(vchannel_id, hchannel_id) => {
                if orientation == Orientation::Vertical {
                    hchannel_id
                } else {
                    vchannel_id
                }
            },
            _ => {
                panic!("expect bend point node in route");
            }
        }
    }
}

// Abstract route gives the nodes in the routing graph that connect one node to another
// In the concrete route for each edge the lines can not overlap so they need to be assigned to port and channel slots
// First we need to create all legs pro channel, sort them and assign to slots
//
//   +------------------------------------+
//   |   +----------+   +------------+    |
//   |   |          |   |            |    |
//   0   0          1   1            2    2
//   #####         ######            ######
//  In this case we have 3 legs in channel with 3 connectors (0,1,2) (leg is one line in channel that is part of edge polygonal line).
//  Connectors can be either port (connection to node from one side) or bend point (connection to another channel) that have also side.
//  The leg are (0-1), (1-2), (0-2). They need 2 slots in channel (because (0-1) and (1-2) can share same slot) and 2 slots for ports.
//  All slots has Port Sides (bothRightOrBottom)
//  It is important to assign concrete slots for channel and port so the edges does not overlap
//  The algorithm:
//   - compute of legs for channel with connector index, circular difference and port sides
//   - compute number of slots needed for each channel connector (just increment then visited)
//   - compute max slots needed for channel (It is the max number of legs between two connectors)
//   - sort legs by port sides and circular difference
//   - assign slots for each legs in sorted order
//   Depending the port side and direction of leg (right or left) the ports slots are assigned from left or right (so the range is used to consume slots from left or right).
//
//  Sorting legs:
//  The legs are sorted to be assign to slots this way that the crossing are minimized.
//  First there are 4 types of legs depending of port sides (See PortSides).
//  Each leg type has the side that the ports are assigned.
//  For example for vertical channel. The slot are numbered from left to right. 
//  If the channel has 5 slots. The slots are assigned from 0 to 4 from left. End from 4 to 0 from right.
//  If leg type is (Ports BothLeft) the assignments is from left. The legs with smallest circular distance is assigned first.
//   
//  The order for legs that change the ports side (RBtoLT and LTtoRB) the order is calculated depending the change direction to top (left to right) and
//  to bottom (left to right). If direction is top the leg with minimum port number is assigned first.
//      +----0
//      |+---1
//      ||
//      ||
//      ||
//   2--+|
//   3---+
//   The leg (2-0) is before (3-1) because 0<1
//   For direction to bottom the leg with maximum port number is assigned first
//   For legs with different directions the order is not important they can cross or not (independent on the order anyway)
pub fn calculate_edge_routes(routing_graph: &RoutingGraph,
    channel_connectors: &mut ChannelConnectors,
    edges: &[Edge], 
    routes: &[AbstractEdgeRoute],
    boxes: &[ERect]) -> GraphEdgeRouting {
    let mut port_slots = vec![0u16; routing_graph.nodes_len * 4];
    let channels_count = routing_graph.vchannels.len() + routing_graph.hchannels.len();
    let mut channels_legs: Vec<Vec<ChannelLeg>> = (0..channels_count).map(|_| Vec::new()).collect();
    let mut edge_routes: Vec<EdgeRoute> = Vec::new();

    // region: compute legs
    // let bend_start_idx = routing_graph.nodes_len + routing_graph.nodes_len * 4 + routing_graph.vchannels.len() + routing_graph.hchannels.len();
    // for node in routing_graph.nodes.iter().skip(bend_start_idx) {
    //    println!("Node: {:?}", node.node_type);
    // }

    for (edge_id, edge) in edges.iter().enumerate() {
        let (node_min, node_max)  = if edge.from < edge.to { (edge.from, edge.to) } else { (edge.to, edge.from) };
        let route_idx = routes.binary_search_by(|r| {
            r.from.cmp(&node_min).then(r.to.cmp(&node_max))
        });
        if let Ok(route_idx) = route_idx {
            let route = &routes[route_idx];
            assert!(route.route.len()>=2,"route needs at least 2 elements");
            let mut r_iter = RouteIterator::from_route(&route.route);
            // Each route start and end with port node.
            // So this is grammar of the route
            // port + bend* + port)
            // We need to transfer it to channel legs 
            // There is at least 1 leg + n legs for each bend
            let mut bend_point_idx = 0;
            let mut leg_order_state = LegOrderState::new(
                boxes[edge.from].center(),
                boxes[edge.to].center(),
            );
            let (start_node_id, channel_id, side_start) = r_iter.next_port(&routing_graph);  
            port_slots[start_node_id *4 + (side_start as usize)] += 1;
            let mut last_orientation = side_start.orientation();
            let node = r_iter.next_route(&routing_graph);
            match node.node_type {
                RNodeType::Port(end_node_id, channel_id, side_end ) => {
                    // Case there is only 2 port (1 leg)
                    port_slots[end_node_id *4 + (side_end as usize)] += 1;
                    let orientation = side_end.orientation();
                    let channel_gid = routing_graph.channel_idx(channel_id, orientation);
                    dbgorth!("Port2 side={:?} orientation={:?} channel_id={} channel_gid={}", side_end, orientation, channel_id, channel_gid);
                    let connectors = channel_connectors.connector_range_mut(channel_gid);
                    let start_connector = connector_port_position(connectors, start_node_id);
                    let end_connector = connector_port_position(connectors, end_node_id);
                    connectors[start_connector].slots += 1;
                    connectors[end_connector].slots += 1;
                    channels_legs[channel_gid].push(ChannelLeg { 
                        start_connector, end_connector, 
                        route_start_idx: 0,
                        route_channel_idx: 0,
                        route_end_idx: 1,
                        edge_idx: edge_id, 
                        circular_distance: ChannelConnectors::circular_distance(start_connector, end_connector, &connectors), 
                        port_sides: PortSides::from_sides(side_start.opposite(), side_end.opposite(), start_connector, end_connector),
                        route_order: 0,
                        is_global_order: true,
                    });
                },
                RNodeType::BendPoint(vchannel_id, hchannel_id) => {
                    // There can be another bends or channel and port
                    let channel_gid = routing_graph.channel_idx(channel_id, last_orientation);
                    let connectors = channel_connectors.connector_range(channel_gid);
                    let start_connector = connector_port_position(connectors, start_node_id);
                        // Now the need to find bend connector 
                    let crossing_channel_id = if last_orientation == Orientation::Vertical {
                        hchannel_id
                    } else {
                        vchannel_id
                    };
                    let side_end = route.bend_directions[bend_point_idx].side_for_orientation(last_orientation);
                    dbgorth!("Bend3 v: {} h: {} channel: {} orientation: {:?}",vchannel_id, hchannel_id, channel_id, last_orientation);
                    let end_connector = connector_bend_position(connectors, crossing_channel_id, side_end);
                    let connectors = channel_connectors.connector_range_mut(channel_gid);
                    connectors[start_connector].slots += 1;
                    connectors[end_connector].slots += 1;
                    channels_legs[channel_gid].push(ChannelLeg { 
                        start_connector, end_connector, 
                        route_start_idx: 0,
                        route_channel_idx: 0,
                        route_end_idx: 1,
                        edge_idx: edge_id, 
                        circular_distance: ChannelConnectors::circular_distance(start_connector, end_connector, &connectors), 
                        port_sides: PortSides::from_sides(side_start.opposite(), side_end, start_connector, end_connector),
                        route_order: 0,
                        is_global_order: leg_order_state.is_global_order(route.bend_directions[bend_point_idx]),
                    });
                    let mut last_channel_id = crossing_channel_id;
                    let mut last_bend_direction = route.bend_directions[bend_point_idx];
                    last_orientation = last_orientation.opposite();
                    bend_point_idx += 1;
                    loop {
                        let node = r_iter.next_route(&routing_graph);
                        match node.node_type {
                            RNodeType::Port(end_node_id, channel_id, side_end) => {
                                port_slots[end_node_id *4 + (side_end as usize)] += 1;
                                let channel_gid = routing_graph.channel_idx(channel_id, side_end.orientation());
                                let connectors = channel_connectors.connector_range_mut(channel_gid);
                                dbgorth!("Bend1 channel: {} orientation: {:?} last-channel-id {}",channel_id, side_end.orientation(), last_channel_id);
                                // last_channel_id is not needed bend input channel, because the last_channel_id is the same and current channel_id
                                let last_bend_channel = r_iter.last_bend_channel(&routing_graph, last_orientation);
                                let start_connector = connector_bend_position(connectors, last_bend_channel, last_bend_direction.side_for_orientation(last_orientation));
                                let end_connector = connector_port_position(connectors,end_node_id);
                                connectors[start_connector].slots += 1;
                                connectors[end_connector].slots += 1;
                                let side_start = last_bend_direction.side_for_orientation(last_orientation);
                                channels_legs[channel_gid].push(ChannelLeg { 
                                    start_connector, end_connector, 
                                    route_start_idx: r_iter.pos -2,
                                    route_channel_idx: r_iter.pos -2,
                                    route_end_idx: r_iter.pos -1,
                                    edge_idx: edge_id, 
                                    circular_distance: ChannelConnectors::circular_distance(start_connector, end_connector, &connectors), 
                                    port_sides: PortSides::from_sides(side_start, side_end.opposite(), start_connector, end_connector),
                                    route_order: 0,
                                    is_global_order: leg_order_state.is_current_global(),
                                });
                                break;
                            },
                            RNodeType::BendPoint(vchannel_id, hchannel_id) => {
                                // Update last channel and orientation
                                let crossing_channel_id = if last_orientation == Orientation::Vertical {
                                    hchannel_id
                                } else {
                                    vchannel_id
                                };
                                let channel_gid = routing_graph.channel_idx(last_channel_id, last_orientation);
                                let side_start = last_bend_direction.side_for_orientation(last_orientation);
                                let side_end = route.bend_directions[bend_point_idx].side_for_orientation(last_orientation);
                                let connectors = channel_connectors.connector_range_mut(channel_gid);
                                dbgorth!("Bend v_a: {} h: {} channel: {} orientation: {:?}",vchannel_id, hchannel_id, last_channel_id, last_orientation);
                                let last_bend_channel = r_iter.last_bend_channel(&routing_graph, last_orientation);
                                let start_connector = connector_bend_position(connectors, last_bend_channel, side_start);
                                dbgorth!("Bend v_b: {} h: {} cross_channel: {}",vchannel_id, hchannel_id, crossing_channel_id);
                                let end_connector = connector_bend_position(connectors, crossing_channel_id, side_end);
                                connectors[start_connector].slots += 1;
                                connectors[end_connector].slots += 1;
                                channels_legs[channel_gid].push(ChannelLeg { 
                                    start_connector, end_connector, 
                                    route_start_idx: r_iter.pos - 2,
                                    route_channel_idx:  r_iter.pos - 2,
                                    route_end_idx: r_iter.pos -1,
                                    edge_idx: edge_id, 
                                    circular_distance: ChannelConnectors::circular_distance(start_connector, end_connector, &connectors), 
                                    port_sides: PortSides::from_sides(side_start, side_end, start_connector, end_connector),
                                    route_order: 0,
                                    is_global_order: leg_order_state.is_global_order(route.bend_directions[bend_point_idx]),
                                });
                                last_bend_direction = route.bend_directions[bend_point_idx];
                                last_orientation = last_orientation.opposite();
                                last_channel_id = crossing_channel_id;
                                bend_point_idx += 1;
                            },
                            _ => {
                                panic!("expect bend or port");
                            }
                        }
                    }
                },
                _ => {
                    panic!("expect bend or port")
                }
            }
            edge_routes.push(EdgeRoute {
                abstract_route: route_idx,
                edge_idx: edge_id,
                port_slots: vec![0; route.route.len()],
                channel_slots: vec![0; route.route.len()]
            });
        } else {
            panic!("No route found for edge from {}-{}", edge.from, edge.to);
        }
    }
    // endregion: compute legs

    // region: Compute max slots for channel
    let mut channel_slots = vec![0u16; routing_graph.vchannels.len()+routing_graph.hchannels.len()];

    // Compute max number of legs between each between area for channel connectors   
    // the max_slots between connectors
    // the 0 means between connector 0 and 1 (so there is one value less then connectors)
    // So legs (2-1) (1-0) does not overlap. So max slots is 1
    for channel_gid in 0..channels_count {
        let connectors = channel_connectors.connector_range(channel_gid);
        let mut max_slots_between_connectors: Vec<u16> = vec![0; connectors.len()];
        for channel_leg in channels_legs[channel_gid].iter() {
            dbgorth!("gid {} leg {:?}",channel_gid,channel_leg);
            let start = channel_leg.start_connector;
            let end = channel_leg.end_connector;
            let (from, to) = if start < end {
                (start, end)
            } else {
                (end, start)
            };
            for idx in from..to {
                max_slots_between_connectors[idx] += 1;
            }
        }
        let max_slots = *max_slots_between_connectors.iter().max().unwrap_or(&0);
        channel_slots[channel_gid] = max_slots;
    }
    // endregion: Compute max slots for channel

    // Sort channel legs by side and circular distance or connector position (for side changing legs)
    let mut route_sort = TopologyRouting::new(edges.len());

    for channel_legs in channels_legs.iter_mut() {
        channel_legs.sort_unstable();

        // Compute for each segments that have ordering the global route ordering
        // So the routes do not cross if possible
        if channel_legs.len() > 1 {
            for i in 0..channel_legs.len()-1 {
                for j in i+1..channel_legs.len() {
                    let ri = &channel_legs[i];
                    let rj = &channel_legs[j];
                    let mut is_order_ij: bool = match ri.leg_relative_order(&rj) {
                        std::cmp::Ordering::Less => true,
                        std::cmp::Ordering::Greater => false,
                        std::cmp::Ordering::Equal => continue,
                    };
                    if !ri.is_global_order && !rj.is_global_order {
                        is_order_ij = !is_order_ij;
                    }
                    /*
                    if !rj.is_global_order {
                        is_order_ij = !is_order_ij;
                    }
                     */
                    if is_order_ij {
                        route_sort.add_route_ord(ri.edge_idx, rj.edge_idx);
                    } else {
                        route_sort.add_route_ord(rj.edge_idx, ri.edge_idx);
                    }
                }
            }
        }
    }
    
    let mut route_order: Vec<i32> = vec![0;edges.len()];
    let routes_ordered = route_sort.topological_sort();
    for (order, route_idx) in routes_ordered.iter().enumerate() {
        route_order[*route_idx] = order as i32;
    }
    #[cfg(feature = "debug-orth")]
    {
        println!("Route order: {:?}", routes_ordered);
        for (pos, r) in routes_ordered.iter().enumerate() {
            println!("  route pos={} r={} {}-{}", pos, *r, edges[*r].from, edges[*r].to);
        }
    }

    for channel_legs in channels_legs.iter_mut() {
        for channel_leg in channel_legs.iter_mut() {
            channel_leg.route_order = if channel_leg.is_global_order {
                route_order[channel_leg.edge_idx]
            } else {
                -route_order[channel_leg.edge_idx]
            };
        }
        channel_legs.sort_unstable();
    }

    // region: Assign slots for each bends in edge route
    for (channel_gid, channel_legs) in channels_legs.iter().enumerate() {
        if channel_legs.is_empty() {
            continue;
        }
        let connectors = channel_connectors.connector_range(channel_gid);
        let max_slots = channel_slots[channel_gid];
        // is could be that some ports has not slots so the range should be empty
        // if there are 3 slots the range should be 0..2 so there are 3 values (0,1,2) to consume from right or left
        let mut slots_between_connectors: Vec<Range<u16>> = (0..connectors.len()).map(|_| 0..max_slots.saturating_sub(1)).collect();
        let mut connectors_slots: Vec<Range<u16>> = connectors.iter().map(|c| 0..c.slots.saturating_sub(1)).collect();
        dbgorth!("channel legs sorted gid={}", channel_gid);
        for channel_leg in channel_legs.iter() {
            dbgorth!("{:?}",channel_leg);
            let start = channel_leg.start_connector;
            let end = channel_leg.end_connector;
            let (from, to) = if start < end {
                (start, end)
            } else {
                (end, start)
            };
            let free_slot = match channel_leg.port_sides.pack_order() {
                ChannelConnectorSide::LeftOrTop => {
                    // find free slot in the range        
                    let free_slot = slots_between_connectors[from..to].iter().map(|r | r.start).max().unwrap();
                    // update slots between connectors
                    for idx in from..to {
                        slots_between_connectors[idx].start = free_slot + 1;
                    }
                    free_slot
                },
                ChannelConnectorSide::RightOrButton => {
                    // find free slot in the range        
                    let free_slot = slots_between_connectors[from..to].iter().map(|r | r.end).min().unwrap();
                    // update slots between connectors
                    for idx in from..to {
                        slots_between_connectors[idx].end = free_slot.saturating_sub(1);
                    }
                    free_slot
                },
            };
            // assign slot to edge route
            let edge_route = &mut edge_routes[channel_leg.edge_idx];
            dbgorth!("  free slot git={} {} channel_idx: {} edge: {}",channel_gid, free_slot, channel_leg.route_channel_idx, channel_leg.edge_idx);
            edge_route.channel_slots[channel_leg.route_channel_idx] = free_slot;
            match channel_leg.port_sides {
                PortSides::BothLeftOrTop | PortSides::BothRightOrBottom => {
                    let start_port_slot = consume_slot(&mut connectors_slots[start], end>start);
                    dbgorth!("  port start slot:{} for connector:{} edge:{}", start_port_slot, start, channel_leg.edge_idx);
                    edge_route.port_slots[channel_leg.route_start_idx] = start_port_slot;

                    let end_port_slot = consume_slot(&mut connectors_slots[end], end<start);
                    edge_route.port_slots[channel_leg.route_end_idx] = end_port_slot;
                },
                PortSides::ChangeDown => {
                    if start > end {
                        // port on left side
                        let end_port_slot = consume_slot(&mut connectors_slots[end], end<start);
                        edge_route.port_slots[channel_leg.route_end_idx] = end_port_slot;
                    } else {
                        let start_port_slot = consume_slot(&mut connectors_slots[start], end>start);
                        dbgorth!("  port start slot:{} for connector:{} edge:{}", start_port_slot, start, channel_leg.edge_idx);
                        edge_route.port_slots[channel_leg.route_start_idx] = start_port_slot;
                    }
                },
                PortSides::ChangeUp => {
                    if start < end {
                        // port on left side
                        let end_port_slot = consume_slot(&mut connectors_slots[end], end<start);
                        edge_route.port_slots[channel_leg.route_end_idx] = end_port_slot;
                    } else {
                        let start_port_slot = consume_slot(&mut connectors_slots[start], end>start);
                        dbgorth!("  port start slot:{} for connector:{} edge:{}", start_port_slot, start, channel_leg.edge_idx);
                        edge_route.port_slots[channel_leg.route_start_idx] = start_port_slot;
                    }
                },
                _ => {
                },                
            }
        }
        // For port side changing legs the order of right port is another so we need to assign them in reverse order
        for channel_leg in channel_legs.iter().rev() {
            let start = channel_leg.start_connector;
            let end = channel_leg.end_connector;
            let edge_route = &mut edge_routes[channel_leg.edge_idx];
            match channel_leg.port_sides {
                PortSides::ChangeDown => {
                    if start < end {
                        // port on right side
                        let end_port_slot = consume_slot(&mut connectors_slots[end], end<start);
                        edge_route.port_slots[channel_leg.route_end_idx] = end_port_slot;
                    } else {
                        let start_port_slot = consume_slot(&mut connectors_slots[start], end>start);
                        dbgorth!("  port start slot:{} for connector:{} edge:{}", start_port_slot, start, channel_leg.edge_idx);
                        edge_route.port_slots[channel_leg.route_start_idx] = start_port_slot;
                    }
                }
                PortSides::ChangeUp => {
                    if start > end {
                        let end_port_slot = consume_slot(&mut connectors_slots[end], end<start);
                        edge_route.port_slots[channel_leg.route_end_idx] = end_port_slot;
                    } else {
                        let start_port_slot = consume_slot(&mut connectors_slots[start], end>start);
                        dbgorth!("  port start slot:{} for connector:{} edge:{}", start_port_slot, start, channel_leg.edge_idx);
                        edge_route.port_slots[channel_leg.route_start_idx] = start_port_slot;
                    }
                },
                _ => {
                },                
            }
        }
    }    
    // endregion: Assign slots for each bends in edge route

    GraphEdgeRouting {
        edge_routes,
        port_slots,
        channel_slots,
    }
}

fn connector_port_position(connectors: &[ChannelConnector], node_id: usize) -> usize {
    let res = connectors.iter().position(|c| matches!(c.connector_type, ChannelConnectorType::Port(n_id) if n_id == node_id));
    match res {
        Some(pos) => {
            pos
        },
        None => {
            #[cfg(feature = "debug-orth")]
            for c in connectors.iter() {
                println!("Connector: {:?}", c);
            }
            panic!("can not find port {}",node_id);
        }
    }
}

fn connector_bend_position(connectors: &[ChannelConnector], crossing_channel_id: usize, side: Side) -> usize {
    let connector_side = ChannelConnectorSide::from_side(side);
    let pos = connectors.iter().position(|c| 
        c.connector_side == connector_side &&
        matches!(c.connector_type, ChannelConnectorType::Bend(c_id) if c_id == crossing_channel_id));
    match pos {
        Some(pos) => {
            pos
        },
        None => {
            #[cfg(feature = "debug-orth")]
            for c in connectors.iter() {
                println!("Connector: {:?}", c);
            }
            panic!("can not find pos {}",crossing_channel_id);
        }
    }
}

fn consume_slot(range: &mut Range<u16>,right_direction: bool) -> u16 {
    if right_direction {
        let slot = range.end;
        if range.end > 0 {
            range.end -= 1;
        }
        slot
    } else {
        let slot = range.start;
        range.start += 1;
        slot
    }
}

// Channel connectors are position and side aware edge crossing points of channel.
// Edge can cross (bend) in a channel to another channel (bend point) or to port
pub fn create_channel_connectors(routing_graph: &RoutingGraph, boxes: &[ERect]) -> ChannelConnectors {
    let mut all_channel_connectors = ChannelConnectors {
        connectors: Vec::new(),
        channel_offsets: Vec::with_capacity(routing_graph.hchannels.len()+routing_graph.vchannels.len()),
    };
    for (channel_id, channel) in routing_graph.vchannels.iter().chain(routing_graph.hchannels.iter()).enumerate() {
        let mut channel_connectors: Vec<ChannelConnector> = Vec::new();
        for port in channel.ports.iter() {
            match port.port_type {
                ChannelPortType::NodePort{node_id, side} => {
                    let rect = &boxes[node_id];
                    let node_port = NodePort {
                        node_id,
                        side,
                    };
                    let pos = node_port.channel_position(rect);
                    channel_connectors.push(ChannelConnector { 
                        slots: 0, 
                        connector_type: ChannelConnectorType::Port(node_id),
                        connector_side: ChannelConnectorSide::from_side(side),
                        circular_index: 0, 
                        pos : pos,
                    });
                },
                _ => {},
            }
        }
        // TODO build bend connectors from channel ports information
        let channel_idx = match channel.orientation {
            Orientation::Vertical => channel_id,
            Orientation::Horizontal => channel_id - routing_graph.vchannels.len()
        };
        let channel_rect = routing_graph.channel(channel_idx, channel.orientation).rect;
        let orientation = channel.orientation;
        for cross_channel_id in routing_graph.bend_iterator(channel_idx, orientation) {
            let cross_channel_rect = routing_graph.channel(cross_channel_id, orientation.opposite()).rect;
            let cross_center = channel_rect.intersect(cross_channel_rect).center();
            let pos = match channel.orientation {
                Orientation::Vertical => cross_center.y,
                Orientation::Horizontal => cross_center.x,
            };
            channel_connectors.push(ChannelConnector {
                slots: 0,
                connector_type: ChannelConnectorType::Bend(cross_channel_id),
                connector_side: ChannelConnectorSide::RightOrButton,
                circular_index: 0,
                pos,
            });
            channel_connectors.push(ChannelConnector {
                slots: 0,
                connector_type: ChannelConnectorType::Bend(cross_channel_id),
                connector_side: ChannelConnectorSide::LeftOrTop,
                circular_index: 0,
                pos,
            });
        }
        channel_connectors.sort_unstable_by(|a,b| a.pos.partial_cmp(&b.pos).unwrap());
        let mut circular_index = 0;
        for connector in channel_connectors.iter_mut() {
            match connector.connector_side {
                ChannelConnectorSide::RightOrButton => {
                    connector.circular_index = circular_index;
                    circular_index += 1;
                },
                _ => {

                }
            }
        }
        for connector in channel_connectors.iter_mut().rev() {
            match connector.connector_side {
                ChannelConnectorSide::LeftOrTop => {
                    connector.circular_index = circular_index;
                    circular_index += 1;
                },
                _ => {

                }
            }
        }

        all_channel_connectors.add_connectors(channel_connectors);
    }
    all_channel_connectors
}

#[cfg(test)]
mod tests {
    use std::{fs, path::PathBuf, vec};

    use crate::layoutalg::ortho::{
        routing::{BendDirection, create_routing_graph, map_abstract_routes, map_routes_to_segments, route_edges}, 
        routing_slots::{calculate_edge_routes, create_channel_connectors}
    };

    use super::*;
    use plotters::{coord::Shift, prelude::*};

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
            root.draw_text(&text, &style, (rect.center().x as i32, rect.min.y as i32))?;
        }
        Ok(())
    }

    fn port_node_idx(routing_graph: &RoutingGraph, node_id: usize, side: Side) -> usize {
        for (node_idx, node) in routing_graph.nodes.iter().enumerate() {
            if let RNodeType::Port(p_node_id, _channel_id, p_side) = node.node_type {
                if p_node_id == node_id && p_side == side {
                    return node_idx;
                }
            }
        }
        panic!("No port node_id={} side={:?} found", node_id, side);
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

        // We build own abstract routes that only use one channel to better test
        // slot sorting and assignment
        let abstract_routes: Vec<AbstractEdgeRoute> = vec![
            AbstractEdgeRoute { 
                from: 0, to: 1, route: vec![
                    port_node_idx(&routing_graph, 0, Side::Bottom),
                    port_node_idx(&routing_graph, 1, Side::Bottom),
                ], bend_directions: vec![]
            },
            AbstractEdgeRoute { 
                from: 0, to: 2, route: vec![
                    port_node_idx(&routing_graph, 0, Side::Bottom),
                    port_node_idx(&routing_graph, 2, Side::Bottom),
                ], bend_directions: vec![]
            },
            AbstractEdgeRoute { 
                from: 0, to: 3, route: vec![
                    port_node_idx(&routing_graph, 0, Side::Bottom),
                    port_node_idx(&routing_graph, 3, Side::Top),
                ], bend_directions: vec![] 
            },
            AbstractEdgeRoute { 
                from: 1, to: 2, route: vec![
                    port_node_idx(&routing_graph, 1, Side::Bottom),
                    port_node_idx(&routing_graph, 2, Side::Bottom),
                ], bend_directions: vec![] 
            },
            AbstractEdgeRoute { 
                from: 2, to: 4, route: vec![
                    port_node_idx(&routing_graph, 2, Side::Bottom),
                    port_node_idx(&routing_graph, 4, Side::Top),
                ], bend_directions: vec![] 
            },            
            AbstractEdgeRoute { 
                from: 3, to: 4, route: vec![
                    port_node_idx(&routing_graph, 3, Side::Top),
                    port_node_idx(&routing_graph, 4, Side::Top),
                ], bend_directions: vec![] 
            },
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
            (0,3),
            (3,4),
            (2,4),
        ].iter().map(|(from,to)| Edge{from:*from, to:*to, predicate: 0, bezier_distance: 0.0}).collect::<Vec<_>>();

        let mut channel_connectors = create_channel_connectors(&routing_graph, &rects);
        let graph_edge_routes = calculate_edge_routes(&routing_graph, &mut channel_connectors, 
            &edges, &abstract_routes, &rects);
        assert_eq!(graph_edge_routes.edge_routes.len(), edges.len());

        let route_segments = map_routes_to_segments(&routing_graph, &rects, &abstract_routes, &graph_edge_routes);
        assert_eq!(route_segments.len(), edges.len());

        for segments in route_segments.iter() {
            let points = segments.iter().map(|p| (p.x as i32, p.y as i32)).collect::<Vec<_>>();
            root.draw(&PathElement::new(
                    points,
                    ShapeStyle::from(&RED).stroke_width(1),
                )
            )?;
        }

        let channel_idx = routing_graph.channel_idx(1, Orientation::Horizontal);
        for channel_connector in channel_connectors.connector_range(channel_idx).iter() {
            println!("Channel Connector: {:?}", channel_connector);   
        }
        Ok(())
    }

    fn bending_point_node_idx(routing_graph: &RoutingGraph, hchannel_id: usize, vchannel_id: usize) -> usize {
        for (node_idx, node) in routing_graph.nodes.iter().enumerate() {
            if let RNodeType::BendPoint(v_id, h_id) = node.node_type {
                if v_id == vchannel_id && h_id == hchannel_id {
                    return node_idx;
                }
            }
        }
        panic!("No bend point found for vchannel {} and hchannel {}", vchannel_id, hchannel_id);
    }


    #[test]
    fn test_edge_route_bend_slots() -> Result<(), Box<dyn std::error::Error>> {
        let rects = vec![
            ERect::from_min_max(egui::pos2(40.0, 40.0), egui::pos2(60.0, 60.0)),
            ERect::from_min_max(egui::pos2(80.0, 40.0), egui::pos2(100.0, 60.0)),
            ERect::from_min_max(egui::pos2(120.0, 40.0), egui::pos2(140.0, 60.0)),
            ERect::from_min_max(egui::pos2(80.0, 80.0), egui::pos2(100.0, 100.0)),
        ];

        let out_dir = PathBuf::from("target/test-output");
        fs::create_dir_all(&out_dir)?;
        let svg_path = out_dir.join("routes_blend_slots.svg");
        let root = SVGBackend::new(&svg_path, (200, 200)).into_drawing_area();
        root.fill(&WHITE)?;

        draw_rects(&root, &rects, &BLACK)?;

        let routing_graph = create_routing_graph(&rects);

        let abstract_routes: Vec<AbstractEdgeRoute> = vec![
            AbstractEdgeRoute { 
                from: 0, to: 1, route: vec![
                    port_node_idx(&routing_graph, 0, Side::Bottom),
                    bending_point_node_idx(&routing_graph, 1, 1),
                    port_node_idx(&routing_graph, 1, Side::Left),
                ], bend_directions: vec![BendDirection::UpLeft] 
            },
            AbstractEdgeRoute { 
                from: 0, to: 2, route: vec![
                    port_node_idx(&routing_graph, 0, Side::Bottom),
                    bending_point_node_idx(&routing_graph, 1, 1),
                    bending_point_node_idx(&routing_graph, 0, 1),
                    port_node_idx(&routing_graph, 2, Side::Top),
                ], bend_directions: vec![BendDirection::UpLeft, BendDirection::DownRight] 
            },
            AbstractEdgeRoute { 
                from: 0, to: 3, route: vec![
                    port_node_idx(&routing_graph, 0, Side::Bottom),
                    port_node_idx(&routing_graph, 3, Side::Top),
                ], bend_directions: vec![] 
            },
            AbstractEdgeRoute { 
                from: 1, to: 2, route: vec![
                    port_node_idx(&routing_graph, 1, Side::Left),
                    bending_point_node_idx(&routing_graph, 0, 1),
                    port_node_idx(&routing_graph, 2, Side::Top),
                ], bend_directions: vec![BendDirection::DownRight] 
            }
        ];

        let segments = map_abstract_routes(&routing_graph, &rects, &abstract_routes);

        for segments in segments.iter() {
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
            (0,3),
            (1,2),
        ].iter().map(|(from,to)| Edge{from:*from, to:*to, predicate: 0, bezier_distance: 0.0}).collect::<Vec<_>>();

        let mut channel_connectors = create_channel_connectors(&routing_graph, &rects);
        let graph_edge_routes = calculate_edge_routes(&routing_graph, &mut channel_connectors, 
            &edges, &abstract_routes, &rects);
        assert_eq!(graph_edge_routes.edge_routes.len(), edges.len());

        graph_edge_routes.edge_routes.iter().for_each(|er| {
            let abstract_route = &abstract_routes[er.abstract_route];
            println!("Edge Route: abstract_route {} {}-{}, port_slots {:?}, channel_slots {:?}", er.abstract_route, abstract_route.from, abstract_route.to, er.port_slots, er.channel_slots);
        });

        println!("channel slots {:?}",graph_edge_routes.channel_slots);
        println!("port slots {:?}",graph_edge_routes.port_slots);

        let route_segments = map_routes_to_segments(&routing_graph, &rects, &abstract_routes, &graph_edge_routes);
        assert_eq!(route_segments.len(), edges.len());

        let small_red_font = TextStyle::from(("sans-serif", 5).into_font()).color(&RED);
        for (r_id, segments) in route_segments.iter().enumerate() {
            let points = segments.iter().map(|p| (p.x as i32, p.y as i32)).collect::<Vec<_>>();
            let arr: [(i32, i32); 2] = points.iter().take(2).cloned().collect::<Vec<_>>()
                    .try_into()
                    .expect("vector must have exactly 2 elements");
            let label_pos = if arr[0].0 > arr[1].1 {
                (arr[0].0 + 1, arr[0].1 - 2)
            } else if arr[0].0 == arr[1].0 {
                if arr[0].1 > arr[1].1 {
                    (arr[0].0 - 1, arr[0].1 - 1)
                } else {
                    (arr[0].0 - 1, arr[0].1 - 4)
                }
            } else {
                (arr[0].0 - 4, arr[0].1 - 2)
            };
            root.draw_text(
                &format!("{}", r_id),
                &small_red_font,
                label_pos,
            )?;
            root.draw(&PathElement::new(
                    points,
                    ShapeStyle::from(&RED).stroke_width(1),           
                )
            )?;
        }

        Ok(())
    }

    #[test]
    fn test_bend_edges() -> Result<(), Box<dyn std::error::Error>> {
        let rects = vec![
            ERect::from_min_max(egui::pos2(20.0, 10.0), egui::pos2(40.0, 40.0)),
            ERect::from_min_max(egui::pos2(20.0, 45.0), egui::pos2(40.0, 75.0)),
            ERect::from_min_max(egui::pos2(20.0, 80.0), egui::pos2(40.0, 110.0)),
            
            ERect::from_min_max(egui::pos2(60.0, 30.0), egui::pos2(100.0, 90.0)),
            
            ERect::from_min_max(egui::pos2(120.0, 0.0), egui::pos2(140.0, 31.0)),
            ERect::from_min_max(egui::pos2(120.0, 40.0), egui::pos2(140.0, 50.0)),
            ERect::from_min_max(egui::pos2(120.0, 100.0), egui::pos2(140.0, 120.0)),
            
            ERect::from_min_max(egui::pos2(160.0, 25.0), egui::pos2(180.0, 55.0)),
            ERect::from_min_max(egui::pos2(120.0, 60.0), egui::pos2(140.0, 75.0)),

            ERect::from_min_max(egui::pos2(60.0, 2.0), egui::pos2(100.0, 20.0)),
            ERect::from_min_max(egui::pos2(60.0, 100.0), egui::pos2(100.0, 140.0)),
        ];

        let edges = vec![
            (1,5),
            (1,5),
            (1,4),
            (1,4),
            (1,6),
            (1,6),
            (1,8),
            (1,8),
            (1,9),
            (1,10),
            // (1,10),
            // (2,5),
            // (0,5),
            // (3,7),
        ].iter().map(|(from,to)| Edge{from:*from, to:*to, predicate: 0, bezier_distance: 0.0}).collect::<Vec<_>>();

        draw_graph(&rects, &edges, "routes_bends.svg")?;

        Ok(())
    }

    #[test]
    fn test_bend_edges2() -> Result<(), Box<dyn std::error::Error>> {
        let rects = vec![
            ERect::from_min_max(egui::pos2(120.0, 0.0), egui::pos2(140.0, 31.0)),
            ERect::from_min_max(egui::pos2(120.0, 40.0), egui::pos2(140.0, 50.0)),
            ERect::from_min_max(egui::pos2(120.0, 60.0), egui::pos2(140.0, 75.0)),
            
            ERect::from_min_max(egui::pos2(60.0, 30.0), egui::pos2(100.0, 90.0)),
            
            ERect::from_min_max(egui::pos2(20.0, 10.0), egui::pos2(40.0, 40.0)),
            ERect::from_min_max(egui::pos2(20.0, 45.0), egui::pos2(40.0, 75.0)),
            ERect::from_min_max(egui::pos2(20.0, 80.0), egui::pos2(40.0, 110.0)),
            
            ERect::from_min_max(egui::pos2(120.0, 100.0), egui::pos2(140.0, 120.0)), 
            ERect::from_min_max(egui::pos2(160.0, 25.0), egui::pos2(180.0, 55.0)),

            ERect::from_min_max(egui::pos2(60.0, 2.0), egui::pos2(100.0, 20.0)),

            ERect::from_min_max(egui::pos2(60.0, 100.0), egui::pos2(100.0, 140.0)),
        ];


        let edges = vec![
            (0,5),
            (0,5),
            (1,5),
            (1,5),
            (2,5),
            (2,5),
            (5,7),
            (5,7),
            (5,9),
            (5,10),
        ].iter().map(|(from,to)| Edge{from:*from, to:*to, predicate: 0, bezier_distance: 0.0}).collect::<Vec<_>>();

        draw_graph(&rects, &edges, "routes_bends2.svg")?;

        Ok(())
    }

    fn draw_graph(rects: &Vec<egui::Rect>, edges: &Vec<Edge>, out_file: &str)  -> Result<(), Box<dyn std::error::Error>> {

        let out_dir = PathBuf::from("target/test-output");
        fs::create_dir_all(&out_dir)?;
        let svg_path = out_dir.join(out_file);
        let root = SVGBackend::new(&svg_path, (200, 200)).into_drawing_area();
        root.fill(&WHITE)?;

        draw_rects(&root, &rects, &BLACK)?;



        let routing_graph = create_routing_graph(&rects);                
        let mut channel_connectors = create_channel_connectors(&routing_graph, &rects);
        let abstract_routes = route_edges(&routing_graph, &edges, &rects);

        let segments = map_abstract_routes(&routing_graph, &rects, &abstract_routes);
        for segments in segments.iter() {
            let points = segments.iter().map(|p| (p.x as i32, p.y as i32)).collect::<Vec<_>>();
            root.draw(&PathElement::new(
                    points,
                    ShapeStyle::from(&YELLOW).stroke_width(1),
                )
            )?;
        }

        let graph_edge_routes = calculate_edge_routes(&routing_graph, &mut channel_connectors, &edges, &abstract_routes, &rects);

        let route_segments = map_routes_to_segments(&routing_graph, &rects, &abstract_routes, &graph_edge_routes);
        assert_eq!(route_segments.len(), edges.len());

        let small_red_font = TextStyle::from(("sans-serif", 5).into_font()).color(&RED);
        for (r_id, segments) in route_segments.iter().enumerate() {
            let points = segments.iter().map(|p| (p.x as i32, p.y as i32)).collect::<Vec<_>>();
            let arr: [(i32, i32); 2] = points.iter().take(2).cloned().collect::<Vec<_>>()
                    .try_into()
                    .expect("vector must have exactly 2 elements");
            let label_pos = if arr[0].0 > arr[1].0 {
                (arr[0].0 + 1, arr[0].1 - 2)
            } else {
                (arr[0].0 - 4, arr[0].1 - 2)
            };
            root.draw_text(
                &format!("{}", r_id),
           &small_red_font,
                label_pos,
            )?;
            root.draw(&PathElement::new(
                    points,
                    ShapeStyle::from(&RED).stroke_width(1),           
                )
            )?;
        }

        for abstract_route in abstract_routes.iter() {
            println!("Abstract Route: from {} to {} bends {:?}", 
                abstract_route.from, abstract_route.to, abstract_route.bend_directions);
            for node_idx in abstract_route.route.iter() {
                println!("  Node idx: {} type {:?}", node_idx, routing_graph.nodes[*node_idx].node_type);
            }
        }

        for (channel_id,&slot) in graph_edge_routes.channel_slots.iter().enumerate() {
            if slot>0 {
                println!("Channel {} slots {}", channel_id, slot);
            }
        }

        for g_route in  graph_edge_routes.edge_routes.iter() {
            let aroute = &abstract_routes[g_route.abstract_route];
            println!("Edge Route: abstract_route {}:{}-{}, port_slots {:?}, channel_slots {:?}", g_route.abstract_route, aroute.from, aroute.to, g_route.port_slots, g_route.channel_slots);
        }


        Ok(())
    }

    #[test]
    fn test_bend_edges3() -> Result<(), Box<dyn std::error::Error>>  {
        let rects = vec![
            // central source element

            ERect::from_min_max(egui::pos2(70.0, 80.0), egui::pos2(80.0, 140.0)),
            ERect::from_min_max(egui::pos2(140.0, 80.0), egui::pos2(150.0, 140.0)),

            ERect::from_min_max(egui::pos2(40.0, 100.0), egui::pos2(60.0, 120.0)),
            ERect::from_min_max(egui::pos2(40.0, 80.0), egui::pos2(60.0, 90.0)),
            ERect::from_min_max(egui::pos2(160.0, 125.0), egui::pos2(180.0, 135.0)),

            ERect::from_min_max(egui::pos2(100.0, 100.0), egui::pos2(120.0, 120.0)),
            ERect::from_min_max(egui::pos2(100.0, 80.0), egui::pos2(120.0, 90.0)),
            ERect::from_min_max(egui::pos2(100.0, 130.0), egui::pos2(120.0, 140.0)),

            ERect::from_min_max(egui::pos2(160.0, 100.0), egui::pos2(180.0, 120.0)),
            ERect::from_min_max(egui::pos2(160.0, 80.0), egui::pos2(180.0, 90.0)),
            ERect::from_min_max(egui::pos2(40.0, 125.0), egui::pos2(60.0, 135.0)),

            ERect::from_min_max(egui::pos2(160.0, 138.0), egui::pos2(180.0, 150.0)),
            ERect::from_min_max(egui::pos2(40.0, 138.0), egui::pos2(60.0, 150.0)),

        ];

        let edges = vec![
            (2,5),
            (2,5),
            (3,5),
            (3,6),
            (4,5),
            (4,5),
            (5,8),
            (5,8),
            (5,9),
            (5,10),
            (5,10),
            (5,11),
            (5,12),
            (6,9),
            (7,11),
            (7,12),
        ].iter().map(|(from,to)| Edge{from:*from, to:*to, predicate: 0, bezier_distance: 0.0}).collect::<Vec<_>>();

        draw_graph(&rects, &edges, "routes_bends3.svg")?;

        Ok(())
    }

    #[test]
    fn test_leg_ordering_state() {
        //DRight +---------------------------+0  DownLeft
        //       |  +-------------------+1   |  
        //       |  |                   |    |
        //       |  |                   |    |
        //       0  1                   0    1
        //   0 - is global order
        //   We follow the route DownRight and DownLeft
        let mut order_state = LegOrderState::new(Pos2::new(0.0,0.0), Pos2::new(20.0,0.0));
        assert!(order_state.is_global_order(BendDirection::DownRight));
        assert!(order_state.is_global_order(BendDirection::DownLeft));
        assert!(!order_state.current_is_global);

        let mut order_state = LegOrderState::new(Pos2::new(20.0,0.0), Pos2::new(0.0,0.0));
        assert!(!order_state.is_global_order(BendDirection::DownLeft));
        assert!(order_state.is_global_order(BendDirection::DownRight));
        assert!(order_state.current_is_global);
    }

    #[test]
    fn test_channel_leg_cmp() {
        fn test_leg(start_connector: usize, end_connector: usize, circular_distance: u16, port_sides: PortSides) -> ChannelLeg {
            ChannelLeg {
                start_connector,
                end_connector,
                route_start_idx: 0,
                route_channel_idx: 0,
                route_end_idx: 1,
                edge_idx: 0,
                circular_distance,
                port_sides,
                route_order: 0,
                is_global_order: true,
            }
        }
        let leg1 = test_leg(0,2,2,PortSides::BothLeftOrTop);
        let leg2 = test_leg(1,3,3,PortSides::BothLeftOrTop);
        let leg3 = test_leg(1,3,1,PortSides::BothRightOrBottom);
        let leg4 = test_leg(1,3,1,PortSides::BothRightOrBottom);
        assert!(leg1 < leg2);
        assert!(leg2 < leg3);
        assert!(leg1 < leg3);
        assert!(leg3 == leg4);
        let leg5 = test_leg(3,1,1,PortSides::ChangeUp);
        let leg6 = test_leg(3,2,1,PortSides::ChangeUp);
        // leg5 is more left as leg6 because the connector orders 1<2
        assert!(leg5 < leg6);
        println!("Leg5 vs Leg6: {:?}", leg5.leg_relative_order(&leg6));
        assert_eq!(leg5.leg_relative_order(&leg6),std::cmp::Ordering::Less);

        let leg7 = test_leg(4,2,1,PortSides::ChangeUp);
        assert!(leg6 < leg7);
        let leg8 = test_leg(2,8,1,PortSides::ChangeUp);
        let leg9 = test_leg(2,9,1,PortSides::ChangeUp);
        assert!(leg9 > leg8);

        let leg10 = test_leg(3,1,1,PortSides::ChangeDown);
        let leg11 = test_leg(3,2,1,PortSides::ChangeDown);
        assert!(leg10 > leg11);

        let leg12 = test_leg(9,14,1,PortSides::ChangeDown);
        let leg13 = test_leg(9,15,1,PortSides::ChangeDown);
        let leg14 = test_leg(10,15,1,PortSides::ChangeDown);
        assert!(leg13 < leg12);
        assert!(leg14 < leg13);
        assert!(leg14 < leg12);

        let leg15 = test_leg(1,3,2,PortSides::BothRightOrBottom);
        let leg16 = test_leg(1,4,3,PortSides::BothRightOrBottom);
        assert!(leg15<leg16);
        // Exception local order is not global order
        assert_eq!(leg15.leg_relative_order(&leg16),std::cmp::Ordering::Greater);

    }

    #[test]
    fn test_port_sides() {
        let ps1 = PortSides::from_sides(Side::Left, Side::Left, 0, 1);
        assert_eq!(ps1, PortSides::BothLeftOrTop);
        let ps2 = PortSides::from_sides(Side::Right, Side::Right, 0, 1);
        assert_eq!(ps2, PortSides::BothRightOrBottom);
        let ps3 = PortSides::from_sides(Side::Left, Side::Right, 1, 0);
        assert_eq!(ps3, PortSides::ChangeUp);
        let ps4 = PortSides::from_sides(Side::Right, Side::Left, 1, 0);
        assert_eq!(ps4, PortSides::ChangeDown);
        let ps5 = PortSides::from_sides(Side::Left, Side::Right, 0, 1);
        assert_eq!(ps5, PortSides::ChangeDown);
        let ps6 = PortSides::from_sides(Side::Right, Side::Left, 0, 1);
        assert_eq!(ps6, PortSides::ChangeUp);
    }

}