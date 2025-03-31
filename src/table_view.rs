use std::{cell, cmp::min, collections::HashMap, time::Instant, vec};

use const_format::concatcp;
use egui::{Align, Align2, Color32, CursorIcon, Layout, Pos2, Rect, Sense, Slider, Stroke, Vec2};
use egui_extras::{Column, StripBuilder, TableBuilder};

use crate::{
    browse_view::show_references, config::IriDisplay, nobject::{IriIndex, NodeData}, play_ground::ScrollBar, prefix_manager::PrefixManager, rdfwrap::{self, RDFWrap}, style::{ICON_CLOSE, ICON_GRAPH}, uitools::{popup_at, strong_unselectable}, ColorCache, LayoutData, NodeAction
};

pub struct CacheStatistics {
    pub nodes: usize,
    pub unique_predicates: usize,
    pub unique_types: usize,
    pub properties: usize,
    pub references: usize,
    pub blank_nodes: usize,
    pub unresolved_references: usize,
    pub types: HashMap<IriIndex, TypeData>,
    pub types_order: Vec<IriIndex>,
    pub selected_type: Option<usize>,
}

pub struct DataPropCharacteristics {
    pub count: u32,
    pub max_len: u32,
}

pub struct TypeData {
    pub instances: Vec<IriIndex>,
    pub filtered_instances: Vec<IriIndex>,
    pub properties: HashMap<IriIndex, DataPropCharacteristics>,
    pub references: HashMap<IriIndex, u32>,
    pub rev_references: HashMap<IriIndex, u32>,
    pub instance_view: InstanceView,
}

pub struct InstanceView {
    // Used for Y ScrollBar
    pub pos: f32,
    pub drag_pos: Option<f32>,
    pub display_properties: Vec<ColumnDesc>,
    pub instance_filter: String,
    context_menu: TableContextMenu,
    pub column_pos: u32,
    pub column_resize: Option<(Pos2, IriIndex)>,
}

enum TableContextMenu {
    None,
    ColumnMenu(Pos2, IriIndex),
    CellMenu(Pos2, IriIndex, IriIndex),
    RefMenu(Pos2, IriIndex),
    RefColumnMenu(Pos2),
}

impl TableContextMenu {
    pub fn pos(&self) -> Pos2 {
        match self {
            TableContextMenu::ColumnMenu(pos, _) => *pos,
            TableContextMenu::CellMenu(pos, _, _) => *pos,
            TableContextMenu::RefMenu(pos, _) => *pos,
            TableContextMenu::RefColumnMenu(pos) => *pos,
            TableContextMenu::None => Pos2::new(0.0, 0.0),
        }
    }
}

impl InstanceView {
    pub fn get_column(&self, predicate_index: IriIndex) -> Option<&ColumnDesc> {
        for column_desc in &self.display_properties {
            if column_desc.predicate_index == predicate_index {
                return Some(column_desc);
            }
        }
        return None;
    }
    pub fn visible_columns(&self) -> u32 {
        let mut count = 0;
        for column_desc in &self.display_properties {
            if column_desc.visible {
                count += 1;
            }
        }
        return count;
    }
}

pub struct ColumnDesc {
    pub predicate_index: IriIndex,
    pub width: f32,
    pub visible: bool,
}

const ROW_HIGHT: f32 = 17.0;
const CHAR_WIDTH: f32 = 8.0;
const DEFAULT_COLUMN_WIDTH: f32 = 220.0;
const COLUMN_GAP: f32 = 2.0;

impl TypeData {
    pub fn new(_type_index: IriIndex) -> Self {
        Self {
            instances: Vec::new(),
            filtered_instances: Vec::new(),
            properties: HashMap::new(),
            references: HashMap::new(),
            rev_references: HashMap::new(),
            instance_view: InstanceView {
                pos: 0.0,
                drag_pos: None,
                column_pos: 0,
                display_properties: vec![],
                instance_filter: String::new(),
                context_menu: TableContextMenu::None,
                column_resize: None,
            },
        }
    }
    pub fn count_property(&mut self, property_index: IriIndex, count_number: u32) {
        let count = self
            .properties
            .entry(property_index)
            .or_insert(DataPropCharacteristics {
                count: 0,
                max_len: 0,
            });
        count.count += count_number;
    }
    pub fn max_len_property(&mut self, property_index: IriIndex, len: u32) {
        let count = self
            .properties
            .entry(property_index)
            .or_insert(DataPropCharacteristics {
                count: 0,
                max_len: 0,
            });
        count.max_len = count.max_len.max(len);
    }
    pub fn count_reverence(&mut self, reference_index: IriIndex, count_number: u32) {
        let count = self.references.entry(reference_index).or_insert(0);
        *count += count_number;
    }
    pub fn count_rev_reference(&mut self, reference_index: IriIndex, count_number: u32) {
        let count = self.rev_references.entry(reference_index).or_insert(0);
        *count += count_number;
    }

