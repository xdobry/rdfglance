use std::{cmp::min, collections::HashMap, time::Instant, vec};

use const_format::concatcp;
use egui::{Align, Align2, Color32, CursorIcon, Layout, Pos2, Rect, Sense, Slider, Stroke, Vec2};
use egui_extras::{Column, StripBuilder, TableBuilder};
use rayon::prelude::*;

const IMMADIATE_FILTER_COUNT: usize = 20000;

use crate::{
    browse_view::{show_references, ReferenceAction}, config::IriDisplay, nobject::{IriIndex, LabelContext, NodeData}, prefix_manager::PrefixManager, style::{ICON_CLOSE, ICON_FILTER, ICON_GRAPH}, uitools::{popup_at, primary_color, strong_unselectable, ScrollBar}, GVisualisationStyle, NodeAction, RdfData, UIState
};

pub struct TypeInstanceIndex {
    pub nodes: usize,
    pub unique_predicates: usize,
    pub unique_types: usize,
    pub properties: usize,
    pub references: usize,
    pub blank_nodes: usize,
    pub max_instance_type_count: usize,
    pub min_instance_type_count: usize,
    pub unresolved_references: usize,
    pub types: HashMap<IriIndex, TypeData>,
    pub types_order: Vec<IriIndex>,
    pub types_filtered: Vec<IriIndex>,
    pub selected_type: Option<IriIndex>,
    pub types_filter: String,
    pub type_cell_action: TypeCellAction,
}

pub enum TypeCellAction {
    None,
    ShowRefTypes(Pos2, IriIndex),
}

impl TypeCellAction {
    pub fn pos(&self) -> Pos2 {
        match self {
            TypeCellAction::ShowRefTypes(pos, _) => *pos,
            TypeCellAction::None => Pos2::new(0.0, 0.0),
        }
    }
}

pub struct DataPropCharacteristics {
    pub count: u32,
    pub max_len: u32,
    pub max_cardinality: u32,
    pub min_cardinality: u32,
}

pub struct ReferenceCharacteristics {
    pub count: u32,
    pub max_cardinality: u32,
    pub min_cardinality: u32,
    pub types: Vec<IriIndex>,
}

pub struct TypeData {
    pub instances: Vec<IriIndex>,
    pub filtered_instances: Vec<IriIndex>,
    pub properties: HashMap<IriIndex, DataPropCharacteristics>,
    pub references: HashMap<IriIndex, ReferenceCharacteristics>,
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
    pub column_resize: InstanceColumnResize,
    pub iri_width: f32,
    pub ref_count_width: f32,
}

pub enum InstanceColumnResize {
    None,
    Predicate(Pos2, IriIndex),
    Iri(Pos2),
    Refs(Pos2),
}

enum TableContextMenu {
    None,
    ColumnMenu(Pos2, IriIndex),
    CellMenu(Pos2, IriIndex, IriIndex),
    RefMenu(Pos2, IriIndex),
    IriColomnMenu(Pos2),
    RefColumnMenu(Pos2),
}

impl TableContextMenu {
    pub fn pos(&self) -> Pos2 {
        match self {
            TableContextMenu::ColumnMenu(pos, _) => *pos,
            TableContextMenu::CellMenu(pos, _, _) => *pos,
            TableContextMenu::RefMenu(pos, _) => *pos,
            TableContextMenu::RefColumnMenu(pos) => *pos,
            TableContextMenu::IriColomnMenu(pos) => *pos,
            TableContextMenu::None => Pos2::new(0.0, 0.0),
        }
    }
}

