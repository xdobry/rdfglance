use egui::{Pos2, Rect as ERect};

use super::routing::{Orientation, NodePort, Side};

#[derive(Debug)]
pub struct RChannel {
    pub rect: ERect,
    pub orientation: Orientation,
    pub ports: Vec<ChannelPort>,
}

#[derive(Debug)]
pub struct ChannelPort {
    pub position: f32,
    pub port_type: ChannelPortType,
    pub rnode_id: usize,
}

#[derive(Debug)]
pub enum ChannelPortType {
    NodePort{
        node_id: usize, 
        side: Side
    },
    Bend{ 
        channel_id: usize,
    },
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

    pub fn middle_pos(&self) -> f32 {
        match self.orientation {
            Orientation::Vertical => self.rect.center().x,
            Orientation::Horizontal => self.rect.center().y,
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

    pub fn width(&self) -> f32 {
        match self.orientation {
            Orientation::Horizontal => self.rect.height(),
            Orientation::Vertical => self.rect.width(),
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

pub fn build_channels(boxes: &[ERect]) -> (Vec<RChannel>,Vec<RChannel>) {
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
    let margin: f32 = 20.0;
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
                                    let port_type = ChannelPortType::NodePort {
                                        node_id: right.node_id,
                                        side: Side::Left,
                                    };
                                    let position = boxes[right.node_id].center().y;
                                    channel.ports.push(ChannelPort { position, port_type, rnode_id: usize::MAX });
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
                                    let port_type = ChannelPortType::NodePort {
                                        node_id: left.node_id,
                                        side: Side::Right,
                                    };
                                    let position = boxes[left.node_id].center().y;
                                    channel.ports.push(ChannelPort{ position, port_type, rnode_id: usize::MAX });
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
                                    let port_type = ChannelPortType::NodePort {
                                        node_id: bottom.node_id,
                                        side: Side::Top,
                                    };
                                    let position = boxes[bottom.node_id].center().x;
                                    channel.ports.push(ChannelPort{ position, port_type, rnode_id: usize::MAX });
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
                                    let port_type = ChannelPortType::NodePort {
                                        node_id: top.node_id,
                                        side: Side::Bottom,
                                    };
                                    let position = boxes[top.node_id].center().x;
                                    channel.ports.push(ChannelPort{ position, port_type, rnode_id: usize::MAX });
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
}