    pub fn instance_table(
        &mut self,
        ui: &mut egui::Ui,
        table_action: &mut TableAction,
        instance_action: &mut NodeAction,
        node_data: &mut NodeData,
        color_cache: &ColorCache,
        rdfwrap: &mut dyn rdfwrap::RDFAdapter,
        prefix_manager: &PrefixManager,
        layout_data: &LayoutData,
    ) {
        let instance_index = (self.instance_view.pos / ROW_HIGHT) as usize;
        let a_height = ui.available_height();
        let capacity = (a_height / ROW_HIGHT) as usize - 1;

        let available_rect = ui.max_rect(); // Get the full available area

        let available_width = ui.available_width();
        let available_height = ui.available_height();
        let size = Vec2::new(available_width, available_height);
        let (_rect, response) = ui.allocate_at_least(size, Sense::click_and_drag());
        let painter = ui.painter();
        let mouse_pos = response.hover_pos().unwrap_or(Pos2::new(0.0, 0.0));
        let secondary_clicked = response.secondary_clicked();
        let primary_clicked = response.clicked();

        let mut xpos = 0.0;
        let iri_len = 300.0;
        let ref_count_len = 80.0;

        let font_id = egui::FontId::default();
        let popup_id = ui.make_persistent_id("column_context_menu");

        painter.rect_filled(
            Rect::from_min_size(
                available_rect.left_top(),
                Vec2::new(available_width, ROW_HIGHT),
            ),
            0.0,
            Color32::GRAY,
        );

        painter.text(
            available_rect.left_top(),
            egui::Align2::LEFT_TOP,
            "iri",
            font_id.clone(),
            egui::Color32::BLACK,
        );

        painter.text(
            available_rect.left_top() + Vec2::new(iri_len, 0.0),
            egui::Align2::LEFT_TOP,
            "out/in",
            font_id.clone(),
            egui::Color32::BLACK,
        );
        let ref_column_rec = egui::Rect::from_min_size(
            available_rect.left_top() + Vec2::new(iri_len, 0.0),
            Vec2::new(ref_count_len, ROW_HIGHT),
        );
        let mut was_context_click = false;

        if ref_column_rec.contains(mouse_pos) {
            if secondary_clicked {
                was_context_click = true;
                ui.memory_mut(|mem| mem.toggle_popup(popup_id));
                self.instance_view.context_menu = TableContextMenu::RefColumnMenu(mouse_pos);
            } else {
                ui.output_mut(|o| o.cursor_icon = CursorIcon::ContextMenu);
            }
        }

        xpos += iri_len + ref_count_len;

        if response.drag_stopped() {
            self.instance_view.column_resize = None;
        }

        if let Some((start_pos, predicate_index)) = self.instance_view.column_resize {
            if response.dragged() {
                for column_desc in self.instance_view.display_properties.iter_mut() {
                    if column_desc.predicate_index == predicate_index {
                        let width = mouse_pos.x - start_pos.x;
                        if width > CHAR_WIDTH * 2.0 {
                            column_desc.width = width;
                        }
                    }
                }
            } else {
                self.instance_view.column_resize = None;
            }
        }

        for column_desc in self
            .instance_view
            .display_properties
            .iter()
            .filter(|p| p.visible)
            .skip(self.instance_view.column_pos as usize)
        {
            let top_left = available_rect.left_top() + Vec2::new(xpos, 0.0);
            let predicate_label = RDFWrap::iri2label_fallback(
                node_data
                    .get_predicate(column_desc.predicate_index)
                    .unwrap(),
            );
            text_wrapped(predicate_label, column_desc.width, painter, top_left, false);
            xpos += column_desc.width + COLUMN_GAP;
            let column_rect =
                egui::Rect::from_min_size(top_left, Vec2::new(column_desc.width, ROW_HIGHT));
            if column_rect.contains(mouse_pos) {
                if secondary_clicked {
                    was_context_click = true;
                    ui.memory_mut(|mem| mem.toggle_popup(popup_id));
                    self.instance_view.context_menu =
                        TableContextMenu::ColumnMenu(mouse_pos, column_desc.predicate_index);
                } else {
                    ui.output_mut(|o| o.cursor_icon = CursorIcon::ContextMenu);
                }
            }
            let colums_drag_size_rect = egui::Rect::from_min_size(
                top_left + Vec2::new(column_desc.width - 3.0, 0.0),
                Vec2::new(6.0, ROW_HIGHT),
            );
            if colums_drag_size_rect.contains(mouse_pos) {
                ui.output_mut(|o| o.cursor_icon = CursorIcon::ResizeHorizontal);
                if response.is_pointer_button_down_on()
                    && self.instance_view.column_resize.is_none()
                {
                    self.instance_view.column_resize = Some((
                        mouse_pos - Vec2::new(column_desc.width, 0.0),
                        column_desc.predicate_index,
                    ));
                }
            }
        }

        let mut ypos = ROW_HIGHT;
        let mut start_pos = instance_index;

        for instance_index in &self.filtered_instances
            [instance_index..min(instance_index + capacity, self.filtered_instances.len())]
        {
            let node = node_data.get_node_by_index(*instance_index);
            if let Some((node_iri, node)) = node {
                if start_pos % 2 == 0 {
                    painter.rect_filled(
                        Rect::from_min_size(
                            available_rect.left_top() + Vec2::new(0.0, ypos),
                            Vec2::new(available_width, ROW_HIGHT),
                        ),
                        0.0,
                        Color32::WHITE,
                    );
                }
                start_pos += 1;
                let mut xpos = iri_len + ref_count_len;

                let graph_button_width = 20.0;
                let graph_pos = available_rect.left_top() + Vec2::new(0.0, ypos+1.0);
                let button_rect = Rect::from_min_size(
                    graph_pos,
                    Vec2::new(graph_button_width, ROW_HIGHT-2.0),
                );
                let button_background = if button_rect.contains(mouse_pos) {
                    if primary_clicked {
                        *instance_action = NodeAction::ShowVisual(*instance_index);
                    }
                    Color32::YELLOW
                } else {
                    Color32::LIGHT_YELLOW
                };

                painter.rect_filled(button_rect, 3.0, button_background);
                painter.text(graph_pos+Vec2::new(graph_button_width/2.0,(ROW_HIGHT-2.0)/2.0), Align2::CENTER_CENTER , ICON_GRAPH, egui::FontId::default(), Color32::BLACK);

                let iri_top_left = available_rect.left_top() + Vec2::new(graph_button_width, ypos);

                let cell_rect =
                    egui::Rect::from_min_size(iri_top_left, Vec2::new(iri_len-graph_button_width, ROW_HIGHT));

                let mut cell_hovered = false;
                if cell_rect.contains(mouse_pos) {
                    ui.output_mut(|o| o.cursor_icon = CursorIcon::PointingHand);
                    cell_hovered = true;
                }

                text_wrapped_link(
                    &prefix_manager.get_prefixed(&node_iri),
                    iri_len-graph_button_width,
                    painter,
                    iri_top_left,
                    cell_hovered,
                );

                if primary_clicked && cell_rect.contains(mouse_pos) {
                    *instance_action = NodeAction::BrowseNode(*instance_index);
                } else if secondary_clicked && cell_rect.contains(mouse_pos) {
                    *instance_action = NodeAction::ShowVisual(*instance_index);
                }
                let s = format!(
                    "{}/{}",
                    node.references.len(),
                    node.reverse_references.len()
                );
                let ref_rect = egui::Rect::from_min_size(
                    available_rect.left_top() + Vec2::new(iri_len, ypos),
                    Vec2::new(ref_count_len, ROW_HIGHT),
                );
                painter.text(
                    ref_rect.left_top(),
                    egui::Align2::LEFT_TOP,
                    s,
                    font_id.clone(),
                    if ref_rect.contains(mouse_pos) { egui::Color32::DARK_BLUE } else { egui::Color32::BLACK },
                );
                if primary_clicked && ref_rect.contains(mouse_pos) {
                    was_context_click = true;
                    ui.memory_mut(|mem| mem.toggle_popup(popup_id));
                    self.instance_view.context_menu =
                        TableContextMenu::RefMenu(mouse_pos, *instance_index);
                }

                for column_desc in self
                    .instance_view
                    .display_properties
                    .iter()
                    .filter(|p| p.visible)
                    .skip(self.instance_view.column_pos as usize)
                {
                    let property = node
                        .get_property(column_desc.predicate_index, layout_data.display_language);
                    if let Some(property) = property {
                        let value = property.as_ref();
                        let cell_rect = egui::Rect::from_min_size(
                            available_rect.left_top() + Vec2::new(xpos, ypos),
                            Vec2::new(column_desc.width, ROW_HIGHT),
                        );
                        let mut cell_hovered = false;
                        if cell_rect.contains(mouse_pos) {
                            cell_hovered = true;
                        }
                        text_wrapped(value, column_desc.width, painter, cell_rect.left_top(), cell_hovered);
                        if primary_clicked && cell_rect.contains(mouse_pos) {
                            was_context_click = true;
                            ui.memory_mut(|mem| mem.toggle_popup(popup_id));
                            self.instance_view.context_menu = TableContextMenu::CellMenu(
                                mouse_pos,
                                *instance_index,
                                column_desc.predicate_index,
                            );
                        }
                    }
                    xpos += column_desc.width + COLUMN_GAP;
                    if xpos > available_rect.width() {
                        break;
                    }
                }
                ypos += ROW_HIGHT;
            }
        }
        // Draw vertical lines
        painter.line(
            [
                Pos2::new(
                    available_rect.left() + iri_len - COLUMN_GAP,
                    available_rect.top(),
                ),
                Pos2::new(
                    available_rect.left() + iri_len - COLUMN_GAP,
                    available_rect.top() + ypos,
                ),
            ]
            .to_vec(),
            Stroke::new(1.0, Color32::DARK_GRAY),
        );
        painter.line(
            [
                Pos2::new(
                    available_rect.left() + iri_len + ref_count_len + -COLUMN_GAP,
                    available_rect.top(),
                ),
                Pos2::new(
                    available_rect.left() + ref_count_len + iri_len - COLUMN_GAP,
                    available_rect.top() + ypos,
                ),
            ]
            .to_vec(),
            Stroke::new(1.0, Color32::DARK_GRAY),
        );
        xpos = iri_len + ref_count_len;
        for column_desc in self
            .instance_view
            .display_properties
            .iter()
            .filter(|p| p.visible)
            .skip(self.instance_view.column_pos as usize)
        {
            xpos += column_desc.width;
            painter.line(
                [
                    Pos2::new(available_rect.left() + xpos, available_rect.top()),
                    Pos2::new(available_rect.left() + xpos, available_rect.top() + ypos),
                ]
                .to_vec(),
                Stroke::new(1.0, Color32::DARK_GRAY),
            );
            xpos += COLUMN_GAP;
        }

        if !was_context_click && (secondary_clicked || primary_clicked) {
            self.instance_view.context_menu = TableContextMenu::None;
            ui.memory_mut(|mem| mem.close_popup());
        }
        let width = match self.instance_view.context_menu {
            TableContextMenu::CellMenu(_, _, _) => 500.0,
            _ => 200.0,
        };
        popup_at(
            ui,
            popup_id,
            self.instance_view.context_menu.pos(),
            width,
            |ui| match self.instance_view.context_menu {
                TableContextMenu::RefColumnMenu(_pos) => {
                    let mut close_menu: bool = false;
                    if ui.button("Sort Asc").clicked() {
                        *table_action = TableAction::SortRefAsc();
                        close_menu = true;
                    }
                    if ui.button("Sort Desc").clicked() {
                        *table_action = TableAction::SortRefDesc();
                        close_menu = true;
                    }
                    if close_menu {
                        self.instance_view.context_menu = TableContextMenu::None;
                        ui.memory_mut(|mem| mem.close_popup());
                    }
                }
                TableContextMenu::ColumnMenu(_pos, _column_predictate) => {
                    let mut close_menu = false;
                    if self.instance_view.visible_columns() > 0 {
                        if ui.button("Hide column").clicked() {
                            *table_action = TableAction::HideColumn(_column_predictate);
                            close_menu = true;
                        }
                    }
                    if ui.button("Sort Asc").clicked() {
                        *table_action = TableAction::SortColumnAsc(_column_predictate);
                        close_menu = true;
                    }
                    if ui.button("Sort Desc").clicked() {
                        *table_action = TableAction::SortColumnDesc(_column_predictate);
                        close_menu = true;
                    }
                    let hidden_columns: Vec<&ColumnDesc> = self
                        .instance_view
                        .display_properties
                        .iter()
                        .filter(|p| !p.visible)
                        .collect();
                    if hidden_columns.len() > 0 {
                        ui.separator();
                        ui.menu_button("Unhide Columns", |ui| {
                            for column_desc in hidden_columns {
                                if ui
                                    .button(RDFWrap::iri2label_fallback(
                                        node_data
                                            .get_predicate(column_desc.predicate_index)
                                            .unwrap(),
                                    ))
                                    .clicked()
                                {
                                    *table_action =
                                        TableAction::UhideColumn(column_desc.predicate_index);
                                    close_menu = true;
                                }
                            }
                        });
                    }

                    if close_menu {
                        self.instance_view.context_menu = TableContextMenu::None;
                        ui.memory_mut(|mem| mem.close_popup());
                    }
                }
                TableContextMenu::CellMenu(_pos, instance_index, predictate) => {
                    let mut close_menu = false;
                    let node = node_data.get_node_by_index(instance_index);
                    if let Some((_node_iri, node)) = node {
                        for (predicate_index, value) in &node.properties {
                            if predictate == *predicate_index {
                                ui.label(value.as_ref());
                            }
                        }
                        let button_text = egui::RichText::new(concatcp!(ICON_CLOSE," Close")).size(16.0);
                        let nav_but = egui::Button::new(button_text).fill(egui::Color32::LIGHT_GREEN);
                        let b_resp = ui.add(nav_but);
                        if b_resp.clicked() {
                            close_menu = true;
                        }
                    } else {
                        close_menu = true;
                    }

                    if close_menu {
                        self.instance_view.context_menu = TableContextMenu::None;
                        ui.memory_mut(|mem| mem.close_popup());
                    }
                }
                TableContextMenu::RefMenu(_pos, instance_index) => {
                    let mut close_menu = false;
                    let node = node_data.get_node_by_index(instance_index);
                    if let Some((_node_iri, node)) = node {
                        let mut node_to_click: Option<IriIndex> = None;
                        if let Some(node_index) = show_references(
                            node_data,
                            rdfwrap,
                            color_cache,
                            ui,
                            "References",
                            &node.references,
                            layout_data,
                            300.0,
                            "ref",
                        ) {
                            node_to_click = Some(node_index);
                            close_menu = true;
                        }
                        ui.push_id("refby", |ui| {
                            if let Some(node_index) = show_references(
                                node_data,
                                rdfwrap,
                                color_cache,
                                ui,
                                "Referenced by",
                                &node.reverse_references,
                                layout_data,
                                300.0,
                                "ref_by",
                            ) {
                                node_to_click = Some(node_index);
                                close_menu = true;
                            }
                        });
                        if let Some(node_to_click) = node_to_click {
                            *instance_action = NodeAction::BrowseNode(node_to_click);
                        }
                        let button_text = egui::RichText::new(concatcp!(ICON_CLOSE," Close")).size(16.0);
                        let nav_but = egui::Button::new(button_text).fill(egui::Color32::LIGHT_GREEN);
                        let b_resp = ui.add(nav_but);
                        if b_resp.clicked() {

                            close_menu = true;
                        }
                    } else {
                        close_menu = true;
                    }
                    if close_menu {
                        self.instance_view.context_menu = TableContextMenu::None;
                        ui.memory_mut(|mem| mem.close_popup());
                    }
                }
                TableContextMenu::None => {}
            },
        );
    }
}

