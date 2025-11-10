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
    pub nodes: Vec<RNode>,
    pub nodes_len: usize,
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

#[derive(Debug)]
pub enum Orientation {
    Vertical,
    Horizontal,
}

impl RoutingGraph {
    pub fn add_edge(&mut self, from: usize, to: usize) {
        self.nodes[from].neighbors.push(to);
        self.nodes[to].neighbors.push(from);
    }

    pub fn port_start_node(&self, node_idx: usize) -> usize {
        node_idx * 4 + self.nodes_len
    }
}

/* One need for different AreaLimit to bild a rect */
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
    };
    for edge in redges {
        routing_grpah.add_edge(edge.from, edge.to);
    }
    routing_grpah
}

pub fn route_edges(routing_graph: &RoutingGraph, edges: &[(usize,usize)]) -> Vec<Vec<usize>> {
    let mut edges_routes = Vec::new();
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
    let mut targets: Vec<usize> = Vec::new();
    let mut last_from = usize::MAX;
    for (from, to) in edges.iter() {
        if last_from != *from {
            if !targets.is_empty() {
                route_edges_from(routing_graph, last_from, &targets, &mut edges_routes);
            }
            last_from = *from;
            targets.clear();
        }
        targets.push(*to);
    }
    route_edges_from(routing_graph, last_from, &targets, &mut edges_routes);
    edges_routes
}

fn route_edges_from(routing_graph: &RoutingGraph, from: usize, to: &[usize], edges_routes: &mut Vec<Vec<usize>>) {
    // make bfs from from node to all to nodes.
    let mut visited: Vec<bool> = vec![false; routing_graph.nodes.len()];
    let mut predecessor: Vec<usize> = vec![usize::MAX; routing_graph.nodes.len()];
    let port_start_idx = routing_graph.port_start_node(from);
    let mut queue: VecDeque<usize> = VecDeque::new();
    for i in 0..4 {
        queue.push_back(port_start_idx+i);
        predecessor[port_start_idx+i] = from;
    }
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
                    edges_routes.push(route);
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

pub fn map_routes(routing_graph: &RoutingGraph, boxes: &[ERect], routes: Vec<Vec<usize>>) -> Vec<Vec<Pos2>> {
    let edge_segments: Vec<Vec<Pos2>> = Vec::new();
    edge_segments
}

#[cfg(test)]
mod tests {
    use std::{fs, path::PathBuf};

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

        let (vchannels, hchannels) = build_channels(&boxes);
        for (idx, c) in vchannels.iter().enumerate() {
            println!("V Channel: {} {:?}", idx, c);
        }
        for (idx, c) in hchannels.iter().enumerate() {
            println!("H Channel: {} {:?}", idx, c);
        }

        let routing_graph = create_routing_graph(&boxes);
        assert!(!routing_graph.nodes.is_empty());

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
        for (idx, route) in routes.iter().enumerate() {
            println!("route for edge {}", idx);
            for node_idx in route {
                let node = &routing_graph.nodes[*node_idx];
                println!(" idx: {} {:?}",*node_idx, node);
            }
        }
        assert_eq!(routes.len(), edges.len());
        let route_segments = map_routes(&routing_graph, &boxes, routes);
        assert_eq!(route_segments.len(), edges.len());

        let out_dir = PathBuf::from("target/test-output");
        fs::create_dir_all(&out_dir)?;
        let rects = test_rects();
        let svg_path = out_dir.join("graph_routes.svg");
        let root = SVGBackend::new(&svg_path, (200, 200)).into_drawing_area();
        draw_rects(&root, &rects, &BLACK)?;

        for segments in route_segments {
            root.draw(&Polygon::new(
                    segments.iter().map(|p| (p.x as i32, p.y as i32)).collect::<Vec<_>>(),
                    &GREEN,
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