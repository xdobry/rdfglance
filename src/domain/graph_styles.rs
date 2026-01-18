use std::collections::HashMap;

use egui::Color32;

use crate::{
    IriIndex, domain::StringIndexer, support::distinct_colors::next_distinct_color, ui::table_view::TypeInstanceIndex
};

pub struct NodeStyle {
    pub color: egui::Color32,
    pub priority: u32,
    pub label_index: IriIndex,
    pub node_shape: NodeShape,
    pub node_size: NodeSize,
    pub width: f32,
    pub height: f32,
    pub border_width: f32,
    pub border_color: egui::Color32,
    pub corner_radius: f32,
    pub max_lines: u16,
    pub label_position: LabelPosition,
    pub label_max_width: f32,
    pub font_size: f32,
    pub label_color: egui::Color32,
    pub icon_style: Option<IconStyle>,
    pub is_default: bool,
}

impl Default for NodeStyle {
    fn default() -> Self {
        Self {
            color: egui::Color32::WHITE,
            priority: 0,
            label_index: 0,
            node_shape: NodeShape::Circle,
            node_size: NodeSize::Fixed,
            width: 10.0,
            height: 10.0,
            border_width: 1.0,
            border_color: egui::Color32::BLACK,
            corner_radius: 3.0,
            max_lines: 1,
            label_position: LabelPosition::Above,
            label_max_width: 0.0,
            font_size: 16.0,
            label_color: egui::Color32::BLACK,
            icon_style: None,
            is_default: true,
        }
    }
}

pub struct IconStyle {
    pub icon_character: char,
    pub icon_position: IconPosition,
    pub icon_size: f32,
    pub icon_color: egui::Color32,
}

impl Default for IconStyle {
    fn default() -> Self {
        Self {
            icon_character: '\u{2606}',
            icon_position: IconPosition::Center,
            icon_size: 20.0,
            icon_color: Color32::BLACK,
        }
    }
}

pub struct EdgeFont {
    pub font_size: f32,
    pub font_color: Color32,
}

impl Default for EdgeFont {
    fn default() -> Self {
        Self {
            font_size: 16.0,
            font_color: Color32::BLACK,
        }
    }
}

#[derive(PartialEq, Copy, Clone, Debug)]
#[repr(u8)]
pub enum NodeShape {
    None = 0,
    Rect = 1,
    Circle = 2,
    Ellipse = 3,
}

impl TryFrom<u8> for NodeShape {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(NodeShape::None),
            1 => Ok(NodeShape::Rect),
            2 => Ok(NodeShape::Circle),
            3 => Ok(NodeShape::Ellipse),
            _ => Err(()),
        }
    }
}

#[derive(PartialEq, Copy, Clone, Debug)]
#[repr(u8)]
pub enum NodeSize {
    Fixed = 1,
    Label = 2,
}

impl TryFrom<u8> for NodeSize {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(NodeSize::Fixed),
            2 => Ok(NodeSize::Label),
            _ => Err(()),
        }
    }
}

#[derive(PartialEq, Copy, Clone, Debug)]
#[repr(u8)]
pub enum LabelPosition {
    Center = 1,
    Above = 2,
    Below = 3,
    Right = 4,
    Left = 5,
}

impl TryFrom<u8> for LabelPosition {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(LabelPosition::Center),
            2 => Ok(LabelPosition::Above),
            3 => Ok(LabelPosition::Below),
            4 => Ok(LabelPosition::Right),
            5 => Ok(LabelPosition::Left),
            _ => Err(()),
        }
    }
}

#[derive(PartialEq, Copy, Clone, Debug)]
#[repr(u8)]
pub enum IconPosition {
    Center = 1,
    Above = 2,
    Below = 3,
    Right = 4,
    Left = 5,
}

impl TryFrom<u8> for IconPosition {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(IconPosition::Center),
            2 => Ok(IconPosition::Above),
            3 => Ok(IconPosition::Below),
            4 => Ok(IconPosition::Right),
            5 => Ok(IconPosition::Left),
            _ => Err(()),
        }
    }
}

pub struct EdgeStyle {
    pub color: egui::Color32,
    pub width: f32,
    pub line_gap: f32,
    pub line_style: LineStyle,
    pub target_style: ArrowStyle,
    pub arrow_location: ArrowLocation,
    pub arrow_size: f32,
    pub icon_style: Option<IconStyle>,
    pub edge_font: Option<EdgeFont>,
}

impl Default for EdgeStyle {
    fn default() -> Self {
        Self {
            color: egui::Color32::BLACK,
            width: 2.0,
            icon_style: None,
            edge_font: None,
            line_style: LineStyle::Solid,
            target_style: ArrowStyle::Arrow,
            arrow_location: ArrowLocation::Target,
            line_gap: 10.0,
            arrow_size: 6.0,
        }
    }
}

#[derive(PartialEq, Copy, Clone, Debug)]
#[repr(u8)]
pub enum LineStyle {
    Solid,
    Dashed,
    Dotted,
}

impl TryFrom<u8> for LineStyle {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(LineStyle::Solid),
            1 => Ok(LineStyle::Dashed),
            2 => Ok(LineStyle::Dotted),
            _ => Err(()),
        }
    }
}

#[derive(PartialEq, Copy, Clone)]
#[repr(u8)]
pub enum ArrowStyle {
    Arrow = 0,
    ArrorFilled = 1,
    ArrorTriangle = 2,
}