fn text_wrapped(text: &str, width: f32, painter: &egui::Painter, top_left: Pos2, cell_hovered: bool) {
    let mut job = egui::text::LayoutJob::default();
    job.append(
        text,
        0.0,
        egui::TextFormat {
            font_id: egui::FontId::default(),
            color: if cell_hovered { Color32::DARK_BLUE } else { Color32::BLACK },
            ..Default::default()
        },
    );

    job.wrap = egui::text::TextWrapping {
        max_width: width,
        max_rows: 1,
        // overflow_character: Some('…'),
        ..Default::default()
    };
    let galley = painter.layout_job(job);
    painter.galley(top_left, galley, Color32::BLACK);
}

fn text_wrapped_link(text: &str, width: f32, painter: &egui::Painter, top_left: Pos2, hovered: bool) {
    let mut job = egui::text::LayoutJob::default();
    job.append(
        text,
        0.0,
        egui::TextFormat {
            font_id: egui::FontId::default(),
            color: Color32::BLUE,
            underline: if hovered {
                Stroke::new(1.0, Color32::BLUE)
            } else {
                Stroke::NONE
            },
            ..Default::default()
        },
    );

    job.wrap = egui::text::TextWrapping {
        max_width: width,
        max_rows: 1,
        // overflow_character: Some('…'),
        ..Default::default()
    };
    let galley = painter.layout_job(job);
    painter.galley(top_left, galley, Color32::BLACK);
}

