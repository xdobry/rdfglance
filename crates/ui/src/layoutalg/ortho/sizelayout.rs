// Adapt position of nodes and channels to minimal channels with

use std::collections::{HashMap, VecDeque};

use egui::{Rect as ERect};

use crate::layoutalg::ortho::channels::ChannelPortType;

use super::routing::{RoutingGraph, NodePort, Side, RNodeType};
use super::channels::{RChannel};

#[derive(Debug)]
enum PosMove {
    MoveBox(f32, usize),
    AddChannelWidth(f32, usize),
    MoveChannel(f32, usize),
}

struct DirectionX;
struct DirectionY;

trait Direction {
    fn get_rect_min(rect: &ERect) -> f32;
    fn get_rect_max(rect: &ERect) -> f32;
    fn set_max(rect: &mut ERect, v: f32);
    fn add_max(rect: &mut ERect, add: f32) -> f32;
    fn move_rect(rect: &mut ERect, add: f32) -> f32;
    fn get_channels(routing_graph: &RoutingGraph) -> &[RChannel];
    fn get_channel_by_id<'a>(routing_graph: &'a mut RoutingGraph, channel_id: usize) -> &'a mut RChannel;
    fn get_orth_channel_by_id<'a>(routing_graph: &'a mut RoutingGraph, channel_id: usize) -> &'a mut RChannel;
    fn max_side_ports(channel: &RChannel) -> impl Iterator<Item=NodePort>;
    fn bend_points(routing_graph: &RoutingGraph) -> impl Iterator<Item=(usize, usize)>;
    fn port_opposite_side() -> Side;
}

impl Direction for DirectionX {
    fn get_rect_min(rect: &ERect) -> f32 {
        rect.min.x
    }
    fn get_rect_max(rect: &ERect) -> f32 {
        rect.max.x
    }
    fn add_max(rect: &mut ERect, add: f32) -> f32 {
        rect.max.x += add;
        rect.max.x
    }
    fn set_max(rect: &mut ERect, v: f32)  {
        rect.max.x = v;
    }
    fn move_rect(rect: &mut ERect, add: f32) -> f32 {
        rect.min.x += add;
        rect.max.x += add;
        rect.max.x
    }
    fn get_channels(routing_graph: &RoutingGraph) -> &[RChannel] {
        &routing_graph.vchannels
    }
    fn get_channel_by_id<'a>(routing_graph: &'a mut RoutingGraph, channel_id: usize) -> &'a mut RChannel {
        &mut routing_graph.vchannels[channel_id]
    }
    fn get_orth_channel_by_id<'a>(routing_graph: &'a mut RoutingGraph, channel_id: usize) -> &'a mut RChannel {
        &mut routing_graph.hchannels[channel_id]
    }
    fn max_side_ports(channel: &RChannel) -> impl Iterator<Item=NodePort> {
        channel.ports.iter().filter_map(|p| 
            match p.port_type {
                ChannelPortType::NodePort{node_id, side, ..} if side == Side::Left => {
                    Some(NodePort {
                        node_id,
                        side,
                    })
                },
                _ => None,
            }
        )
    }
    fn bend_points(routing_graph: &RoutingGraph) -> impl Iterator<Item=(usize, usize)> {
        routing_graph.nodes.iter()
            .skip(routing_graph.nodes_len*5+routing_graph.hchannels.len()+routing_graph.vchannels.len())
            .map(|n| match n.node_type {
                RNodeType::BendPoint( vchannel_id, hchannel_id, ) => {
                    (vchannel_id, hchannel_id)
                }
                _ => {
                    panic!("expect only bend nodes");
                }
            })

    }
    fn port_opposite_side() -> Side {
        Side::Right
    }
}

