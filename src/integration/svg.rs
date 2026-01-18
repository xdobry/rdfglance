use std::io;

use crate::{
    RdfGlanceApp,
    domain::{
        Indexers, LabelContext, NObject, NodeData,
        config::Config,
        graph_styles::{ArrowLocation, EdgeStyle, GVisualizationStyle, LabelPosition, NodeShape, NodeSize, NodeStyle},
    },
    support::distinct_colors::next_distinct_color,
    uistate::{UIState, layout::IndividualNodeStyleData},
};
use egui::{Align2, Color32, Pos2, Rect, Vec2};
use quick_xml::Writer;
use quick_xml::events::{BytesEnd, BytesStart, BytesText, Event};

const POS_SPACE: f32 = 3.0;

impl RdfGlanceApp {
    pub fn export_svg<W: io::Write>(
        &self,
        wtr: &mut W,
        node_data: &NodeData,
        label_context: &LabelContext,
    ) -> std::io::Result<()> {
        if let Ok(positions) = self.visible_nodes.positions.read() {
            if let Ok(nodes) = self.visible_nodes.nodes.read() {
                if let Ok(individual_node_style) = self.visible_nodes.individual_node_styles.read() {
                    if let Ok(node_shapes) = self.visible_nodes.node_shapes.read() {
                        let mut view_rect = Rect::NOTHING;
                        for (position, shape) in positions.iter().zip(node_shapes.iter()) {
                            view_rect.extend_with(position.pos - shape.size);
                            view_rect.extend_with(position.pos + shape.size);
                        }
                        let mut writer = Writer::new_with_indent(wtr, b' ', 2);

                        // XML declaration (optional but recommended)
                        writer.write_event(Event::Decl(quick_xml::events::BytesDecl::new(
                            "1.0",
                            Some("UTF-8"),
                            None,
                        )))?;

                        // <svg ...>
                        let mut svg = BytesStart::new("svg");
                        svg.push_attribute(("xmlns", "http://www.w3.org/2000/svg"));
                        svg.push_attribute(("version", "1.1"));
                        svg.push_attribute((
                            "viewBox",
                            format!(
                                "{} {} {} {}",
                                view_rect.min.x,
                                view_rect.min.y,
                                view_rect.width(),
                                view_rect.height()
                            )
                            .as_str(),
                        ));
                        writer.write_event(Event::Start(svg))?;

                        writer.write_event(Event::Start(BytesStart::new("defs")))?;
                        let mut marker = BytesStart::new("marker");
                        marker.push_attribute(("id", "arrow"));
                        marker.push_attribute(("viewBox", "0 0 10 10"));
                        marker.push_attribute(("refX", "10"));
                        marker.push_attribute(("refY", "5"));
                        marker.push_attribute(("markerWidth", "6"));
                        marker.push_attribute(("markerHeight", "6"));
                        marker.push_attribute(("orient", "auto"));

                        writer.write_event(Event::Start(marker))?;

                        let mut path = BytesStart::new("path");
                        path.push_attribute(("d", "M 0 0 L 10 5 L 0 10 z"));
                        path.push_attribute(("fill", "context-stroke"));

                        writer.write_event(Event::Empty(path))?;

                        writer.write_event(Event::End(BytesEnd::new("marker")))?;
                        writer.write_event(Event::End(BytesEnd::new("defs")))?;

                        let default_edge_style = EdgeStyle::default();
                        if self.visible_nodes.show_orthogonal
                            && let Some(orth_edges) = &self.visible_nodes.orth_edges
                        {
                            for orth_edge in orth_edges.edges.iter() {
                                if self.visible_nodes.has_semantic_zoom {
                                    if !individual_node_style[orth_edge.from_node]
                                        .semantic_zoom_interval
                                        .is_visible(self.ui_state.semantic_zoom_magnitude)
                                        || !individual_node_style[orth_edge.to_node]
                                            .semantic_zoom_interval
                                            .is_visible(self.ui_state.semantic_zoom_magnitude)
                                    {
                                        continue;
                                    }
                                }
                                draw_orth_edge_svg(
                                    &mut writer,
                                    self.visualization_style
                                        .edge_styles
                                        .get(&orth_edge.predicate)
                                        .unwrap_or(&default_edge_style),
                                    &orth_edge.control_points,
                                )?;
                            }
                        } else if let Ok(edges) = self.visible_nodes.edges.read() {
                            for edge in edges.iter() {
                                if self.ui_state.hidden_predicates.contains(edge.predicate) {
                                    continue;
                                }
                                if self.visible_nodes.has_semantic_zoom {
                                    if !individual_node_style[edge.from]
                                        .semantic_zoom_interval
                                        .is_visible(self.ui_state.semantic_zoom_magnitude)
                                        || !individual_node_style[edge.to]
                                            .semantic_zoom_interval
                                            .is_visible(self.ui_state.semantic_zoom_magnitude)
                                    {
                                        continue;
                                    }
                                }

                                let node_label = || {
                                    let reference_label = node_data.predicate_display(
                                        edge.predicate,
                                        &label_context,
                                        &node_data.indexers,
                                    );
                                    reference_label.as_str().to_owned()
                                };
                                let pos1 = positions[edge.from].pos;
                                if edge.from != edge.to {
                                    let node_shape_from = &node_shapes[edge.from];
                                    let node_shape_to = &node_shapes[edge.to];
                                    let pos2 = positions[edge.to].pos;
                                    draw_edge_svg(
                                        &mut writer,
                                        pos1,
                                        node_shape_from.size,
                                        node_shape_from.node_shape,
                                        pos2,
                                        node_shape_to.size,
                                        node_shape_to.node_shape,
                                        self.visualization_style
                                            .edge_styles
                                            .get(&edge.predicate)
                                            .unwrap_or(&default_edge_style),
                                        node_label,
                                        edge.bezier_distance,
                                    )?;
                                } else {
                                    let node_shape_from = &node_shapes[edge.from];
                                    draw_self_edge_svg(
                                        &mut writer,
                                        pos1,
                                        node_shape_from.size,
                                        edge.bezier_distance,
                                        node_shape_from.node_shape,
                                        self.visualization_style
                                            .edge_styles
                                            .get(&edge.predicate)
                                            .unwrap_or(&default_edge_style),
                                        node_label,
                                    )?;
                                }
                            }
                        }

                        for ((node_pos, node_layout), node_position) in nodes.iter().enumerate().zip(positions.iter()) {
                            if let Some((object_iri, object)) = node_data.get_node_by_index(node_layout.node_index) {
                                if self.visible_nodes.has_semantic_zoom && !self.visible_nodes.update_node_shapes {
                                    if !individual_node_style[node_pos]
                                        .semantic_zoom_interval
                                        .is_visible(self.ui_state.semantic_zoom_magnitude)
                                    {
                                        continue;
                                    }
                                }
                                draw_node_svg(
                                    &self.visualization_style,
                                    individual_node_style.get(node_pos),
                                    &node_data.indexers,
                                    &self.ui_state,
                                    &self.persistent_data.config_data,
                                    &mut writer,
                                    object,
                                    object_iri,
                                    node_position.pos,
                                    node_shapes[node_pos].size,
                                )?;
                            }
                        }
                        // </svg>
                        writer.write_event(Event::End(BytesEnd::new("svg")))?;
                    }
                }
            }
        }

        Ok(())
    }
}