impl InstanceView {
    pub fn get_column(&self, predicate_index: IriIndex) -> Option<&ColumnDesc> {
        self.display_properties
            .iter()
            .find(|column_desc| column_desc.predicate_index == predicate_index)
    }
    pub fn visible_columns(&self) -> u32 {
        let mut count = 0;
        for column_desc in &self.display_properties {
            if column_desc.visible {
                count += 1;
            }
        }
        count
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
const IRI_WIDTH: f32 = 300.0;
const REF_COUNT_WIDTH: f32 = 80.0;

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
                column_resize: InstanceColumnResize::None,
                iri_width: IRI_WIDTH,
                ref_count_width: REF_COUNT_WIDTH,
            },
        }
    }

    pub fn count_rev_reference(&mut self, reference_index: IriIndex, count_number: u32) {
        let count = self.rev_references.entry(reference_index).or_insert(0);
        *count += count_number;
    }

    pub fn instance_table(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        table_action: &mut TableAction,
        instance_action: &mut NodeAction,
        node_data: &mut NodeData,
        color_cache: &GVisualisationStyle,
        prefix_manager: &PrefixManager,
        layout_data: &UIState,
        iri_display: IriDisplay,
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

        let font_id = egui::FontId::default();
        let popup_id = ui.make_persistent_id("column_context_menu");

        painter.rect_filled(
            Rect::from_min_size(available_rect.left_top(), Vec2::new(available_width, ROW_HIGHT)),
            0.0,
            ui.visuals().code_bg_color,
        );

        if response.drag_stopped() {
            self.instance_view.column_resize = InstanceColumnResize::None;
        }
        if response.dragged() {
            match self.instance_view.column_resize {
                InstanceColumnResize::None => {}
                InstanceColumnResize::Predicate(start_pos, predicate_index) => {
                    ui.output_mut(|o| o.cursor_icon = CursorIcon::ResizeHorizontal);
                    for column_desc in self.instance_view.display_properties.iter_mut() {
                        if column_desc.predicate_index == predicate_index {
                            let width = mouse_pos.x - start_pos.x;
                            if width > CHAR_WIDTH * 2.0 {
                                column_desc.width = width;
                            }
                        }
                    }
                }
                InstanceColumnResize::Iri(start_pos) => {
                    ui.output_mut(|o| o.cursor_icon = CursorIcon::ResizeHorizontal);
                    let width = mouse_pos.x - start_pos.x;
                    if width > CHAR_WIDTH * 3.0 {
                        self.instance_view.iri_width = width;
                    }
                }
                InstanceColumnResize::Refs(start_pos) => {
                    ui.output_mut(|o| o.cursor_icon = CursorIcon::ResizeHorizontal);
                    let width = mouse_pos.x - start_pos.x;
                    if width > CHAR_WIDTH * 5.0 {
                        self.instance_view.ref_count_width = width;
                    }
                }
            }
        }

        painter.text(
            available_rect.left_top(),
            egui::Align2::LEFT_TOP,
            "iri",
            font_id.clone(),
            ui.visuals().strong_text_color(),
        );

        let iri_colums_drag_size_rect = egui::Rect::from_min_size(
            available_rect.left_top() + Vec2::new(self.instance_view.iri_width - 3.0, 0.0),
            Vec2::new(6.0, ROW_HIGHT),
        );

        let mut primary_down = false;
        ctx.input(|input| {
            if input.pointer.button_pressed(egui::PointerButton::Primary) {
                primary_down = true;
            }
        });

        let mut was_context_click = false;

        if iri_colums_drag_size_rect.contains(mouse_pos) {
            ui.output_mut(|o| o.cursor_icon = CursorIcon::ResizeHorizontal);
            if primary_down && matches!(self.instance_view.column_resize, InstanceColumnResize::None) {
                self.instance_view.column_resize =
                    InstanceColumnResize::Iri(mouse_pos - Vec2::new(self.instance_view.iri_width, 0.0));
            }
        }

        let iri_column_rec = egui::Rect::from_min_size(
            available_rect.left_top(),
            Vec2::new(self.instance_view.iri_width, ROW_HIGHT),
        );
        if secondary_clicked && iri_column_rec.contains(mouse_pos) {
            was_context_click = true;
            ui.memory_mut(|mem| mem.toggle_popup(popup_id));
            self.instance_view.context_menu = TableContextMenu::IriColomnMenu(mouse_pos);
        }
        painter.text(
            available_rect.left_top() + Vec2::new(self.instance_view.iri_width, 0.0),
            egui::Align2::LEFT_TOP,
            "out/in",
            font_id.clone(),
            ui.visuals().strong_text_color(),
        );
        let ref_column_rec = egui::Rect::from_min_size(
            available_rect.left_top() + Vec2::new(self.instance_view.iri_width, 0.0),
            Vec2::new(self.instance_view.ref_count_width, ROW_HIGHT),
        );
        let refs_colums_drag_size_rect = egui::Rect::from_min_size(
            available_rect.left_top()
                + Vec2::new(
                    self.instance_view.iri_width + self.instance_view.ref_count_width - 3.0,
                    0.0,
                ),
            Vec2::new(6.0, ROW_HIGHT),
        );
        if refs_colums_drag_size_rect.contains(mouse_pos) {
            ui.output_mut(|o| o.cursor_icon = CursorIcon::ResizeHorizontal);
            if primary_down && matches!(self.instance_view.column_resize, InstanceColumnResize::None) {
                self.instance_view.column_resize =
                    InstanceColumnResize::Refs(mouse_pos - Vec2::new(self.instance_view.ref_count_width, 0.0));
            }
        }
        if ref_column_rec.contains(mouse_pos) && secondary_clicked {
            was_context_click = true;
            ui.memory_mut(|mem| mem.toggle_popup(popup_id));
            self.instance_view.context_menu = TableContextMenu::RefColumnMenu(mouse_pos);
        }
        xpos += self.instance_view.iri_width + self.instance_view.ref_count_width;

        let label_context = LabelContext::new(layout_data.display_language, iri_display, prefix_manager);
        for column_desc in self
            .instance_view
            .display_properties
            .iter()
            .filter(|p| p.visible)
            .skip(self.instance_view.column_pos as usize)
        {
            let top_left = available_rect.left_top() + Vec2::new(xpos, 0.0);
            let predicate_label =
                node_data.predicate_display(column_desc.predicate_index, &label_context, &node_data.indexers);
            text_wrapped(predicate_label.as_str(), column_desc.width, painter, top_left, false, true, ui.visuals());
            xpos += column_desc.width + COLUMN_GAP;
            let column_rect = egui::Rect::from_min_size(top_left, Vec2::new(column_desc.width, ROW_HIGHT));
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
                if primary_down && matches!(self.instance_view.column_resize, InstanceColumnResize::None) {
                    self.instance_view.column_resize = InstanceColumnResize::Predicate(
                        mouse_pos - Vec2::new(column_desc.width, 0.0),
                        column_desc.predicate_index,
                    );
                }
            }
        }

        let mut ypos = ROW_HIGHT;
        let mut start_pos = instance_index;

        for instance_index in
            &self.filtered_instances[instance_index..min(instance_index + capacity, self.filtered_instances.len())]
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
                        ui.visuals().faint_bg_color,
                    );
                }
                start_pos += 1;
                let mut xpos = self.instance_view.iri_width + self.instance_view.ref_count_width;

                let graph_button_width = 20.0;
                let graph_pos = available_rect.left_top() + Vec2::new(0.0, ypos + 1.0);
                let button_rect = Rect::from_min_size(graph_pos, Vec2::new(graph_button_width, ROW_HIGHT - 2.0));
                let button_background = if button_rect.contains(mouse_pos) {
                    if primary_clicked {
                        *instance_action = NodeAction::ShowVisual(*instance_index);
                    }
                    Color32::YELLOW
                } else {
                    Color32::LIGHT_YELLOW
                };

                painter.rect_filled(button_rect, 3.0, button_background);
                painter.text(
                    graph_pos + Vec2::new(graph_button_width / 2.0, (ROW_HIGHT - 2.0) / 2.0),
                    Align2::CENTER_CENTER,
                    ICON_GRAPH,
                    egui::FontId::default(),
                    ui.visuals().text_color(),
                );

                let iri_top_left = available_rect.left_top() + Vec2::new(graph_button_width, ypos);

                let cell_rect = egui::Rect::from_min_size(
                    iri_top_left,
                    Vec2::new(self.instance_view.iri_width - graph_button_width, ROW_HIGHT),
                );

                let mut cell_hovered = false;
                if cell_rect.contains(mouse_pos) {
                    ui.output_mut(|o| o.cursor_icon = CursorIcon::PointingHand);
                    cell_hovered = true;
                }

                text_wrapped_link(
                    &prefix_manager.get_prefixed(node_iri),
                    self.instance_view.iri_width - graph_button_width,
                    painter,
                    iri_top_left,
                    cell_hovered,
                    ui.visuals()
                );

                if primary_clicked && cell_rect.contains(mouse_pos) {
                    *instance_action = NodeAction::BrowseNode(*instance_index);
                } else if secondary_clicked && cell_rect.contains(mouse_pos) {
                    *instance_action = NodeAction::ShowVisual(*instance_index);
                }
                let s = format!("{}/{}", node.references.len(), node.reverse_references.len());
                let ref_rect = egui::Rect::from_min_size(
                    available_rect.left_top() + Vec2::new(self.instance_view.iri_width, ypos),
                    Vec2::new(self.instance_view.ref_count_width, ROW_HIGHT),
                );
                painter.text(
                    ref_rect.left_top(),
                    egui::Align2::LEFT_TOP,
                    s,
                    font_id.clone(),
                    if ref_rect.contains(mouse_pos) {
                        ui.visuals().selection.stroke.color
                    } else {
                        ui.visuals().text_color()
                    },
                );
                if primary_clicked && ref_rect.contains(mouse_pos) {
                    was_context_click = true;
                    ui.memory_mut(|mem| mem.toggle_popup(popup_id));
                    self.instance_view.context_menu = TableContextMenu::RefMenu(mouse_pos, *instance_index);
                }

                for column_desc in self
                    .instance_view
                    .display_properties
                    .iter()
                    .filter(|p| p.visible)
                    .skip(self.instance_view.column_pos as usize)
                {
                    let property = node.get_property_count(column_desc.predicate_index, layout_data.display_language);
                    if let Some((property, count)) = property {
                        let value = property.as_str_ref(&node_data.indexers);
                        let cell_rect = egui::Rect::from_min_size(
                            available_rect.left_top() + Vec2::new(xpos, ypos),
                            Vec2::new(column_desc.width, ROW_HIGHT),
                        );
                        let mut cell_hovered = false;
                        if cell_rect.contains(mouse_pos) {
                            cell_hovered = true;
                        }
                        if count > 1 {
                            painter.rect_filled(cell_rect, 0.0, ui.visuals().code_bg_color);
                        }
                        text_wrapped(value, column_desc.width, painter, cell_rect.left_top(), cell_hovered, false, ui.visuals());
                        if primary_clicked && cell_rect.contains(mouse_pos) {
                            was_context_click = true;
                            ui.memory_mut(|mem| mem.toggle_popup(popup_id));
                            self.instance_view.context_menu =
                                TableContextMenu::CellMenu(mouse_pos, *instance_index, column_desc.predicate_index);
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
                    available_rect.left() + self.instance_view.iri_width - COLUMN_GAP,
                    available_rect.top(),
                ),
                Pos2::new(
                    available_rect.left() + self.instance_view.iri_width - COLUMN_GAP,
                    available_rect.top() + ypos,
                ),
            ]
            .to_vec(),
            Stroke::new(1.0, Color32::DARK_GRAY),
        );
        painter.line(
            [
                Pos2::new(
                    available_rect.left()
                        + self.instance_view.iri_width
                        + self.instance_view.ref_count_width
                        + -COLUMN_GAP,
                    available_rect.top(),
                ),
                Pos2::new(
                    available_rect.left() + self.instance_view.ref_count_width + self.instance_view.iri_width
                        - COLUMN_GAP,
                    available_rect.top() + ypos,
                ),
            ]
            .to_vec(),
            Stroke::new(1.0, Color32::DARK_GRAY),
        );
        xpos = self.instance_view.iri_width + self.instance_view.ref_count_width;
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
                TableContextMenu::IriColomnMenu(_pos) => {
                    let mut close_menu: bool = false;
                    if ui.button("Sort Asc").clicked() {
                        *table_action = TableAction::SortIriAsc();
                        close_menu = true;
                    }
                    if ui.button("Sort Desc").clicked() {
                        *table_action = TableAction::SortIriDesc();
                        close_menu = true;
                    }
                    if close_menu {
                        self.instance_view.context_menu = TableContextMenu::None;
                        ui.memory_mut(|mem| mem.close_popup());
                    }
                }
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
                TableContextMenu::ColumnMenu(_pos, column_predictate) => {
                    let mut close_menu = false;
                    if self.instance_view.visible_columns() > 0 && ui.button("Hide column").clicked() {
                        *table_action = TableAction::HideColumn(column_predictate);
                        close_menu = true;
                    }
                    if ui.button("Sort Asc").clicked() {
                        *table_action = TableAction::SortColumnAsc(column_predictate);
                        close_menu = true;
                    }
                    if ui.button("Sort Desc").clicked() {
                        *table_action = TableAction::SortColumnDesc(column_predictate);
                        close_menu = true;
                    }
                    if ui.button("Show Only Value Exists").clicked() {
                        *table_action = TableAction::HidePropExists(column_predictate);
                        close_menu = true;
                    }
                    if ui.button("Show Only Value Not Exists").clicked() {
                        *table_action = TableAction::HidePropNotExists(column_predictate);
                        close_menu = true;
                    }
                    if ui.button("Show Only Mutivalue").clicked() {
                        *table_action = TableAction::HidePropNonMulti(column_predictate);
                        close_menu = true;
                    }
                    let hidden_columns: Vec<&ColumnDesc> = self
                        .instance_view
                        .display_properties
                        .iter()
                        .filter(|p| !p.visible)
                        .collect();
                    if !hidden_columns.is_empty() {
                        ui.separator();
                        ui.menu_button("Unhide Columns", |ui| {
                            for column_desc in hidden_columns {
                                let predicate_label = node_data.predicate_display(
                                    column_desc.predicate_index,
                                    &label_context,
                                    &node_data.indexers,
                                );
                                if ui.button(predicate_label.as_str()).clicked() {
                                    *table_action = TableAction::UhideColumn(column_desc.predicate_index);
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
                                ui.label(value.as_str_ref(&node_data.indexers));
                            }
                        }
                        let button_text = egui::RichText::new(concatcp!(ICON_CLOSE, " Close")).size(16.0);
                        let nav_but = egui::Button::new(button_text).fill(primary_color(ui.visuals()));
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
                        let mut node_to_click: ReferenceAction = ReferenceAction::None;
                        let label_context =
                            LabelContext::new(layout_data.display_language, iri_display, prefix_manager);
                        let ref_result = show_references(
                            node_data,
                            color_cache,
                            ui,
                            "References",
                            &node.references,
                            layout_data,
                            300.0,
                            "ref",
                            &label_context,
                        );
                        if ref_result != ReferenceAction::None {
                            node_to_click = ref_result;
                            close_menu = true;
                        }
                        ui.push_id("refby", |ui| {
                            let ref_result = show_references(
                                node_data,
                                color_cache,
                                ui,
                                "Referenced by",
                                &node.reverse_references,
                                layout_data,
                                300.0,
                                "ref_by",
                                &label_context,
                            );
                            if ref_result != ReferenceAction::None {
                                node_to_click = ref_result;
                                close_menu = true;
                            }
                        });
                        match node_to_click {
                            ReferenceAction::None => {},
                            ReferenceAction::ShowNode(node_index) => {
                                *instance_action = NodeAction::BrowseNode(node_index);
                            },
                            ReferenceAction::Filter(type_index,instances ) => {
                                *instance_action = NodeAction::ShowTypeInstances(type_index, instances)
                            }
                        }
                        let button_text = egui::RichText::new(concatcp!(ICON_CLOSE, " Close")).size(16.0);
                        let nav_but = egui::Button::new(button_text).fill(primary_color(ui.visuals()));
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

    pub fn display_data_props(&self, ui: &mut egui::Ui, label_context: &LabelContext, node_data: &NodeData) {
        egui::Grid::new("fields").show(ui, |ui| {
            ui.strong("Data Property");
            ui.strong("Count");
            ui.strong("Max Len");
            ui.strong("Min Card.");
            ui.strong("Max Card.");
            ui.end_row();
            for (predicate_index, pcharecteristics) in &self.properties {
                let predicate_label = node_data.predicate_display(*predicate_index, label_context, &node_data.indexers);
                ui.label(predicate_label.as_str());
                ui.label(pcharecteristics.count.to_string());
                ui.label(pcharecteristics.max_len.to_string());
                ui.label(pcharecteristics.min_cardinality.to_string());
                ui.label(pcharecteristics.max_cardinality.to_string());
                ui.end_row();
            }
        });
    }

    pub fn display_references(
        &self,
        ui: &mut egui::Ui,
        label_context: &LabelContext,
        node_data: &NodeData,
    ) -> TypeCellAction {
        let mut type_cell_action: TypeCellAction = TypeCellAction::None;
        egui::Grid::new("referenced").show(ui, |ui| {
            ui.strong("Out Ref");
            ui.strong("Count");
            ui.strong("Type");
            ui.strong("Min Card.");
            ui.strong("Max Card.");
            ui.end_row();
            for (predicate_index, reference_characteristics) in &self.references {
                let predicate_label = node_data.predicate_display(*predicate_index, label_context, &node_data.indexers);
                ui.label(predicate_label.as_str());
                ui.label(reference_characteristics.count.to_string());
                if reference_characteristics.types.is_empty() {
                    ui.label("<None>");
                } else {
                    let first_type = reference_characteristics.types[0];
                    let type_label = node_data.type_display(first_type, label_context, &node_data.indexers);
                    let type_response = if reference_characteristics.types.len()==1 {
                        ui.label(type_label.as_str())
                    } else {
                        ui.label(format!("{} +{}", type_label.as_str(), reference_characteristics.types.len() - 1).as_str())
                    };
                    if type_response.clicked() {
                        let mouse_pos = type_response.hover_pos().unwrap_or(Pos2::new(0.0, 0.0));
                        type_cell_action = TypeCellAction::ShowRefTypes(mouse_pos, *predicate_index);
                    }
                }
                ui.label(reference_characteristics.min_cardinality.to_string());
                ui.label(reference_characteristics.max_cardinality.to_string());
                ui.end_row();
            }
        });
        type_cell_action
    }
    pub fn display_reverse_references(&self, ui: &mut egui::Ui, label_context: &LabelContext, node_data: &NodeData) {
        egui::Grid::new("referenced by").show(ui, |ui| {
            ui.strong("In Ref");
            ui.strong("Count");
            ui.end_row();
            for (predicate_index, count) in &self.rev_references {
                let predicate_label = node_data.predicate_display(*predicate_index, label_context, &node_data.indexers);
                ui.label(predicate_label.as_str());
                ui.label(count.to_string());
                ui.end_row();
            }
        });
    }
}

fn text_wrapped(text: &str, width: f32, painter: &egui::Painter, top_left: Pos2, cell_hovered: bool, strong: bool, visuals: &egui::Visuals) {
    let mut job = egui::text::LayoutJob::default();
    job.append(
        text,
        0.0,
        egui::TextFormat {
            font_id: egui::FontId::default(),
            color: if cell_hovered {
                visuals.selection.stroke.color
            } else {
                if strong {
                    visuals.strong_text_color()
                } else {
                    visuals.text_color()
                }
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
    painter.galley(top_left, galley, visuals.text_color());
}

fn text_wrapped_link(text: &str, width: f32, painter: &egui::Painter, top_left: Pos2, hovered: bool, visuals: &egui::Visuals) {
    let mut job = egui::text::LayoutJob::default();
    job.append(
        text,
        0.0,
        egui::TextFormat {
            font_id: egui::FontId::default(),
            color: visuals.hyperlink_color,
            underline: if hovered {
                Stroke::new(1.0, visuals.hyperlink_color)
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
    painter.galley(top_left, galley, visuals.text_color());
}

impl TypeInstanceIndex {
    pub fn new() -> Self {
        Self {
            nodes: 0,
            unique_predicates: 0,
            unique_types: 0,
            properties: 0,
            references: 0,
            blank_nodes: 0,
            unresolved_references: 0,
            max_instance_type_count: 0,
            min_instance_type_count: 0,
            types: HashMap::new(),
            types_order: Vec::new(),
            types_filtered: Vec::new(),
            selected_type: None,
            types_filter: String::new(),
            type_cell_action: TypeCellAction::None,
        }
    }

    pub fn clean(&mut self) {
        self.nodes = 0;
        self.unique_predicates = 0;
        self.unique_types = 0;
        self.properties = 0;
        self.references = 0;
        self.blank_nodes = 0;
        self.unresolved_references = 0;
        self.max_instance_type_count = 0;
        self.min_instance_type_count = 0;
        self.types.clear();
        self.types_order.clear();
    }

    pub fn update(&mut self, node_data: &NodeData) {
        self.clean();
        #[cfg(not(target_arch = "wasm32"))]
        let start = Instant::now();
        let node_len = node_data.len();
        // TODO concurrent optimization
        // 1. partion the instances in groups (count  rayon::current_num_threads()) in dependency to type
        // 2. build hash map of each group (there are disjuct)
        // 3. merge all hash maps
        for (node_index, (_node_iri, node)) in node_data.iter().enumerate() {
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
                type_data.instances.push(node_index as IriIndex);
                for (property_index, property_stat) in type_data.properties.iter_mut() {
                    let mut property_card = 0;
                    for (predicate_index, value) in &node.properties {
                        if *property_index == *predicate_index {
                            property_stat.count += 1;
                            property_card += 1;
                            property_stat.max_len = property_stat
                                .max_len
                                .max(value.as_str_ref(&node_data.indexers).len() as u32);
                        }
                    }
                    property_stat.max_cardinality = property_stat.max_cardinality.max(property_card);
                    property_stat.min_cardinality = property_stat.min_cardinality.min(property_card);
                }
                let mut unknown_properties = vec![];
                for (predicate_index, _value) in &node.properties {
                    if !type_data.properties.contains_key(predicate_index) {
                        unknown_properties.push(*predicate_index);
                    }
                }
                for predicate_index in unknown_properties {
                    let mut property_card = 0;
                    let mut property_stat = DataPropCharacteristics {
                        count: 0,
                        max_len: 0,
                        min_cardinality: u32::MAX,
                        max_cardinality: 0,
                    };
                    for (property_index, value) in &node.properties {
                        if *property_index == predicate_index {
                            property_stat.count += 1;
                            property_card += 1;
                            property_stat.max_len = property_stat
                                .max_len
                                .max(value.as_str_ref(&node_data.indexers).len() as u32);
                        }
                    }
                    property_stat.max_cardinality = property_card;
                    property_stat.min_cardinality = property_card;
                    type_data.properties.insert(predicate_index, property_stat);
                }
                let mut ref_counts: Vec<(IriIndex, u32, Vec<IriIndex>)> = Vec::new();
                for (predicate_index, ref_index) in &node.references {
                    let ref_node = node_data.get_node_by_index(*ref_index);
                    if let Some((_str, ref_node)) = ref_node {
                        let mut found = false;
                        for (predicate_count_index, predicate_count, types) in ref_counts.iter_mut() {
                            if *predicate_count_index == *predicate_index {
                                *predicate_count += 1;
                                found = true;
                                for type_index in &ref_node.types {
                                    if !types.contains(type_index) {
                                        types.push(*type_index);
                                    }
                                }
                                break;
                            }
                        }
                        if !found {
                            ref_counts.push((*predicate_index, 1, ref_node.types.clone()));
                        }
                    }
                }
                // Search unknown references (set count to 0)
                for predicate_index in type_data.references.keys() {
                    if !ref_counts.iter().any(|(index, _, _)| *index == *predicate_index) {
                        ref_counts.push((*predicate_index, 0, vec![]));
                    }
                }
                for (predicate_index, count, types) in ref_counts {
                    let reference_characteristics = type_data.references.get_mut(&predicate_index);
                    if let Some(reference_characteristics) = reference_characteristics {
                        reference_characteristics.count += count;
                        reference_characteristics.max_cardinality =
                            reference_characteristics.max_cardinality.max(count);
                        reference_characteristics.min_cardinality =
                            reference_characteristics.min_cardinality.min(count);
                    } else {
                        type_data.references.insert(
                            predicate_index,
                            ReferenceCharacteristics {
                                count,
                                min_cardinality: count,
                                max_cardinality: count,
                                types,
                            },
                        );
                    }
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
            if self.min_instance_type_count == 0 && self.max_instance_type_count == 0 {
                self.min_instance_type_count = type_data.instances.len();
                self.max_instance_type_count = type_data.instances.len();
            } else {
                self.min_instance_type_count = self.min_instance_type_count.min(type_data.instances.len());
                self.max_instance_type_count = self.max_instance_type_count.max(type_data.instances.len());
            }
            for (predicate_index, data_characteristics) in type_data.properties.iter() {
                if type_data.instance_view.get_column(*predicate_index).is_none() {
                    let predicate_str = node_data.get_predicate(*predicate_index);
                    let column_desc = ColumnDesc {
                        predicate_index: *predicate_index,
                        width: (((data_characteristics.max_len + 1).max(3) as f32) * CHAR_WIDTH)
                            .min(DEFAULT_COLUMN_WIDTH),
                        visible: true,
                    };
                    if let Some(predicate_str) = predicate_str {
                        if predicate_str.contains("label") {
                            type_data.instance_view.display_properties.insert(0, column_desc);
                            continue;
                        }
                    }
                    type_data.instance_view.display_properties.push(column_desc);
                }
            }
            type_data.filtered_instances = type_data.instances.clone();
        }
        self.types_order.sort_by(|a, b| {
            let a_data = self.types.get(a).unwrap();
            let b_data = self.types.get(b).unwrap();
            b_data.instances.len().cmp(&a_data.instances.len())
        });
        if self.types_order.is_empty() {
            self.selected_type = None;
        } else {
            self.selected_type = Some(self.types_order[0]);
        }
        self.types_filter.clear();
        self.types_filtered = self.types_order.clone();
        #[cfg(not(target_arch = "wasm32"))]
        {
            let duration = start.elapsed();
            println!("Time taken to index {} nodes: {:?}", node_len, duration);
            println!("Nodes per second: {}", node_len as f64 / duration.as_secs_f64());
        }
    }

    pub fn apply_filter(&mut self, node_data: &mut NodeData, label_context: &LabelContext) {
        if self.types_filter.is_empty() {
            self.types_filtered = self.types_order.clone();
        } else {
            let filter = self.types_filter.to_lowercase();
            self.types_filtered = self
                .types_order
                .par_iter()
                .filter(|type_index| {
                    let label = node_data.type_display(**type_index, label_context, &node_data.indexers);
                    label.as_str().to_lowercase().contains(&filter)
                })
                .cloned()
                .collect();
        }
    }

    pub fn display(
        &mut self,
        ctx: &egui::Context,
        ui: &mut egui::Ui,
        rdf_data: &mut RdfData,
        layout_data: &mut UIState,
        color_cache: &GVisualisationStyle,
        iri_display: IriDisplay,
    ) -> NodeAction {
        let mut instance_action = NodeAction::None;
        egui::ScrollArea::horizontal().id_salt("h").show(ui, |ui| {
            ui.with_layout(Layout::left_to_right(Align::TOP), |ui| {
                ui.vertical(|ui| {
                    ui.heading("Statistics:");
                    ui.label(format!("Nodes: {}", self.nodes));
                    ui.label(format!("Unresolved References: {}", self.unresolved_references));
                    ui.label(format!("Blank Nodes: {}", self.blank_nodes));
                    ui.label(format!("Properties: {}", self.properties));
                    ui.label(format!("References: {}", self.references));
                    ui.label(format!("Unique Predicates: {}", self.unique_predicates));
                    ui.label(format!("Unique Types: {}", self.unique_types));
                    ui.label(format!("Unique Languages: {}", rdf_data.node_data.unique_languages()));
                    ui.label(format!("Unique Data Types: {}", rdf_data.node_data.unique_data_types()));
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
                        let type_filter_response = ui.text_edit_singleline(&mut self.types_filter);
                        let label_context =
                            LabelContext::new(layout_data.display_language, iri_display, &rdf_data.prefix_manager);
                        if type_filter_response.changed() {
                            self.apply_filter(&mut rdf_data.node_data, &label_context);
                        }
                        let (selected_type, type_table_action) = self.show_types(
                            ui,
                            &mut rdf_data.node_data,
                            &rdf_data.prefix_manager,
                            layout_data,
                            iri_display,
                            200.0,
                        );
                        if selected_type.is_some() {
                            self.selected_type = selected_type;
                        }
                        match type_table_action {
                            TypeTableAction::SortByLabel => {
                                self.types_filtered.par_sort_by(|a, b| {
                                    let label_a = rdf_data.node_data.type_display(
                                        *a,
                                        &label_context,
                                        &rdf_data.node_data.indexers,
                                    );
                                    let label_b = rdf_data.node_data.type_display(
                                        *b,
                                        &label_context,
                                        &rdf_data.node_data.indexers,
                                    );
                                    label_a.as_str().cmp(label_b.as_str())
                                });
                            }
                            TypeTableAction::SortByInstances => {
                                self.types_filtered.par_sort_by(|a, b| {
                                    let a_data = self.types.get(a).unwrap();
                                    let b_data = self.types.get(b).unwrap();
                                    b_data.instances.len().cmp(&a_data.instances.len())
                                });
                            }
                            TypeTableAction::SortByDataProps => {
                                self.types_filtered.par_sort_by(|a, b| {
                                    let a_data = self.types.get(a).unwrap();
                                    let b_data = self.types.get(b).unwrap();
                                    b_data.properties.len().cmp(&a_data.properties.len())
                                });
                            }
                            TypeTableAction::SortByOutRef => {
                                self.types_filtered.par_sort_by(|a, b| {
                                    let a_data = self.types.get(a).unwrap();
                                    let b_data = self.types.get(b).unwrap();
                                    b_data.references.len().cmp(&a_data.references.len())
                                });
                            }
                            TypeTableAction::SortByInRef => {
                                self.types_filtered.par_sort_by(|a, b| {
                                    let a_data = self.types.get(a).unwrap();
                                    let b_data = self.types.get(b).unwrap();
                                    b_data.rev_references.len().cmp(&a_data.rev_references.len())
                                });
                            }
                            TypeTableAction::None => {}
                        }
                    });
                });
                if let Some(selected_type) = self.selected_type {
                    let popup_id = ui.make_persistent_id("column_type_popup");
                    if let Some(type_data) = self.types.get_mut(&selected_type) {
                        let label_context =
                            LabelContext::new(layout_data.display_language, iri_display, &rdf_data.prefix_manager);
                        ui.allocate_ui(Vec2::new(ui.available_width(), 200.0), |ui| {
                            ui.separator();
                            egui::ScrollArea::vertical().id_salt("data").show(ui, |ui| {
                                type_data.display_data_props(ui, &label_context, &rdf_data.node_data);
                            });
                        });
                        ui.allocate_ui(Vec2::new(ui.available_width(), 200.0), |ui| {
                            ui.separator();
                            egui::ScrollArea::vertical().id_salt("ref").show(ui, |ui| {
                                let type_cell_action =
                                    type_data.display_references(ui, &label_context, &rdf_data.node_data);
                                match type_cell_action {
                                    TypeCellAction::ShowRefTypes(pos, predicate_index) => {
                                        ui.memory_mut(|mem| mem.toggle_popup(popup_id));
                                        self.type_cell_action = TypeCellAction::ShowRefTypes(pos, predicate_index);
                                    }
                                    _ => {}
                                }
                            });
                        });
                        ui.allocate_ui(Vec2::new(ui.available_width(), 200.0), |ui| {
                            ui.separator();
                            egui::ScrollArea::vertical().id_salt("refby").show(ui, |ui| {
                                type_data.display_reverse_references(ui, &label_context, &rdf_data.node_data);
                            });
                        });
                        popup_at(ui, popup_id, self.type_cell_action.pos(), 500.0, |ui| {
                            match self.type_cell_action {
                                TypeCellAction::ShowRefTypes(_pos, predicate_index) => {
                                    let mut close_menu = false;
                                    let charteristics = type_data.references.get(&predicate_index);
                                    if let Some(characterisics) = charteristics {
                                        for type_index in &characterisics.types {
                                            ui.label(
                                                rdf_data.node_data.type_display(
                                                    *type_index,
                                                    &label_context,
                                                    &rdf_data.node_data.indexers,
                                                ).as_str()
                                            );
                                        }
                                    } else {
                                        close_menu = true;
                                    }
                                    let button_text = egui::RichText::new(concatcp!(ICON_CLOSE, " Close")).size(16.0);
                                    let nav_but = egui::Button::new(button_text).fill(primary_color(ui.visuals()));
                                    let b_resp = ui.add(nav_but);
                                    if b_resp.clicked() {
                                        close_menu = true;
                                    }
                                    if close_menu {
                                        self.type_cell_action = TypeCellAction::None;
                                        ui.memory_mut(|mem| mem.close_popup());
                                    }
                                }
                                TypeCellAction::None => {}
                            }
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
                    let filter_immandiately = type_data.instances.len() < IMMADIATE_FILTER_COUNT;
                    let text_edit = egui::TextEdit::singleline(&mut type_data.instance_view.instance_filter);
                    let text_edit_response = ui.add(text_edit);
                    if ui.ctx().input(|i| i.key_pressed(egui::Key::F) && i.modifiers.command) {
                        text_edit_response.request_focus();
                    }
                    if filter_immandiately {
                        if text_edit_response.changed() {
                            table_action = TableAction::Filter;
                        }
                    } else if ui.button(ICON_FILTER).clicked() {
                        table_action = TableAction::Filter;
                    }
                    if ui.button(ICON_CLOSE).clicked() {
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
                                ctx,
                                &mut table_action,
                                &mut instance_action,
                                &mut rdf_data.node_data,
                                color_cache,
                                &rdf_data.prefix_manager,
                                layout_data,
                                iri_display,
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
                            for column_desc in type_data.instance_view.display_properties.iter_mut() {
                                if column_desc.predicate_index == predicate_to_hide {
                                    column_desc.visible = false;
                                    break;
                                }
                            }
                        }
                    }
                    TableAction::UhideColumn(preducate_to_unhide) => {
                        if let Some(type_data) = self.types.get_mut(&selected_type) {
                            for column_desc in type_data.instance_view.display_properties.iter_mut() {
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
                                let node_a = rdf_data.node_data.get_node_by_index(*a);
                                let node_b = rdf_data.node_data.get_node_by_index(*b);
                                if let Some((_, node_a)) = node_a {
                                    if let Some((_, node_b)) = node_b {
                                        let a_value =
                                            &node_a.get_property(predicate_to_sort, layout_data.display_language);
                                        let b_value =
                                            &node_b.get_property(predicate_to_sort, layout_data.display_language);
                                        if let Some(a_value) = a_value {
                                            if let Some(b_value) = b_value {
                                                let a_value = a_value.as_str_ref(&rdf_data.node_data.indexers);
                                                let b_value = b_value.as_str_ref(&rdf_data.node_data.indexers);
                                                a_value.cmp(b_value)
                                            } else {
                                                std::cmp::Ordering::Less
                                            }
                                        } else if let Some(_b_value) = b_value {
                                            std::cmp::Ordering::Greater
                                        } else {
                                            std::cmp::Ordering::Equal
                                        }
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
                                let node_a = rdf_data.node_data.get_node_by_index(*a);
                                let node_b = rdf_data.node_data.get_node_by_index(*b);
                                if let Some((_, node_a)) = node_a {
                                    if let Some((_, node_b)) = node_b {
                                        let a_value =
                                            &node_a.get_property(predicate_to_sort, layout_data.display_language);
                                        let b_value =
                                            node_b.get_property(predicate_to_sort, layout_data.display_language);
                                        if let Some(a_value) = a_value {
                                            if let Some(b_value) = b_value {
                                                let a_value = a_value.as_str_ref(&rdf_data.node_data.indexers);
                                                let b_value = b_value.as_str_ref(&rdf_data.node_data.indexers);
                                                b_value.cmp(a_value)
                                            } else {
                                                std::cmp::Ordering::Less
                                            }
                                        } else if let Some(_b_value) = b_value {
                                            std::cmp::Ordering::Greater
                                        } else {
                                            std::cmp::Ordering::Equal
                                        }
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
                                let node_a = rdf_data.node_data.get_node_by_index(*a);
                                let node_b = rdf_data.node_data.get_node_by_index(*b);
                                if let Some((_, node_a)) = node_a {
                                    if let Some((_, node_b)) = node_b {
                                        let a_value = node_a.references.len() + node_a.reverse_references.len();
                                        let b_value = node_b.references.len() + node_b.reverse_references.len();
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
                                let node_a = rdf_data.node_data.get_node_by_index(*a);
                                let node_b = rdf_data.node_data.get_node_by_index(*b);
                                if let Some((_, node_a)) = node_a {
                                    if let Some((_, node_b)) = node_b {
                                        let a_value = node_a.references.len() + node_a.reverse_references.len();
                                        let b_value = node_b.references.len() + node_b.reverse_references.len();
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
                    TableAction::SortIriAsc() => {
                        if let Some(type_data) = self.types.get_mut(&selected_type) {
                            type_data.filtered_instances.sort_by(|a, b| {
                                let node_a = rdf_data.node_data.get_node_by_index(*a);
                                let node_b = rdf_data.node_data.get_node_by_index(*b);
                                if let Some((iri_a, _)) = node_a {
                                    if let Some((iri_b, _)) = node_b {
                                        iri_a.cmp(iri_b)
                                    } else {
                                        std::cmp::Ordering::Greater
                                    }
                                } else {
                                    std::cmp::Ordering::Less
                                }
                            });
                        }
                    }
                    TableAction::SortIriDesc() => {
                        if let Some(type_data) = self.types.get_mut(&selected_type) {
                            type_data.filtered_instances.sort_by(|a, b| {
                                let node_a = rdf_data.node_data.get_node_by_index(*a);
                                let node_b = rdf_data.node_data.get_node_by_index(*b);
                                if let Some((iri_a, _)) = node_a {
                                    if let Some((iri_b, _)) = node_b {
                                        iri_b.cmp(iri_a)
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
                                let node = rdf_data.node_data.get_node_by_index(instance_index);
                                if let Some((node_iri, node)) = node {
                                    if node.apply_filter(
                                        &type_data.instance_view.instance_filter,
                                        node_iri,
                                        &rdf_data.node_data.indexers,
                                    ) {
                                        return true;
                                    }
                                }
                                false
                            })
                            .collect();
                        if (type_data.instance_view.pos / ROW_HIGHT) as usize >= type_data.filtered_instances.len() {
                            type_data.instance_view.pos = 0.0;
                        }
                    }
                    TableAction::HidePropExists(predicate_to_hide) => {
                        type_data.filtered_instances.retain(|&instance_index| {
                            let node = rdf_data.node_data.get_node_by_index(instance_index);
                            if let Some((_, node)) = node {
                                return node.has_property(predicate_to_hide);
                            }
                            false
                        });
                        if (type_data.instance_view.pos / ROW_HIGHT) as usize >= type_data.filtered_instances.len() {
                            type_data.instance_view.pos = 0.0;
                        }
                    }
                    TableAction::HidePropNonMulti(predicate_to_hide) => {
                        type_data.filtered_instances.retain(|&instance_index| {
                            let node = rdf_data.node_data.get_node_by_index(instance_index);
                            if let Some((_, node)) = node {
                                let mut found = false;
                                for (predicate, _literal) in node.properties.iter() {
                                    if *predicate == predicate_to_hide {
                                        if found {
                                            return true;
                                        }
                                        found = true;
                                    }
                                }
                            }
                            false
                        });
                        if (type_data.instance_view.pos / ROW_HIGHT) as usize >= type_data.filtered_instances.len() {
                            type_data.instance_view.pos = 0.0;
                        }
                    }
                    TableAction::HidePropNotExists(predicate_to_hide) => {
                        type_data.filtered_instances.retain(|&instance_index| {
                            let node = rdf_data.node_data.get_node_by_index(instance_index);
                            if let Some((_, node)) = node {
                                return !node.has_property(predicate_to_hide);
                            }
                            false
                        });
                        if (type_data.instance_view.pos / ROW_HIGHT) as usize >= type_data.filtered_instances.len() {
                            type_data.instance_view.pos = 0.0;
                        }
                    }
                    TableAction::None => {}
                }
            }
        } else {
            ui.label("Select a type to display its instances");
        }
        instance_action
    }

    fn show_types(
        &self,
        ui: &mut egui::Ui,
        node_data: &mut NodeData,
        prefix_manager: &PrefixManager,
        layout_data: &UIState,
        iri_display: IriDisplay,
        height: f32,
    ) -> (Option<IriIndex>, TypeTableAction) {
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
                    strong_unselectable(ui, "Type");
                    if ui.response().hovered() {
                        ui.output_mut(|o| o.cursor_icon = CursorIcon::PointingHand);
                    }
                    if ui.response().clicked() {
                        type_table_action = TypeTableAction::SortByLabel;
                    }
                });
                header.col(|ui| {
                    strong_unselectable(ui, "Inst#");
                    if ui.response().hovered() {
                        ui.output_mut(|o| o.cursor_icon = CursorIcon::PointingHand);
                    }
                    if ui.response().clicked() {
                        type_table_action = TypeTableAction::SortByInstances;
                    }
                });
                header.col(|ui| {
                    strong_unselectable(ui, "Data#");
                    if ui.response().hovered() {
                        ui.output_mut(|o| o.cursor_icon = CursorIcon::PointingHand);
                    }
                    if ui.response().clicked() {
                        type_table_action = TypeTableAction::SortByDataProps;
                    }
                });
                header.col(|ui| {
                    strong_unselectable(ui, "Out Ref#");
                    if ui.response().hovered() {
                        ui.output_mut(|o| o.cursor_icon = CursorIcon::PointingHand);
                    }
                    if ui.response().clicked() {
                        type_table_action = TypeTableAction::SortByOutRef;
                    }
                });
                header.col(|ui| {
                    strong_unselectable(ui, "In Ref#");
                    if ui.response().hovered() {
                        ui.output_mut(|o| o.cursor_icon = CursorIcon::PointingHand);
                    }
                    if ui.response().clicked() {
                        type_table_action = TypeTableAction::SortByInRef;
                    }
                });
            })
            .body(|body| {
                let label_context = LabelContext::new(layout_data.display_language, iri_display, prefix_manager);
                body.rows(text_height, self.types_filtered.len(), |mut row| {
                    let type_index = self.types_filtered.get(row.index()).unwrap();
                    row.set_selected(self.selected_type == Some(*type_index));
                    let type_data = self.types.get(type_index).unwrap();
                    let type_label = node_data.type_display(*type_index, &label_context, &node_data.indexers);
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
        (selected_type, type_table_action)
    }
}

impl Default for TypeInstanceIndex {
    fn default() -> Self {
        Self::new()
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
    SortIriAsc(),
    SortIriDesc(),
    HidePropNotExists(IriIndex),
    HidePropExists(IriIndex),
    HidePropNonMulti(IriIndex),
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
