use egui::{Rect as ERect, Vec2};

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
    pub edges: Vec<REdge>,
}

pub enum RNodeType {
    BendPoint,
    PortNord(usize),
    PortSouth(usize),
    PortEast(usize),
    PortWest(usize),
}
pub struct REdge {
    pub from: usize,
    pub to: usize,
}

pub struct RNode {
    pub node_type: RNodeType,
}

impl Default for RoutingGraph {
    fn default() -> Self {
        RoutingGraph {
            nodes: Vec::new(),
            edges: Vec::new(),
        }
    }
}

enum Side {
    Right,
    Left,
    Top,
    Bottom,
}

/* One need for different AreaLimit to bild a rect */
struct AreaLimit {
    coord: f32, 
    min: f32, 
    max: f32, 
    node_id: usize,
}

fn build_channels(boxes: &[ERect]) -> (Vec<ERect>,Vec<ERect>) {
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
    
    let mut vchannels: Vec<ERect> = Vec::new();

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
                                let channel = ERect::from_min_max(
                                    egui::pos2(left.coord, top.coord),
                                    egui::pos2(right.coord, bottom.coord),
                                );
                                let mut merged = false;
                                for mc in vchannels.iter_mut() {
                                    if mc.intersects(channel) {
                                        mc.set_top(mc.top().min(channel.top()));
                                        mc.set_bottom(mc.bottom().max(channel.bottom()));
                                        mc.set_left(mc.left().max(channel.left()));
                                        mc.set_right(mc.right().min(channel.right()));
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
    let mut hchannels: Vec<ERect> = Vec::new();

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
                                let channel = ERect::from_min_max(
                                    egui::pos2(left.coord, top.coord),
                                    egui::pos2(right.coord, bottom.coord),
                                );
                                let mut merged = false;
                                for mc in hchannels.iter_mut() {
                                    if mc.intersects(channel) {
                                        mc.set_left(mc.left().min(channel.left()));
                                        mc.set_right(mc.right().max(channel.right()));
                                        mc.set_top(mc.top().max(channel.top()));
                                        mc.set_bottom(mc.bottom().min(channel.bottom()));
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
    RoutingGraph::default()
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
        assert_eq!(routing_graph.nodes.len(), 0);
        assert_eq!(routing_graph.edges.len(), 0);
    }

    fn draw_rects(root: &DrawingArea<SVGBackend,Shift>,rects: &[ERect],color: &RGBColor) -> Result<(), Box<dyn std::error::Error>>{
        for rect in rects {
            let top_left = (rect.min.x as i32, rect.min.y as i32);
            let bottom_right = (rect.max.x as i32, rect.max.y as i32);
            root.draw(&Rectangle::new(
                [top_left, bottom_right],
                ShapeStyle::from(color).stroke_width(1),
            ))?;
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

        draw_rects(&root, &vchannels, &RED)?;
        draw_rects(&root, &hchannels, &BLUE)?;
        draw_rects(&root, &rects, &BLACK)?;

        assert_eq!(vchannels.len(), 3);
        assert_eq!(hchannels.len(), 4);

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