impl Direction for DirectionY {
    fn get_rect_min(rect: &ERect) -> f32 {
        rect.min.y
    }
    fn get_rect_max(rect: &ERect) -> f32 {
        rect.max.y
    }
    fn add_max(rect: &mut ERect, add: f32) -> f32 {
        rect.max.y += add;
        rect.max.y
    }
    fn set_max(rect: &mut ERect, v: f32)  {
        rect.max.y = v;
    }
    fn move_rect(rect: &mut ERect, add: f32) -> f32 {
        rect.min.y += add;
        rect.max.y += add;
        rect.max.y
    }
    fn get_channels(routing_graph: &RoutingGraph) -> &[RChannel] {
        &routing_graph.hchannels
    }
    fn get_channel_by_id<'a>(routing_graph: &'a mut RoutingGraph, channel_id: usize) -> &'a mut RChannel {
        &mut routing_graph.hchannels[channel_id]
    }
    fn get_orth_channel_by_id<'a>(routing_graph: &'a mut RoutingGraph, channel_id: usize) -> &'a mut RChannel {
        &mut routing_graph.vchannels[channel_id]
    }
    fn max_side_ports<'a>(channel: &'a RChannel) -> impl Iterator<Item=NodePort> {
        channel.ports.iter().filter_map(|p| 
            match p.port_type {
                ChannelPortType::NodePort{node_id, side, ..} if side == Side::Top => {
                    Some(NodePort {
                        node_id,
                        side,
                    })
                },
                _ => None,
            }
        )
    }
    fn bend_points(routing_graph: &RoutingGraph) -> impl Iterator<Item=(usize, usize)> {
        routing_graph.nodes.iter()
            .skip(routing_graph.nodes_len*5+routing_graph.hchannels.len()+routing_graph.vchannels.len())
            .map(|n| match n.node_type {
                RNodeType::BendPoint( vchannel_id, hchannel_id, ) => {
                    (hchannel_id, vchannel_id)
                }
                _ => {
                    panic!("expect only bend nodes");
                }
            })

    }
    fn port_opposite_side() -> Side {
        Side::Bottom
    }
}


pub fn resize_channels(routing_graph: &mut RoutingGraph, nodes: &mut [ERect], min_with_vertical: &[f32], min_with_horizontal: &[f32]) {
    assert_eq!(routing_graph.vchannels.len(),min_with_vertical.len());
    assert_eq!(routing_graph.hchannels.len(),min_with_horizontal.len());
    resize_direction::<DirectionX>(routing_graph, nodes, &min_with_vertical);    
    resize_direction::<DirectionY>(routing_graph, nodes, &min_with_horizontal);    
}

fn resize_direction<D: Direction>(routing_graph: &mut RoutingGraph, nodes: &mut [ERect], min_with: &[f32]) {
    let mut pos_moves: VecDeque<PosMove> = VecDeque::new();
    for (channel_idx,min_with) in min_with.iter().enumerate() {
        let channel = D::get_channel_by_id(routing_graph, channel_idx);
        let delta = min_with - channel.width();
        if delta>0.0 {
            let pos_move = PosMove::AddChannelWidth(delta, channel_idx);
            pos_moves.push_back(pos_move);
        }
    }
    // All right channels for all nodes (right ports)
    let mut right_channels: Vec<usize> = vec![usize::MAX;routing_graph.nodes_len];
    for (channel_id, channel) in D::get_channels(&routing_graph).iter().enumerate() {
        for channel_port in channel.ports.iter() {
            match channel_port.port_type {
                ChannelPortType::NodePort{node_id, side} => {
                    if side == D::port_opposite_side() {
                        right_channels[node_id] = channel_id;
                    }
                }
                _ => {}
            }
        }
    }
    // All channels that lenght must be adapted to max value
    let mut orth_margin_channels : HashMap<usize,Vec<usize>> = HashMap::new();
    let bend_points: Vec<(usize,usize)> = D::bend_points(&routing_graph).collect();
    for (channel_id, orth_channel_id) in bend_points.iter() {
        let rect = D::get_channel_by_id(routing_graph, *channel_id).rect;
        let orth_rect = D::get_orth_channel_by_id(routing_graph, *orth_channel_id).rect;
        if D::get_rect_max(&rect) == D::get_rect_max(&orth_rect) {
            orth_margin_channels.entry(*channel_id).or_default().push(*orth_channel_id);
        }       
    }
    while let Some(pos_move) = pos_moves.pop_front() {
        // println!("apply pos_move {:?}", pos_move);
        match pos_move {
            PosMove::MoveBox(new_min, node_idx) => {
                let mut rect = &mut nodes[node_idx];
                let delta = new_min - D::get_rect_min(&rect);
                if delta>0.0 {
                    let new_max_pos = D::move_rect(&mut rect, delta);
                    let right_channel_id = right_channels[node_idx];
                    if right_channel_id!=usize::MAX {
                        let rect = &D::get_channel_by_id(routing_graph, right_channel_id).rect;
                        let delta = new_max_pos - D::get_rect_min(&rect);
                        if delta > 0.0 {
                            pos_moves.push_front(PosMove::MoveChannel(new_max_pos, right_channel_id));
                        }
                    }
                }
            },
            PosMove::MoveChannel(new_min, channel_idx) => {
                let channel = D::get_channel_by_id(routing_graph, channel_idx);
                let delta = new_min - D::get_rect_min(&channel.rect);
                if delta>0.0 {
                    let new_max_pos = D::move_rect(&mut channel.rect, delta);
                    for port in D::max_side_ports(&channel) {
                        let rect = &nodes[port.node_id];
                        let delta = new_max_pos - D::get_rect_min(&rect);
                        if delta>0.0 {
                            pos_moves.push_front(PosMove::MoveBox(new_max_pos, port.node_id));
                        }
                    }
                }
            },
            PosMove::AddChannelWidth(delta, channel_idx) => {
                let channel = D::get_channel_by_id(routing_graph, channel_idx);
                // println!("resize channel {} by {}",channel_idx, delta);
                let new_max_pos = D::add_max(&mut channel.rect, delta);
                for port in D::max_side_ports(&channel) {
                    let rect = &nodes[port.node_id];
                    let delta = new_max_pos - D::get_rect_min(&rect);
                    // println!("max port {:?} min-port {} new_max_port {}",port,D::get_rect_min(&rect),new_max_pos);
                    if delta>0.0 {
                        pos_moves.push_front(PosMove::MoveBox(new_max_pos, port.node_id));
                    }
                }
            },
        }
    }
    for (channel_id, margin_channels) in orth_margin_channels.iter() {
        let rect = D::get_channel_by_id(routing_graph, *channel_id).rect;
        let max_value = D::get_rect_max(&rect);
        for orth_channel_id in margin_channels {
            let orth_channel = D::get_orth_channel_by_id(routing_graph, *orth_channel_id);
            D::set_max(&mut orth_channel.rect, max_value);
        }
    }
}