fn draw_node_svg<W: io::Write>(
    visualization_style: &GVisualizationStyle,
    individual_node_style: Option<&IndividualNodeStyleData>,
    indexers: &Indexers,
    ui_state: &UIState,
    config: &Config,
    writer: &mut Writer<W>,
    node_object: &NObject,
    object_iri: &str,
    pos: Pos2,
    size: Vec2,
) -> std::io::Result<()> {
    let node_type_style = visualization_style.get_type_style(&node_object.types);
    let type_style = if (visualization_style.use_size_overwrite || visualization_style.use_color_overwrite)
        && individual_node_style.is_some()
    {
        let individual_node_style = individual_node_style.unwrap();
        let overwrite_size = visualization_style.use_size_overwrite && !individual_node_style.size_overwrite.is_nan();
        &NodeStyle {
            color: if visualization_style.use_color_overwrite && individual_node_style.color_overwrite > 0 {
                let lightness = 0.6;
                next_distinct_color(individual_node_style.color_overwrite as usize - 1, 0.8, lightness, 200)
            } else {
                node_type_style.color
            },
            priority: 100,
            label_index: node_type_style.label_index,
            node_shape: if overwrite_size {
                NodeShape::Circle
            } else {
                node_type_style.node_shape
            },
            node_size: if overwrite_size {
                NodeSize::Fixed
            } else {
                node_type_style.node_size
            },
            width: if overwrite_size {
                individual_node_style.size_overwrite
            } else {
                node_type_style.width
            },
            height: node_type_style.height,
            border_width: node_type_style.border_width,
            border_color: node_type_style.border_color,
            corner_radius: node_type_style.corner_radius,
            max_lines: node_type_style.max_lines,
            label_position: node_type_style.label_position,
            label_max_width: node_type_style.label_max_width,
            font_size: node_type_style.font_size,
            label_color: node_type_style.label_color,
            icon_style: None,
            is_default: false,
        }
    } else {
        node_type_style
    };
    let node_label = node_object.node_label(
        object_iri,
        visualization_style,
        config.short_iri,
        ui_state.display_language,
        indexers,
    );
    let display_num_hidden_refs = if ui_state.show_num_hidden_refs {
        individual_node_style.map_or(0, |f| f.hidden_references)
    } else {
        0
    };
    draw_node_label_svg(
        writer,
        node_label,
        type_style,
        pos,
        size,
        ui_state.show_labels,
        display_num_hidden_refs,
    )
}

