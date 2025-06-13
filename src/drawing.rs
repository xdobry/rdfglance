use eframe::egui::{Color32, Painter, Pos2, Stroke};
use egui::{
    epaint::{CubicBezierShape, EllipseShape, QuadraticBezierShape, TextShape}, text::LayoutJob, Align2, FontId, Rect, Shape, StrokeKind, Vec2
};

use crate::{
    EdgeStyle, NodeStyle,
    graph_styles::{ArrowLocation, ArrowStyle, IconPosition, LabelPosition, LineStyle, NodeShape, NodeSize},
};

const POS_SPACE: f32 = 3.0;
const NODE_RADIUS: f32 = 10.0;

pub fn draw_edge<F>(
    painter: &Painter,
    point_from: Pos2,
    size_from: Vec2,
    shape_from: NodeShape,
    point_to: Pos2,
    size_to: Vec2,
    shape_to: NodeShape,
    edge_style: &EdgeStyle,
    label_cb: F,
    faded: bool,
    bezier_distance: f32,
) where
    F: Fn() -> String,
{
    let dir = point_to - point_from;

    // Compute the length (Euclidean distance)
    let length = dir.length();
    let radius_to = size_to.x / 2.0;
    let radius_from = size_from.x / 2.0;

    if !matches!(shape_to, NodeShape::Rect) && !matches!(shape_from, NodeShape::Rect) && length <= radius_to + radius_from
    {
        // nodes are non rect (so handle as circles) overlapping, no edge needed
        return;
    }
    if matches!(shape_to, NodeShape::Rect) && matches!(shape_from, NodeShape::Rect) {
        // both nodes are rect, test if react overlapping
        let rect_from = Rect::from_center_size(point_from, size_from);
        let rect_to = Rect::from_center_size(point_to, size_to);
        if rect_from.intersects(rect_to) {
            // nodes are overlapping, no edge needed
            return;
        }
    }

    // Normalize and scale to radius
    let unit = dir / length;
    let mut arrow_unit = unit;

    // Find intersection on shape surface
    let edge_to = match shape_to {
        NodeShape::Rect => {
            let rect = Rect::from_center_size((-dir).to_pos2(), size_to);
            let interect_pos = rect.intersects_ray_from_center(unit);
            let edge_to = (point_from - interect_pos).to_pos2();
            if !matches!(shape_from, NodeShape::Rect) {
                let v_to_center = edge_to - point_from;
                if v_to_center.length() < radius_from {
                    // the intersection point is inside the circle, so we need to move it to the edge of the circle
                    return;
                }
            }
            edge_to
        }
        _ => point_to - unit * radius_to,
    };

    let edge_from = match shape_from {
        NodeShape::Rect => {
            let rect = Rect::from_center_size(dir.to_pos2(), size_from);
            let interect_pos = rect.intersects_ray_from_center(-unit);
            let edge_from = (point_to - interect_pos).to_pos2();
            if !matches!(shape_to, NodeShape::Rect) {
                let v_from_center = edge_from - point_to;
                if v_from_center.length() < radius_to {
                    // the intersection point is inside the circle, so we need to move it to the edge of the circle
                    return;
                }
            }
            edge_from
        }
        _ => point_from + unit * radius_from,
    };

    // Draw arrow (line + head)
    let stroke = Stroke::new(edge_style.width, fade_color(edge_style.color, faded));
    match edge_style.line_style {
        LineStyle::Solid =>{
            if bezier_distance != 0.0 {
                let middle = (edge_from + edge_to.to_vec2()) / 2.0;
                let ctrl_pos = middle + unit.rot90() * bezier_distance;
                painter.add(Shape::QuadraticBezier(
                    QuadraticBezierShape::from_points_stroke(
                        [edge_from, ctrl_pos, edge_to],
                        false,
                        Color32::TRANSPARENT,
                        stroke,
                    ),
                ));
                arrow_unit = (edge_to - ctrl_pos).normalized();
            } else {
                painter.line_segment([edge_from, edge_to], stroke);
            }
        }
        LineStyle::Dashed => {
            painter.add(Shape::dashed_line(
                &[edge_from, edge_to],
                stroke,
                edge_style.line_gap,
                edge_style.width * 5.0,
            ));
        }
        LineStyle::Dotted => {
            painter.add(Shape::dotted_line(
                &[edge_from, edge_to],
                fade_color(edge_style.color, faded),
                edge_style.line_gap,
                edge_style.width,
            ));
        }
    }

    if !matches!(edge_style.arrow_location, ArrowLocation::None) {
        // Draw arrowhead
        let arrow_size = edge_style.arrow_size; // Size of the arrowhead
        let arrow_angle = std::f32::consts::PI / 6.0; // 30 degrees

        let arrow_pos = match edge_style.arrow_location {
            ArrowLocation::Middle => (edge_from + edge_to.to_vec2()) / 2.0,
            _ => edge_to,
        };

        // Rotate vector by ±arrow_angle to get arrowhead points
        let cos_theta = arrow_angle.cos();
        let sin_theta = arrow_angle.sin();

        let left = arrow_pos
            - arrow_size
                * Vec2::new(
                    cos_theta * arrow_unit.x - sin_theta * arrow_unit.y,
                    sin_theta * arrow_unit.x + cos_theta * arrow_unit.y,
                );
        let right = arrow_pos
            - arrow_size
                * Vec2::new(
                    cos_theta * arrow_unit.x + sin_theta * arrow_unit.y,
                    -sin_theta * arrow_unit.x + cos_theta * arrow_unit.y,
                );

        // Draw arrowhead lines
        match edge_style.target_style {
            ArrowStyle::Arrow => {
                painter.line_segment([arrow_pos, left], stroke);
                painter.line_segment([arrow_pos, right], stroke);
            }
            ArrowStyle::ArrorTriangle => {
                painter.line_segment([arrow_pos, left], stroke);
                painter.line_segment([arrow_pos, right], stroke);
                painter.line_segment([left, right], stroke);
            }
            ArrowStyle::ArrorFilled => {
                let shape = Shape::convex_polygon(vec![arrow_pos, left, right], fade_color(edge_style.color, faded), Stroke::NONE);
                painter.add(shape);
            }
        }
    }

    if let Some(edge_font) = &edge_style.edge_font {
        let line_midle = (edge_from + edge_to.to_vec2()) / 2.0;
        let label_font = FontId::proportional(edge_font.font_size);
        let unit_ort = -unit.rot90() * (edge_font.font_size + bezier_distance/2.0);
        let label_pos = line_midle + unit_ort;
        let label = label_cb();
        let mut job = LayoutJob::default();
        job.append(
            label.as_str(),
            0.0,
            egui::TextFormat {
                font_id: label_font,
                color: fade_color(edge_font.font_color, faded),
                ..Default::default()
            },
        );
        let galley = painter.layout_job(job);
        // change the direction if pointing to left side to ensure the text is left to right
        let unit_adapted = if unit.x < 0.0 {
            -unit
        } else {
            unit
        };
        let angle = unit_adapted.angle();
        let gallay_center = galley.rect.center();
        let gallay_center = Vec2::new(
            gallay_center.x * angle.cos() - gallay_center.y * angle.sin(),
            gallay_center.x * angle.sin() + gallay_center.y * angle.cos(),
        );
        painter.add(Shape::Text(
            TextShape::new(label_pos - gallay_center, galley, Color32::BLACK).with_angle(angle),
        ));
    }

    if let Some(icon_style) = &edge_style.icon_style {
        let line_midle = (edge_from + edge_to.to_vec2()) / 2.0;
        let icon_font = FontId::proportional(icon_style.icon_size);
        let unit_ort = unit.rot90() * icon_style.icon_size / 2.0;
        let icon_pos = line_midle + unit_ort;
        painter.text(
            icon_pos,
            Align2::CENTER_CENTER,
            icon_style.icon_character.to_string(),
            icon_font,
            fade_color(icon_style.icon_color, faded),
        );
    }
}