impl CacheStatistics {
    pub fn new() -> Self {
        Self {
            nodes: 0,
            unique_predicates: 0,
            unique_types: 0,
            properties: 0,
            references: 0,
            blank_nodes: 0,
            unresolved_references: 0,
            types: HashMap::new(),
            types_order: Vec::new(),
            selected_type: None,
        }
    }

    fn reset(&mut self) {
        self.nodes = 0;
        self.unique_predicates = 0;
        self.unique_types = 0;
        self.properties = 0;
        self.references = 0;
        self.blank_nodes = 0;
        self.unresolved_references = 0;
        self.types.clear();
        self.types_order.clear();
    }

    pub fn update(&mut self, node_data: &NodeData) {
        self.reset();
        let start = Instant::now();
        let node_len = node_data.len();
        for (node_index, (node_iri, node)) in node_data.iter().enumerate() {
            if node.has_subject {
                self.nodes += 1;
            } else {
                self.unresolved_references += 1;
            }
            if node.is_blank_node {
                self.blank_nodes += 1;
            }
            for type_index in &node.types {
                let type_data = self
                    .types
                    .entry(*type_index)
                    .or_insert_with(|| TypeData::new(*type_index));
                type_data.instances.push(node_index);
                for (predicate_index, value) in &node.properties {
                    type_data.count_property(*predicate_index, 1);
                    type_data.max_len_property(*predicate_index, value.as_ref().len() as u32);
                }
                for (predicate_index, _) in &node.references {
                    type_data.count_reverence(*predicate_index, 1);
                }
                for (predicate_index, _) in &node.reverse_references {
                    type_data.count_rev_reference(*predicate_index, 1);
                }
            }
            self.references += node.references.len();
            self.properties += node.properties.len();
        }
        self.unique_predicates = node_data.unique_predicates();
        self.unique_types = node_data.unique_types();
        for (type_index, type_data) in self.types.iter_mut() {
            self.types_order.push(*type_index);
            for (predicate_index, data_characteristics) in type_data.properties.iter() {
                if type_data
                    .instance_view
                    .get_column(*predicate_index)
                    .is_none()
                {
                    let predicate_str = node_data.get_predicate(*predicate_index);
                    let column_desc = ColumnDesc {
                        predicate_index: *predicate_index,
                        width: (((data_characteristics.max_len + 1).max(3) as f32) * CHAR_WIDTH)
                            .min(DEFAULT_COLUMN_WIDTH),
                        visible: true,
                    };
                    if let Some(predicate_str) = predicate_str {
                        if predicate_str.contains("label") {
                            type_data
                                .instance_view
                                .display_properties
                                .insert(0, column_desc);
                            continue;
                        }
                    }
                    type_data.instance_view.display_properties.push(column_desc);
                }
            }
            type_data.filtered_instances = type_data.instances.clone();
        }
        self.selected_type = None;
        self.types_order.sort_by(|a, b| {
            let a_data = self.types.get(a).unwrap();
            let b_data = self.types.get(b).unwrap();
            b_data.instances.len().cmp(&a_data.instances.len())
        });
        let duration = start.elapsed();
        println!("Time taken to index {} nodes: {:?}", node_len, duration);
        println!(
            "Nodes per second: {}",
            node_len as f64 / duration.as_secs_f64()
        );
    }