fn draw_node_label_svg<W: io::Write>(
    writer: &mut Writer<W>,
    node_label: &str,
    type_style: &NodeStyle,
    pos: Pos2,
    size: Vec2,
    show_labels: bool,
    num_hidden_references: u32,
) -> std::io::Result<()> {
    let node_rect = {
        let node_rect = match type_style.node_size {
            NodeSize::Fixed => Rect::from_center_size(pos, Vec2::new(type_style.width, type_style.height)),
            NodeSize::Label => Rect::from_center_size(pos, size),
        };
        match type_style.node_shape {
            NodeShape::Circle => {
                let mut circle = BytesStart::new("circle");
                circle.push_attribute(("cx", node_rect.center().x.to_string().as_str()));
                circle.push_attribute(("cy", node_rect.center().y.to_string().as_str()));
                circle.push_attribute(("r", (node_rect.width() / 2.0).to_string().as_str()));
                fill_stoke(&mut circle, type_style);
                writer.write_event(Event::Empty(circle))?;
            }
            NodeShape::Ellipse => {
                let mut ellipse = BytesStart::new("ellipse");
                ellipse.push_attribute(("cx", node_rect.center().x.to_string().as_str()));
                ellipse.push_attribute(("cy", node_rect.center().y.to_string().as_str()));
                ellipse.push_attribute(("rx", (node_rect.width() / 2.0).to_string().as_str()));
                ellipse.push_attribute(("ry", (node_rect.width() / 2.0).to_string().as_str()));
                fill_stoke(&mut ellipse, type_style);
                writer.write_event(Event::Empty(ellipse))?;
            }
            NodeShape::Rect => {
                let mut rect = BytesStart::new("rect");
                rect.push_attribute(("x", node_rect.min.x.to_string().as_str()));
                rect.push_attribute(("y", node_rect.min.y.to_string().as_str()));
                if type_style.corner_radius > 0.0 {
                    rect.push_attribute(("rx", type_style.corner_radius.to_string().as_str()));
                }
                rect.push_attribute(("width", node_rect.width().to_string().as_str()));
                rect.push_attribute(("height", node_rect.height().to_string().as_str()));
                fill_stoke(&mut rect, type_style);
                // <rect ... />
                writer.write_event(Event::Empty(rect))?;
            }
            NodeShape::None => {
                // No shape, just text
            }
        }
        node_rect
    };
    if show_labels {
        let (text_pos, text_anchor, baseline): (Pos2, &str, &str) = match type_style.label_position {
            LabelPosition::Center => (node_rect.center(), "middle", "middle"),
            LabelPosition::Above => (
                Pos2::new(node_rect.center().x, node_rect.min.y - POS_SPACE),
                "middle",
                "auto",
            ),
            LabelPosition::Below => (
                Pos2::new(node_rect.center().x, node_rect.max.y + POS_SPACE),
                "middle",
                "hanging",
            ),
            LabelPosition::Left => (Pos2::new(node_rect.min.x, node_rect.center().y), "end", "middle"),
            LabelPosition::Right => (Pos2::new(node_rect.max.x, node_rect.center().y), "start", "middle"),
        };
        let mut text = BytesStart::new("text");
        text.push_attribute(("x", text_pos.x.to_string().as_str()));
        text.push_attribute(("y", text_pos.y.to_string().as_str()));
        add_color(&mut text, "fill", type_style.label_color);
        text.push_attribute(("font-size", type_style.font_size.to_string().as_str()));
        text.push_attribute(("text-anchor", text_anchor));
        text.push_attribute(("dominant-baseline", baseline));
        writer.write_event(Event::Start(text))?;
        let num_text_event = BytesText::new(node_label);
        writer.write_event(Event::Text(num_text_event))?;
        writer.write_event(Event::End(BytesEnd::new("text")))?;
    }
    if num_hidden_references > 0 {
        let (num_pos, anchor) = if matches!(type_style.label_position, LabelPosition::Right) {
            let num_pos = node_rect.right_bottom() + Vec2::new(node_rect.width() * -0.5, 3.0);
            (num_pos, Align2::CENTER_TOP)
        } else {
            let num_pos = node_rect.right_top() + Vec2::new(3.0, 0.0);
            (num_pos, Align2::LEFT_TOP)
        };
        let num_text = num_hidden_references.to_string();
        let mut text = BytesStart::new("text");
        text.push_attribute(("x", num_pos.x.to_string().as_str()));
        text.push_attribute(("y", num_pos.y.to_string().as_str()));
        text.push_attribute(("fill", "gray"));
        if anchor == Align2::CENTER_TOP {
            text.push_attribute(("text-anchor", "middle"));
        } else {
            text.push_attribute(("text-anchor", "start"));
        }
        text.push_attribute(("dominant-baseline", "hanging"));
        writer.write_event(Event::Start(text))?;
        let num_text_event = BytesText::new(num_text.as_str());
        writer.write_event(Event::Text(num_text_event))?;
        writer.write_event(Event::End(BytesEnd::new("text")))?;
    }
    Ok(())
}