pub fn draw_self_edge<F>(
    painter: &Painter,
    point: Pos2,
    size: Vec2,
    rotation: f32,
    _shape: NodeShape,
    edge_style: &EdgeStyle,
    faded: bool,
    label_cb: F,
) where
F: Fn() -> String,
{
    let stroke = Stroke::new(edge_style.width, fade_color(edge_style.color, faded));
    let radius = size.x / 2.0;
    let angle_1 = std::f32::consts::FRAC_PI_4 + rotation;
    let angle_2 = angle_1 + std::f32::consts::FRAC_PI_2;
    let direction_1 = Vec2::new(angle_1.sin(), -angle_1.cos());
    let direction_2 = Vec2::new(angle_2.sin(), -angle_2.cos());
    let pos1 = point + (direction_1 * radius);
    let pos2 = point + (direction_2 * radius);
    let ctrl_pos_distance = 100.0;
    let ctrl_pos1 = point + direction_1 * (radius + ctrl_pos_distance);
    let ctrl_pos2 = point +direction_2 * (radius + ctrl_pos_distance);
    painter.add(Shape::CubicBezier(
        CubicBezierShape::from_points_stroke([pos1, ctrl_pos1, ctrl_pos2, pos2], false, Color32::TRANSPARENT, stroke),
    ));

    if !matches!(edge_style.arrow_location, ArrowLocation::None) {
        // Draw arrowhead
        let arrow_size = edge_style.arrow_size; // Size of the arrowhead
        let arrow_angle = std::f32::consts::PI / 6.0; // 30 degrees

        let arrow_pos = match edge_style.arrow_location {
            ArrowLocation::Middle => bezier_middle_point(pos1, ctrl_pos1, ctrl_pos2, pos2),
            _ => pos2,
        };

        let unit = (pos2 - ctrl_pos2).normalized();

        // Rotate vector by ±arrow_angle to get arrowhead points
        let cos_theta = arrow_angle.cos();
        let sin_theta = arrow_angle.sin();

        let left = arrow_pos
            - arrow_size
                * Vec2::new(
                    cos_theta * unit.x - sin_theta * unit.y,
                    sin_theta * unit.x + cos_theta * unit.y,
                );
        let right = arrow_pos
            - arrow_size
                * Vec2::new(
                    cos_theta * unit.x + sin_theta * unit.y,
                    -sin_theta * unit.x + cos_theta * unit.y,
                );

        // Draw arrowhead lines
        match edge_style.target_style {
            ArrowStyle::Arrow => {
                painter.line_segment([arrow_pos, left], stroke);
                painter.line_segment([arrow_pos, right], stroke);
            }
            ArrowStyle::ArrorTriangle => {
                painter.line_segment([arrow_pos, left], stroke);
                painter.line_segment([arrow_pos, right], stroke);
                painter.line_segment([left, right], stroke);
            }
            ArrowStyle::ArrorFilled => {
                let shape = Shape::convex_polygon(vec![arrow_pos, left, right], fade_color(edge_style.color, faded), Stroke::NONE);
                painter.add(shape);
            }
        }
    }

    if let Some(edge_font) = &edge_style.edge_font {
        let curve_midle = bezier_middle_point(pos1, ctrl_pos1, ctrl_pos2, pos2);
        let label_font = FontId::proportional(edge_font.font_size);
        let label = label_cb();
        let mut job = LayoutJob::default();
        job.append(
            label.as_str(),
            0.0,
            egui::TextFormat {
                font_id: label_font,
                color: fade_color(edge_font.font_color, faded),
                ..Default::default()
            },
        );
        let galley = painter.layout_job(job);
        // change the direction if pointing to left side to ensure the text is left to right
        painter.add(Shape::Text(
            TextShape::new(curve_midle, galley, Color32::BLACK),
        ));
    }
}