    pub fn display(
        &mut self,
        _ctx: &egui::Context,
        ui: &mut egui::Ui,
        node_data: &mut NodeData,
        layout_data: &mut LayoutData,
        prefix_manager: &PrefixManager,
        color_cache: &ColorCache,
        rdfwrap: &mut dyn rdfwrap::RDFAdapter,
        iri_display: IriDisplay,
    ) -> NodeAction {
        let mut instance_action = NodeAction::None;
        egui::ScrollArea::horizontal().id_salt("h").show(ui, |ui| {
            ui.with_layout(Layout::left_to_right(Align::TOP), |ui| {
                ui.vertical(|ui| {
                    ui.heading("Statistics:");
                    ui.label(format!("Nodes: {}", self.nodes));
                    ui.label(format!(
                        "Unresolved References: {}",
                        self.unresolved_references
                    ));
                    ui.label(format!("Blank Nodes: {}", self.blank_nodes));
                    ui.label(format!("Properties: {}", self.properties));
                    ui.label(format!("References: {}", self.references));
                    ui.label(format!("Unique Predicates: {}", self.unique_predicates));
                    ui.label(format!("Unique Types: {}", self.unique_types));
                    ui.label(format!(
                        "Unique Languages: {}",
                        node_data.unique_languages()
                    ));
                    ui.label(format!(
                        "Unique Data Types: {}",
                        node_data.unique_data_types()
                    ));
                    /*
                    ui.horizontal(|ui| {
                        if ui.button("Update").clicked() {
                            self.update(node_data);
                        }
                        if ui.button("Clean").clicked() {
                            node_data.clean();
                            layout_data.visible_nodes.data.clear();
                        }
                    });
                    */
                });
                ui.with_layout(Layout::top_down(Align::LEFT), |ui| {
                    ui.push_id("types", |ui| {
                        let (selected_type, type_table_action) = self.show_types(
                            ui,
                            node_data,
                            prefix_manager,
                            layout_data,
                            iri_display,
                            200.0,
                        );
                        if selected_type.is_some() {
                            self.selected_type = selected_type;
                        }
                        match type_table_action {
                            TypeTableAction::SortByLabel => {
                                self.types_order.sort_by(|a, b| {
                                    let label_a = node_data.type_display(
                                        *a,
                                        layout_data.display_language,
                                        iri_display,
                                        prefix_manager,
                                    );
                                    let label_b = node_data.type_display(
                                        *b,
                                        layout_data.display_language,
                                        iri_display,
                                        prefix_manager,
                                    );
                                    return label_a.as_str().cmp(label_b.as_str());
                                });
                            },
                            TypeTableAction::SortByInstances => {
                                self.types_order.sort_by(|a, b| {
                                    let a_data = self.types.get(a).unwrap();
                                    let b_data = self.types.get(b).unwrap();
                                    b_data.instances.len().cmp(&a_data.instances.len())
                                });
                            },
                            TypeTableAction::SortByDataProps => {
                                self.types_order.sort_by(|a, b| {
                                    let a_data = self.types.get(a).unwrap();
                                    let b_data = self.types.get(b).unwrap();
                                    b_data.properties.len().cmp(&a_data.properties.len())
                                });
                            },
                            TypeTableAction::SortByOutRef => {
                                self.types_order.sort_by(|a, b| {
                                    let a_data = self.types.get(a).unwrap();
                                    let b_data = self.types.get(b).unwrap();
                                    b_data.references.len().cmp(&a_data.references.len())
                                });
                            },
                            TypeTableAction::SortByInRef => {
                                self.types_order.sort_by(|a, b| {
                                    let a_data = self.types.get(a).unwrap();
                                    let b_data = self.types.get(b).unwrap();
                                    b_data.rev_references.len().cmp(&a_data.rev_references.len())
                                });
                            }
                            TypeTableAction::None => {

                            }
                        }
                    });
                });
                if let Some(selected_type) = self.selected_type {
                    if let Some(type_data) = self.types.get_mut(&selected_type) {
                        ui.allocate_ui(Vec2::new(ui.available_width(), 200.0), |ui| {
                            ui.separator();
                            egui::ScrollArea::vertical().id_salt("data").show(ui, |ui| {
                                egui::Grid::new("fields").show(ui, |ui| {
                                    ui.strong("Data Property");
                                    ui.strong("Count");
                                    ui.strong("Max Len");
                                    ui.end_row();
                                    for (predicate_index, pcharecteristics) in &type_data.properties
                                    {
                                        let predicate_label = node_data.predicate_display(
                                            *predicate_index,
                                            layout_data.display_language,
                                            iri_display,
                                            prefix_manager,
                                        );
                                        ui.label(predicate_label.as_str());
                                        ui.label(pcharecteristics.count.to_string());
                                        ui.label(pcharecteristics.max_len.to_string());
                                        ui.end_row();
                                    }
                                });
                            });
                        });
                        ui.allocate_ui(Vec2::new(ui.available_width(), 200.0), |ui| {
                            ui.separator();
                            egui::ScrollArea::vertical().id_salt("ref").show(ui, |ui| {
                                egui::Grid::new("referenced").show(ui, |ui| {
                                    ui.strong("Out Ref");
                                    ui.strong("Count");
                                    ui.end_row();
                                    for (predicate_index, count) in &type_data.references {
                                        let predicate_label = node_data.predicate_display(
                                            *predicate_index,
                                            layout_data.display_language,
                                            iri_display,
                                            prefix_manager,
                                        );
                                        ui.label(predicate_label.as_str());
                                        ui.label(count.to_string());
                                        ui.end_row();
                                    }
                                });
                            });
                        });
                        ui.allocate_ui(Vec2::new(ui.available_width(), 200.0), |ui| {
                            ui.separator();
                            egui::ScrollArea::vertical()
                                .id_salt("refby")
                                .show(ui, |ui| {
                                    egui::Grid::new("referenced by").show(ui, |ui| {
                                        ui.strong("In Ref");
                                        ui.strong("Count");
                                        ui.end_row();
                                        for (predicate_index, count) in &type_data.rev_references {
                                            let predicate_label = node_data.predicate_display(
                                                *predicate_index,
                                                layout_data.display_language,
                                                iri_display,
                                                prefix_manager,
                                            );
                                            ui.label(predicate_label.as_str());
                                            ui.label(count.to_string());
                                            ui.end_row();
                                        }
                                    });
                                });
                        });
                    }
                }
            });
        });
        ui.separator();
        if let Some(selected_type) = self.selected_type {
            if let Some(type_data) = self.types.get_mut(&selected_type) {
                let mut table_action: TableAction = TableAction::None;
                ui.horizontal(|ui| {
                    let text_edit =
                        egui::TextEdit::singleline(&mut type_data.instance_view.instance_filter);
                    let text_edit_response = ui.add(text_edit);
                    if ui
                        .ctx()
                        .input(|i| i.key_pressed(egui::Key::F) && i.modifiers.command)
                    {
                        text_edit_response.request_focus();
                    }
                    if text_edit_response.lost_focus() {
                        table_action = TableAction::Filter;
                    }
                    if ui.button("Filter").clicked() {
                        table_action = TableAction::Filter;
                    }
                    if ui.button("Reset filter").clicked() {
                        type_data.instance_view.instance_filter.clear();
                        type_data.filtered_instances = type_data.instances.clone();
                        type_data.instance_view.instance_filter.clear();
                    }
                    ui.label(format!(
                        "{}/{}",
                        type_data.filtered_instances.len(),
                        type_data.instances.len()
                    ));
                    let visible_columns = type_data.instance_view.visible_columns();
                    if visible_columns > 1 {
                        if type_data.instance_view.column_pos > visible_columns - 1 {
                            type_data.instance_view.column_pos = visible_columns - 1;
                        }
                        ui.add(Slider::new(
                            &mut type_data.instance_view.column_pos,
                            0..=visible_columns - 1,
                        ));
                    }
                });
                let needed_len = (type_data.filtered_instances.len() + 2) as f32 * ROW_HIGHT;
                let a_height = ui.available_height();
                StripBuilder::new(ui)
                    .size(egui_extras::Size::remainder())
                    .size(egui_extras::Size::exact(20.0)) // Two resizable panels with equal initial width
                    .horizontal(|mut strip| {
                        strip.cell(|ui| {
                            type_data.instance_table(
                                ui,
                                &mut table_action,
                                &mut instance_action,
                                node_data,
                                color_cache,
                                rdfwrap,
                                prefix_manager,
                                layout_data,
                            );
                        });
                        strip.cell(|ui| {
                            ui.add(ScrollBar::new(
                                &mut type_data.instance_view.pos,
                                &mut type_data.instance_view.drag_pos,
                                needed_len,
                                a_height,
                            ));
                        });
                    });

                match table_action {
                    TableAction::HideColumn(predicate_to_hide) => {
                        if let Some(type_data) = self.types.get_mut(&selected_type) {
                            for column_desc in type_data.instance_view.display_properties.iter_mut()
                            {
                                if column_desc.predicate_index == predicate_to_hide {
                                    column_desc.visible = false;
                                    break;
                                }
                            }
                        }
                    }
                    TableAction::UhideColumn(preducate_to_unhide) => {
                        if let Some(type_data) = self.types.get_mut(&selected_type) {
                            for column_desc in type_data.instance_view.display_properties.iter_mut()
                            {
                                if column_desc.predicate_index == preducate_to_unhide {
                                    column_desc.visible = true;
                                    break;
                                }
                            }
                        }
                    }
                    TableAction::SortColumnAsc(predicate_to_sort) => {
                        if let Some(type_data) = self.types.get_mut(&selected_type) {
                            type_data.filtered_instances.sort_by(|a, b| {
                                let node_a = node_data.get_node_by_index(*a);
                                let node_b = node_data.get_node_by_index(*b);
                                if let Some((_, node_a)) = node_a {
                                    if let Some((_, node_b)) = node_b {
                                        let a_value = &node_a.get_property(
                                            predicate_to_sort,
                                            layout_data.display_language,
                                        );
                                        let b_value = &node_b.get_property(
                                            predicate_to_sort,
                                            layout_data.display_language,
                                        );
                                        a_value.cmp(b_value)
                                    } else {
                                        std::cmp::Ordering::Less
                                    }
                                } else {
                                    std::cmp::Ordering::Greater
                                }
                            });
                        }
                    }
                    TableAction::SortColumnDesc(predicate_to_sort) => {
                        if let Some(type_data) = self.types.get_mut(&selected_type) {
                            type_data.filtered_instances.sort_by(|a, b| {
                                let node_a = node_data.get_node_by_index(*a);
                                let node_b = node_data.get_node_by_index(*b);
                                if let Some((_, node_a)) = node_a {
                                    if let Some((_, node_b)) = node_b {
                                        let a_value = &node_a.get_property(
                                            predicate_to_sort,
                                            layout_data.display_language,
                                        );
                                        let b_value = node_b.get_property(
                                            predicate_to_sort,
                                            layout_data.display_language,
                                        );
                                        b_value.cmp(a_value)
                                    } else {
                                        std::cmp::Ordering::Greater
                                    }
                                } else {
                                    std::cmp::Ordering::Less
                                }
                            });
                        }
                    }
                    TableAction::SortRefAsc() => {
                        if let Some(type_data) = self.types.get_mut(&selected_type) {
                            type_data.filtered_instances.sort_by(|a, b| {
                                let node_a = node_data.get_node_by_index(*a);
                                let node_b = node_data.get_node_by_index(*b);
                                if let Some((_, node_a)) = node_a {
                                    if let Some((_, node_b)) = node_b {
                                        let a_value = &node_a.references.len()
                                            + &node_a.reverse_references.len();
                                        let b_value = node_b.references.len()
                                            + node_b.reverse_references.len();
                                        b_value.cmp(&a_value)
                                    } else {
                                        std::cmp::Ordering::Greater
                                    }
                                } else {
                                    std::cmp::Ordering::Less
                                }
                            });
                        }
                    }
                    TableAction::SortRefDesc() => {
                        if let Some(type_data) = self.types.get_mut(&selected_type) {
                            type_data.filtered_instances.sort_by(|a, b| {
                                let node_a = node_data.get_node_by_index(*a);
                                let node_b = node_data.get_node_by_index(*b);
                                if let Some((_, node_a)) = node_a {
                                    if let Some((_, node_b)) = node_b {
                                        let a_value = &node_a.references.len()
                                            + &node_a.reverse_references.len();
                                        let b_value = node_b.references.len()
                                            + node_b.reverse_references.len();
                                        a_value.cmp(&b_value)
                                    } else {
                                        std::cmp::Ordering::Greater
                                    }
                                } else {
                                    std::cmp::Ordering::Less
                                }
                            });
                        }
                    }
                    TableAction::Filter => {
                        type_data.filtered_instances = type_data
                            .instances
                            .iter()
                            .cloned()
                            .filter(|&instance_index| {
                                let node = node_data.get_node_by_index(instance_index);
                                if let Some((node_iri, node)) = node {
                                    if node.apply_filter(
                                        &type_data.instance_view.instance_filter,
                                        &node_iri,
                                    ) {
                                        return true;
                                    }
                                }
                                return false;
                            })
                            .collect();
                        if (type_data.instance_view.pos / ROW_HIGHT) as usize
                            >= type_data.filtered_instances.len()
                        {
                            type_data.instance_view.pos = 0.0;
                        }
                    }
                    TableAction::None => {}
                }
            }
        } else {
            ui.label("Select a type to display its instances");
        }
        return instance_action;
    }