fn fill_stoke(xml_node: &mut BytesStart, type_style: &NodeStyle) {
    add_color(xml_node, "fill", type_style.color);
    if type_style.border_width > 0.0 {
        xml_node.push_attribute(("stroke-width", type_style.border_width.to_string().as_str()));
        add_color(xml_node, "stroke", type_style.border_color);
    }
}

fn add_color(xml_node: &mut BytesStart, attr: &str, color: Color32) {
    let col = color.to_array();
    xml_node.push_attribute((attr, format!("#{:02X}{:02X}{:02X}", col[0], col[1], col[2]).as_str()));
    if col[3] < 255 {
        xml_node.push_attribute((
            format!("{}-opacity", attr).as_str(),
            format!("{:.2}", col[3] as f32 / 255.0).as_str(),
        ));
    }
}

fn color_to_hex4(color: Color32) -> String {
    let col = color.to_array();
    format!("#{:02X}{:02X}{:02X}{:02X}", col[0], col[1], col[2], col[3])
}

fn draw_edge_svg<F, W>(
    writer: &mut Writer<W>,
    point_from: Pos2,
    size_from: Vec2,
    shape_from: NodeShape,
    point_to: Pos2,
    size_to: Vec2,
    shape_to: NodeShape,
    edge_style: &EdgeStyle,
    label_cb: F,
    bezier_distance: f32,
) -> std::io::Result<()>
where
    F: Fn() -> String,
    W: io::Write,
{
    let dir = point_to - point_from;

    // Compute the length (Euclidean distance)
    let length = dir.length();
    let radius_to = size_to.x / 2.0;
    let radius_from = size_from.x / 2.0;

    if !matches!(shape_to, NodeShape::Rect)
        && !matches!(shape_from, NodeShape::Rect)
        && length <= radius_to + radius_from
    {
        // nodes are non rect (so handle as circles) overlapping, no edge needed
        return Ok(());
    }
    if matches!(shape_to, NodeShape::Rect) && matches!(shape_from, NodeShape::Rect) {
        // both nodes are rect, test if react overlapping
        let rect_from = Rect::from_center_size(point_from, size_from);
        let rect_to = Rect::from_center_size(point_to, size_to);
        if rect_from.intersects(rect_to) {
            // nodes are overlapping, no edge needed
            return Ok(());
        }
    }

    // Normalize and scale to radius
    let unit = dir / length;

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
                    return Ok(());
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
                    return Ok(());
                }
            }
            edge_from
        }
        _ => point_from + unit * radius_from,
    };
    if bezier_distance != 0.0 {
        let middle = (edge_from + edge_to.to_vec2()) / 2.0;
        let ctrl_pos = middle + unit.rot90() * bezier_distance;
        let d = format!(
            "M {} {} Q {} {}, {} {}",
            edge_from.x, edge_from.y, ctrl_pos.x, ctrl_pos.y, edge_to.x, edge_to.y
        );

        let mut path = BytesStart::new("path");
        path.push_attribute(("d", d.as_str()));
        path.push_attribute(("fill", "none"));
        path.push_attribute(("stroke-width", edge_style.width.to_string().as_str()));
        add_color(&mut path, "stroke", edge_style.color);
        if !matches!(edge_style.arrow_location, ArrowLocation::None) {
            path.push_attribute(("marker-end", "url(#arrow)"));
        }
        writer.write_event(Event::Empty(path))?;
    } else {
        let mut s_line: BytesStart<'_> = BytesStart::new("line");
        s_line.push_attribute(("x1", edge_from.x.to_string().as_str()));
        s_line.push_attribute(("y1", edge_from.y.to_string().as_str()));
        s_line.push_attribute(("x2", edge_to.x.to_string().as_str()));
        s_line.push_attribute(("y2", edge_to.y.to_string().as_str()));
        s_line.push_attribute(("stroke-width", edge_style.width.to_string().as_str()));
        add_color(&mut s_line, "stroke", edge_style.color);
        if !matches!(edge_style.arrow_location, ArrowLocation::None) {
            s_line.push_attribute(("marker-end", "url(#arrow)"));
        }
        writer.write_event(Event::Empty(s_line))?;
    }

    Ok(())
}