impl TryFrom<u8> for ArrowStyle {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(ArrowStyle::Arrow),
            1 => Ok(ArrowStyle::ArrorFilled),
            2 => Ok(ArrowStyle::ArrorTriangle),
            _ => Err(()),
        }
    }
}

#[derive(PartialEq, Copy, Clone, Debug)]
#[repr(u8)]
pub enum ArrowLocation {
    Target = 0,
    Middle = 1,
    None = 2,
}

impl TryFrom<u8> for ArrowLocation {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(ArrowLocation::Target),
            1 => Ok(ArrowLocation::Middle),
            2 => Ok(ArrowLocation::None),
            _ => Err(()),
        }
    }
}

pub struct GVisualizationStyle {
    pub node_styles: HashMap<IriIndex, NodeStyle>,
    pub default_node_style: NodeStyle,
    pub edge_styles: HashMap<IriIndex, EdgeStyle>,
    pub use_size_overwrite: bool,
    pub use_color_overwrite: bool,
    pub default_label_in_node: bool,
    pub min_size: f32,
    pub max_size: f32,
}

impl GVisualizationStyle {
    pub fn preset_styles(&mut self, type_instance_index: &TypeInstanceIndex, predicate_indexer: &StringIndexer, is_dark_mode: bool) {
        for (type_index, type_desc) in type_instance_index.types.iter() {
            let type_style = self.node_styles.get(type_index);
            if type_style.is_none() {
                let lightness = if is_dark_mode { 0.3 } else { 0.6 };
                let new_color = next_distinct_color(self.node_styles.len(), 0.8, lightness, 200);
                let order = type_instance_index.types_order.iter().position(|&i| i == *type_index);
                let priority = order.map(|o| o as u32).unwrap_or(0);
                let label_index = if type_desc.properties.contains_key(&0) {
                    // use rdf:label if exists other wise use first property that has name or label in iri and min occurs is greater 1
                    0
                } else {
                    let mut selected_label_index: Option<IriIndex> = None;
                    for (prop_index, prop_desc) in type_desc.properties.iter() {
                        if prop_desc.min_cardinality >= 1 {
                            if let Some(label_str) = predicate_indexer.index_to_str(*prop_index) {
                                if label_str.contains("label") || label_str.contains("name") || label_str.contains("Name") {
                                    selected_label_index = Some(*prop_index);
                                    break;
                                }
                            }
                        }
                    }
                    selected_label_index.unwrap_or(0)
                };
                self.node_styles.insert(
                    *type_index,
                    NodeStyle {
                        color: new_color,
                        priority,
                        label_index,
                        ..Default::default()
                    },
                );
            }
        }
    }

    pub fn change_default_styles(&mut self) {
        for style in self.node_styles.values_mut() {
            if style.is_default {
                if self.default_label_in_node {
                    style.label_position = LabelPosition::Center;
                    style.node_size = NodeSize::Label;
                    style.node_shape = NodeShape::Rect;
                    style.max_lines = 3;
                    style.label_max_width = 130.0;
                } else {
                    style.label_position = LabelPosition::Above;
                    style.node_size = NodeSize::Fixed;
                    style.node_shape = NodeShape::Circle;
                    style.max_lines = 0;
                    style.label_max_width = 0.0;
                }
            }            
        }
    }

    pub fn get_type_style(&self, types: &Vec<IriIndex>) -> &NodeStyle {
        let mut style: Option<&NodeStyle> = None;
        for type_iri in types {
            let type_style = self.node_styles.get(type_iri);
            if let Some(type_style) = type_style {
                if let Some(current_style) = style {
                    if type_style.priority > current_style.priority {
                        style = Some(type_style);
                    }
                } else {
                    style = Some(type_style);
                }
            }
        }
        style.unwrap_or(&self.default_node_style)
    }

    pub fn get_type_style_one(&self, type_iri: IriIndex) -> &NodeStyle {
        self.node_styles.get(&type_iri).unwrap_or(&self.default_node_style)
    }

    pub fn get_predicate_color(&mut self, iri: IriIndex, is_dark_mode: bool) -> egui::Color32 {
        let len = self.edge_styles.len();
        self.edge_styles
            .entry(iri)
            .or_insert_with(|| {
                let lightness = if is_dark_mode { 0.6 } else { 0.3 };
                EdgeStyle {
                    color: next_distinct_color(len, 0.5, lightness, 170),
                    ..EdgeStyle::default()
                }
            })
            .color
    }

    pub fn get_edge_syle(&mut self, iri: IriIndex, is_dark_mode: bool) -> &EdgeStyle {
        let len = self.edge_styles.len();
        self.edge_styles.entry(iri).or_insert_with(|| {
            let lightness = if is_dark_mode { 0.6 } else { 0.3 };
            EdgeStyle {
                color: next_distinct_color(len, 0.5, lightness, 170),
                ..EdgeStyle::default()
            }
        })
    }

    pub fn update_label(&mut self, iri: IriIndex, label_index: IriIndex) {
        if let Some(type_style) = self.node_styles.get_mut(&iri) {
            type_style.label_index = label_index;
        }
    }

    pub fn clean(&mut self) {
        self.node_styles.clear();
        self.edge_styles.clear();
    }
}