    fn show_types(
        &self,
        ui: &mut egui::Ui,
        node_data: &mut NodeData,
        prefix_manager: &PrefixManager,
        layout_data: &LayoutData,
        iri_display: IriDisplay,
        height: f32,
    ) -> (Option<IriIndex>,TypeTableAction) {
        let mut selected_type: Option<IriIndex> = None;
        let mut type_table_action: TypeTableAction = TypeTableAction::None;
        let text_height = egui::TextStyle::Body
            .resolve(ui.style())
            .size
            .max(ui.spacing().interact_size.y);

        let table: TableBuilder<'_> = TableBuilder::new(ui)
            .striped(true)
            .resizable(true)
            .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
            .column(Column::exact(200.0).at_least(30.0).at_most(300.0))
            .column(Column::exact(50.0).at_least(30.0).at_most(300.0))
            .column(Column::exact(50.0).at_least(30.0).at_most(300.0))
            .column(Column::exact(50.0).at_least(30.0).at_most(300.0))
            .column(Column::exact(50.0).at_least(30.0).at_most(300.0))
            .min_scrolled_height(height)
            .max_scroll_height(height)
            .sense(Sense::click());

        table
            .header(20.0, |mut header| {
                header.col(|ui| {
                    strong_unselectable(ui,"Type");
                    if ui.response().hovered() {
                        ui.output_mut(|o| o.cursor_icon = CursorIcon::PointingHand);
                    }
                    if ui.response().clicked() {
                        type_table_action = TypeTableAction::SortByLabel;
                    }
                });
                header.col(|ui| {
                    strong_unselectable(ui,"Inst#");
                    if ui.response().hovered() {
                        ui.output_mut(|o| o.cursor_icon = CursorIcon::PointingHand);
                    }
                    if ui.response().clicked() {
                        type_table_action = TypeTableAction::SortByInstances;
                    }
                });
                header.col(|ui| {
                    strong_unselectable(ui,"Data#");
                    if ui.response().hovered() {
                        ui.output_mut(|o| o.cursor_icon = CursorIcon::PointingHand);
                    }
                    if ui.response().clicked() {
                        type_table_action = TypeTableAction::SortByDataProps;
                    }
                });
                header.col(|ui| {
                    strong_unselectable(ui,"Out Ref#");
                    if ui.response().hovered() {
                        ui.output_mut(|o| o.cursor_icon = CursorIcon::PointingHand);
                    }
                    if ui.response().clicked() {
                        type_table_action = TypeTableAction::SortByOutRef;
                    }
                });
                header.col(|ui| {
                    strong_unselectable(ui,"In Ref#");
                    if ui.response().hovered() {
                        ui.output_mut(|o| o.cursor_icon = CursorIcon::PointingHand);
                    }
                    if ui.response().clicked() {
                        type_table_action = TypeTableAction::SortByInRef;
                    }
                });
            })
            .body(|body| {
                body.rows(text_height, self.types_order.len(), |mut row| {
                    let type_index = self.types_order.get(row.index()).unwrap();
                    row.set_selected(self.selected_type == Some(*type_index));
                    let type_data = self.types.get(type_index).unwrap();
                    let type_label = node_data.type_display(
                        *type_index,
                        layout_data.display_language,
                        iri_display,
                        prefix_manager,
                    );
                    row.col(|ui| {
                        ui.add(egui::Label::new(type_label.as_str()).selectable(false));
                    });
                    row.col(|ui| {
                        ui.label(type_data.instances.len().to_string());
                    });
                    row.col(|ui| {
                        ui.label(type_data.properties.len().to_string());
                    });
                    row.col(|ui| {
                        ui.label(type_data.references.len().to_string());
                    });
                    row.col(|ui| {
                        ui.label(type_data.rev_references.len().to_string());
                    });
                    if row.response().clicked() {
                        selected_type = Some(*type_index);
                    }
                });
            });
        return (selected_type, type_table_action);
    }
}

pub enum TableAction {
    None,
    HideColumn(IriIndex),
    UhideColumn(IriIndex),
    SortColumnAsc(IriIndex),
    SortColumnDesc(IriIndex),
    SortRefAsc(),
    SortRefDesc(),
    Filter,
}

enum TypeTableAction {
    None,
    SortByLabel,
    SortByInstances,
    SortByDataProps,
    SortByOutRef,
    SortByInRef,
}