fn bezier_middle_point(pos1: Pos2, ctrl_pos1: Pos2, ctrl_pos2: Pos2, pos2: Pos2) -> Pos2 {
    let t = 0.5;
    let u = 1.0 - t;

    let p = pos1.to_vec2() * u * u * u
        + ctrl_pos1.to_vec2() * 3.0 * u * u * t
        + ctrl_pos2.to_vec2() * 3.0 * u * t * t
        + pos2.to_vec2() * t * t * t;

    p.to_pos2()
}

#[inline]
fn fade_color(color: Color32, fade: bool) -> Color32 {
    if fade {
        color.gamma_multiply_u8(60)
    } else {
        color
    }
}

pub fn draw_node_label(
    painter: &Painter,
    node_label: &str,
    type_style: &NodeStyle,
    pos: Pos2,
    selected: bool,
    highlighted: bool,
    faded: bool,
    show_labels: bool,
) -> (Rect, NodeShape) {
    let mut job = LayoutJob::default();
    let font = FontId::proportional(type_style.font_size);
    job.append(
        node_label,
        0.0,
        egui::TextFormat {
            font_id: font,
            color: if highlighted {
                Color32::BLUE
            } else {
                fade_color(type_style.label_color, faded)
            },
            ..Default::default()
        },
    );
    if type_style.label_max_width > 0.0 {
        job.wrap = egui::text::TextWrapping {
            max_width: type_style.label_max_width,
            max_rows: type_style.max_lines as usize,
            overflow_character: None,
            ..Default::default()
        };
    }
    let galley = painter.layout_job(job);
    let text_rect = galley.rect;
    let text_pos = match type_style.label_position {
        LabelPosition::Center => pos - Vec2::new(text_rect.width() / 2.0, text_rect.height() / 2.0),
        LabelPosition::Above => {
            pos + Vec2::new(
                -text_rect.width() / 2.0,
                -(text_rect.height() + POS_SPACE + type_style.height / 2.0),
            )
        }
        LabelPosition::Below => pos + Vec2::new(-text_rect.width() / 2.0, POS_SPACE + type_style.height / 2.0),
        LabelPosition::Left => {
            pos + Vec2::new(
                -(text_rect.width() + type_style.width / 2.0 + POS_SPACE),
                -text_rect.height() / 2.0,
            )
        }
        LabelPosition::Right => pos + Vec2::new(type_style.width / 2.0 + POS_SPACE, -text_rect.height() / 2.0),
    };
    let node_rect = if show_labels {
        let stroke = if type_style.border_width > 0.0 {
            Stroke::new(type_style.border_width, fade_color(type_style.border_color,faded))
        } else {
            Stroke::NONE
        };
        let node_rect = match type_style.node_size {
            NodeSize::Fixed => Rect::from_center_size(pos, Vec2::new(type_style.width, type_style.height)),
            NodeSize::Label => Rect::from_center_size(
                pos,
                Vec2::new(
                    galley.rect.width() + type_style.width,
                    galley.rect.height() + type_style.height,
                ),
            ),
        };
        if selected {
            let select_rec = if type_style.node_shape == NodeShape::Circle {
                Rect::from_center_size(pos, Vec2::splat(node_rect.width() + 4.0))
            } else {
                node_rect.expand(4.0)
            };
            painter.rect_filled(
                select_rec,
                3.0,
                egui::Color32::from_rgba_premultiplied(200, 200, 0, 150),
            );
        }
        match type_style.node_shape {
            NodeShape::Circle => {
                painter.circle(pos, node_rect.width() / 2.0, fade_color(type_style.color, faded), stroke);
            }
            NodeShape::Elipse => {
                painter.add(egui::Shape::Ellipse(EllipseShape {
                    center: pos,
                    radius: Vec2::new(node_rect.width() / 2.0, node_rect.height() / 2.0),
                    fill: fade_color(type_style.color, faded),
                    stroke,
                }));
            }
            NodeShape::Rect => {
                painter.rect(
                    node_rect,
                    type_style.corner_radius,
                    fade_color(type_style.color, faded),
                    stroke,
                    StrokeKind::Outside,
                );
            }
            NodeShape::None => {
                // No shape, just text
            }
        }
        node_rect
    } else {
        if selected {
            painter.circle_filled(
                pos,
                NODE_RADIUS + 3.0,
                egui::Color32::from_rgba_premultiplied(255, 255, 0, 170),
            );
        }
        painter.circle_filled(pos, NODE_RADIUS, fade_color(type_style.color, faded));
        if let Some(icon_style) = &type_style.icon_style {
            let icon_pos = pos;
            let icon_font = FontId::proportional(icon_style.icon_size);
            painter.text(
                icon_pos,
                Align2::CENTER_CENTER,
                icon_style.icon_character.to_string(),
                icon_font,
                fade_color(icon_style.icon_color, faded),
            );
        }
        Rect::from_center_size(pos, Vec2::new(NODE_RADIUS * 2.0, NODE_RADIUS * 2.0))
    };
    if highlighted {
        let hrec = galley.rect.translate(Vec2::new(text_pos.x, text_pos.y));
        painter.rect_filled(hrec, 3.0, Color32::from_rgba_unmultiplied(255, 255, 153, 200));
    }
    if show_labels || highlighted {
        painter.galley(text_pos, galley, Color32::BLACK);
        if let Some(icon_style) = &type_style.icon_style {
            let icon_pos = match icon_style.icon_position {
                IconPosition::Center => pos,
                IconPosition::Above => text_pos + Vec2::new(text_rect.width() / 2.0, -icon_style.icon_size / 2.0),
                IconPosition::Below => {
                    text_pos + Vec2::new(text_rect.width() / 2.0, text_rect.height() + icon_style.icon_size / 2.0)
                }
                IconPosition::Left => text_pos + Vec2::new(-icon_style.icon_size / 2.0, text_rect.height() / 2.0),
                IconPosition::Right => {
                    text_pos + Vec2::new(text_rect.width() + icon_style.icon_size / 2.0, text_rect.height() / 2.0)
                }
            };
            let icon_font = FontId::proportional(icon_style.icon_size);
            painter.text(
                icon_pos,
                Align2::CENTER_CENTER,
                icon_style.icon_character.to_string(),
                icon_font,
                fade_color(icon_style.icon_color, faded),
            );
        }
    }
    if type_style.node_shape == NodeShape::Circle {
        (
            Rect::from_center_size(node_rect.center(), Vec2::splat(node_rect.width())),
            NodeShape::Circle,
        )
    } else {
        (node_rect, type_style.node_shape)
    }
}
