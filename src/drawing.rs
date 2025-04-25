use eframe::egui::{Color32, Painter, Pos2, Stroke};
use egui::{epaint::EllipseShape, text::LayoutJob, FontId, Rect, StrokeKind, Vec2};

use crate::{EdgeStyle, LabelPosition, NodeShape, NodeSize, TypeStyle};

const POS_SPACE: f32 = 3.0;
const NODE_RADIUS: f32 = 10.0;

pub fn draw_edge(painter: &Painter, point_from: Pos2, size_from: Vec2, shape_from: NodeShape, 
    point_to: Pos2, size_to: Vec2, shape_to: NodeShape,
    edge_style: &EdgeStyle) {

    let dir= point_to - point_from;
    
    // Compute the length (Euclidean distance)
    let length = dir.length();
    let radius_to = size_to.x/2.0;
    let radius_from = size_from.x/2.0;

    if length <= radius_to + radius_from {
        // nodes overlapping, no edge needed
        return;
    }
    
    // Normalize and scale to radius
    let unit = dir / length;
    
    // Find intersection on shape surface
    let edge_to = match shape_to {
        NodeShape::Rect => {
            let rect = Rect::from_center_size((-dir).to_pos2(), size_to);
            let interect_pos = rect.intersects_ray_from_center(unit);
            (point_from-interect_pos).to_pos2()
        },
        _ => {
            point_to - unit * radius_to
        },
    };

    let edge_from = match shape_from {
        NodeShape::Rect => {
            let rect = Rect::from_center_size(dir.to_pos2(), size_from);
            let interect_pos = rect.intersects_ray_from_center(-unit);
            (point_to-interect_pos).to_pos2()
        },
        _ => {
            point_from + unit * radius_from
        },
    };
    
    // Draw arrow (line + head)
    let stroke = Stroke::new(edge_style.width, edge_style.color);
    painter.line_segment([edge_from, edge_to], stroke);
    
    // Draw arrowhead
    let arrow_size = 8.0;  // Size of the arrowhead
    let arrow_angle = std::f32::consts::PI / 6.0; // 30 degrees
    
    // Rotate vector by ±arrow_angle to get arrowhead points
    let cos_theta = arrow_angle.cos();
    let sin_theta = arrow_angle.sin();
    
    let left = edge_to - arrow_size * Vec2::new(cos_theta * unit.x - sin_theta * unit.y,sin_theta * unit.x + cos_theta * unit.y);
    let right = edge_to - arrow_size * Vec2::new(cos_theta * unit.x + sin_theta * unit.y, -sin_theta * unit.x + cos_theta * unit.y);
    
    // Draw arrowhead lines
    painter.line_segment([edge_to, left], stroke);
    painter.line_segment([edge_to, right], stroke);
}

pub fn draw_node_label(
    painter: &Painter,
    node_label: &str,
    type_style: &TypeStyle,
    pos: Pos2,
    selected: bool,
    highlighted: bool,
    show_labels: bool,
) -> (Rect, NodeShape) {
    let mut job = LayoutJob::default();
    let font = FontId::proportional(type_style.font_size);
    job.append(
        node_label,
        0.0,
        egui::TextFormat {
            font_id: font,
            color: if highlighted { Color32::BLUE } else { type_style.label_color},
            ..Default::default()
        },
    );
    if type_style.label_max_width>0.0 {
        job.wrap = egui::text::TextWrapping {
            max_width: type_style.label_max_width,
            max_rows: type_style.max_lines as usize,
            overflow_character: None,
            ..Default::default()
        };
    }                     
    let galley = painter.layout_job(job);
    let text_pos = match type_style.label_position {
        LabelPosition::Center => {
            pos-Vec2::new(galley.rect.width()/2.0,galley.rect.height()/2.0)
        },
        LabelPosition::Above => {
            pos+Vec2::new(-galley.rect.width()/2.0,-(galley.rect.height()+POS_SPACE+type_style.height/2.0))
        },
        LabelPosition::Below => {
            pos+Vec2::new(-galley.rect.width()/2.0,POS_SPACE+type_style.height/2.0)
        },
        LabelPosition::Left => {
            pos+Vec2::new(-(galley.rect.width()+type_style.width/2.0+POS_SPACE), -galley.rect.height()/2.0)
        },
        LabelPosition::Right => {
            pos+Vec2::new(type_style.width/2.0+POS_SPACE, -galley.rect.height()/2.0)
        },
    };
    let node_rect = if show_labels {
        let stroke = if type_style.border_width>0.0 {
            Stroke::new(type_style.border_width, type_style.border_color)
        } else {
            Stroke::NONE
        };
        let node_rect = match type_style.node_size {
            NodeSize::Fixed => {
                Rect::from_center_size(pos, Vec2::new(type_style.width, type_style.height))
            },
            NodeSize::Label => {
                Rect::from_center_size(pos, Vec2::new(galley.rect.width()+type_style.width, galley.rect.height()+type_style.height))
            }
        };
        if selected {
            let select_rec = if type_style.node_shape==NodeShape::Circle {
                Rect::from_center_size(pos, Vec2::splat(node_rect.width()+4.0))
            } else {
                node_rect.expand(4.0)
            };
            painter.rect_filled(select_rec, 3.0, egui::Color32::from_rgba_premultiplied(200, 200, 0, 150));
        }   
        match type_style.node_shape {
            NodeShape::Circle => {
                painter.circle(pos, node_rect.width()/2.0, type_style.color, stroke);
            },
            NodeShape::Elipse => {
                painter.add(egui::Shape::Ellipse(EllipseShape {
                    center: pos,
                    radius: Vec2::new(node_rect.width()/2.0, node_rect.height()/2.0),
                    fill: type_style.color,
                    stroke: stroke,
                }));
            }
            NodeShape::Rect => { 
                painter.rect(node_rect,type_style.corner_radius,type_style.color, stroke,StrokeKind::Outside);
            }
            NodeShape::None => {
                // No shape, just text
            }
        }
        node_rect
    } else {
        if selected {
            painter.circle_filled(pos, NODE_RADIUS + 3.0, egui::Color32::from_rgba_premultiplied(255, 255, 0, 170));
        }   
        painter.circle_filled(pos, NODE_RADIUS, type_style.color);
        Rect::from_center_size(pos, Vec2::new(NODE_RADIUS*2.0, NODE_RADIUS*2.0))
    };
    if highlighted {
        let hrec = galley.rect.translate(Vec2::new(text_pos.x,text_pos.y));
        painter.rect_filled(hrec, 3.0, Color32::from_rgba_unmultiplied(255, 255, 153, 200));
    }
    if show_labels || highlighted {
        painter.galley(text_pos, galley, Color32::BLACK);
    }
    if type_style.node_shape == NodeShape::Circle {
        (Rect::from_center_size(node_rect.center(), Vec2::splat(node_rect.width())), NodeShape::Circle)
    } else {
        (node_rect, type_style.node_shape)
    }
    
}