#[cfg(test)]
mod tests {
    use std::{fs, path::PathBuf, vec};

    use super::super::routing::create_routing_graph;
    use super::super::channels::RChannel;

    use super::*;
    use plotters::{coord::Shift, prelude::*};

    fn test_rects() -> Vec<ERect> {
        vec![
            ERect::from_min_max(egui::pos2(10.0, 10.0), egui::pos2(40.0, 20.0)),
            ERect::from_min_max(egui::pos2(50.0, 12.0), egui::pos2(70.0, 20.0)),
            ERect::from_min_max(egui::pos2(10.0, 30.0), egui::pos2(40.0, 37.0)),
            ERect::from_min_max(egui::pos2(45.0, 30.0), egui::pos2(67.0, 40.0)),
            ERect::from_min_max(egui::pos2(30.0, 50.0), egui::pos2(65.0, 60.0)),
        ]
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
    fn test_resize_channels() -> Result<(), Box<dyn std::error::Error>> {
        let mut rects = test_rects();
        let mut routing_graph = create_routing_graph(&rects);

        let out_dir = PathBuf::from("target/test-output");
        fs::create_dir_all(&out_dir)?;
        let svg_path = out_dir.join("channel_resize.svg");
        let backend = SVGBackend::new(&svg_path, (200, 400));
        let root = backend.into_drawing_area();
        root.fill(&WHITE)?;

        draw_channels(&root, &routing_graph.vchannels, &RED)?;
        draw_channels(&root, &routing_graph.hchannels, &BLUE)?;
        draw_rects(&root, &rects, &BLACK)?;

        assert_eq!(routing_graph.vchannels.len(), 3);
        assert_eq!(routing_graph.hchannels.len(), 4);

        let shifted = root.apply_coord_spec(Shift((0,100)));


        let min_sizes_horizontal : Vec<f32> = vec![20.0 ; routing_graph.hchannels.len()];
        let min_sizes_vertical : Vec<f32> = vec![20.0 ; routing_graph.vchannels.len()];
        resize_channels(&mut routing_graph, &mut rects, &min_sizes_vertical, &min_sizes_horizontal);       

        draw_channels(&shifted, &routing_graph.vchannels, &RED)?;
        draw_channels(&shifted, &routing_graph.hchannels, &BLUE)?;
        draw_rects(&shifted, &rects, &BLACK)?;       

        Ok(())
    }
}