fn draw_self_edge_svg<F, W>(
    writer: &mut Writer<W>,
    point: Pos2,
    size: Vec2,
    rotation: f32,
    _shape: NodeShape,
    edge_style: &EdgeStyle,
    label_cb: F,
) -> std::io::Result<()>
where
    F: Fn() -> String,
    W: io::Write,
{
    let radius = size.x / 2.0;
    let angle_1 = std::f32::consts::FRAC_PI_4 + rotation;
    let angle_2 = angle_1 + std::f32::consts::FRAC_PI_2;
    let direction_1 = Vec2::new(angle_1.sin(), -angle_1.cos());
    let direction_2 = Vec2::new(angle_2.sin(), -angle_2.cos());
    let pos1 = point + (direction_1 * radius);
    let pos2 = point + (direction_2 * radius);
    let ctrl_pos_distance = 100.0;
    let ctrl_pos1 = point + direction_1 * (radius + ctrl_pos_distance);
    let ctrl_pos2 = point + direction_2 * (radius + ctrl_pos_distance);

    let d = format!(
        "M {} {} C {} {}, {} {}, {} {}",
        pos1.x, pos1.y, ctrl_pos1.x, ctrl_pos1.y, ctrl_pos2.x, ctrl_pos2.y, pos2.x, pos2.y
    );

    let mut path = BytesStart::new("path");
    path.push_attribute(("d", d.as_str()));
    path.push_attribute(("fill", "none"));
    path.push_attribute(("stroke-width", edge_style.width.to_string().as_str()));
    add_color(&mut path, "stroke", edge_style.color);
    if !matches!(edge_style.arrow_location, ArrowLocation::None) {
        path.push_attribute(("marker-end", "url(#arrow)"));
    }
    writer.write_event(Event::Empty(path))?;

    Ok(())
}

fn draw_orth_edge_svg<W: io::Write>(
    writer: &mut Writer<&mut W>,
    edge_style: &EdgeStyle,
    control_points: &[Pos2],
) -> std::io::Result<()> {
    
    let mut polyline = BytesStart::new("polyline");
    let points = control_points
        .iter()
        .map(|p| format!("{},{}", p.x, p.y))
        .collect::<Vec<String>>()
        .join(" ");
    polyline.push_attribute(("points", points.as_str()));
    polyline.push_attribute(("fill", "none"));
    polyline.push_attribute(("stroke-width", edge_style.width.to_string().as_str()));
    add_color(&mut polyline, "stroke", edge_style.color);
    if !matches!(edge_style.arrow_location, ArrowLocation::None) {
        polyline.push_attribute(("marker-end", "url(#arrow)"));
    }
    writer.write_event(Event::Empty(polyline))?;

    Ok(())
}
