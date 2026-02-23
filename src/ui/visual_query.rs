use std::cmp::min;

use egui::{Color32, CursorIcon, Frame, Key, Popup, Pos2, Rect, Sense, Stroke, Ui, Vec2};
use egui_extras::StripBuilder;
use const_format::concatcp;
#[cfg(not(target_arch = "wasm32"))]
use rfd::FileDialog;

use crate::{
    IriIndex, RdfGlanceApp, 
    domain::{
        LabelContext, RdfData, graph_styles::NodeStyle, rdf_data, 
        type_index::{InstanceColumnResize, TableContextMenu, TypeInstanceIndex, ValueStatistics}, 
        visual_query::{PredicateFilter, QueryReference, TableQuery}
    }, 
    support::uitools::{ScrollBar, popup_at, primary_color}, 
    ui::{draw_node_label, 
        style::{ICON_CLEAN_ALL, ICON_CLOSE, ICON_EXPORT, ICON_LINK, ICON_REV_LINK, ICON_RUN}, 
        table_view::{CHAR_WIDTH, COLUMN_GAP, ROW_HIGHT, text_wrapped}}, 
    uistate::{actions::NodeAction, ref_selection::RefSelection, SystemMessage} 
};

const TABLE_V_GAP: f32 = 50.0;
const TABLE_H_GAP: f32 = 50.0;
const TABLE_H: f32 = 20.0;
const TABLE_W: f32 = 200.0;
const PANEL_W: f32 = 400.0;
const PANEL_H: f32 = 300.0;

impl RdfGlanceApp {
    pub fn show_visual_query(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) -> NodeAction {       
        ui.horizontal(|ui| {
            if self.ui_state.visual_query.selected_type_iri.is_none() {
                if let Some(selected_type) = self.type_index.selected_type {
                    self.ui_state.visual_query.selected_type_iri = Some(selected_type);
                }
            }
            if let Some(selected_type) = self.ui_state.visual_query.selected_type_iri {
                let mut export_csv = false;
                if let Ok(rdf_data) = self.rdf_data.read() {
                    if let Some(table_query) = self.visual_query.root_table.as_mut() {
                        ui.horizontal(|ui| {
                            let any_popup = Popup::is_any_open(ctx);
                            let mut key_run = false;
                            if !any_popup {
                                ui.input(|i| {
                                    if i.key_pressed(Key::F5) {
                                        key_run = true;
                                    }
                                });
                            }
                            if key_run || ui.button(concatcp!(ICON_RUN, " Run (F5)")).clicked() {
                                if let Some(type_data) = self.type_index.types.get(&table_query.type_iri) {
                                    table_query.instances = type_data.query_instances_for_table_query(table_query, &rdf_data);
                                }
                                self.visual_query.instance_view.pos = 0.0;
                                self.visual_query.instances = table_query.compute_instances(&rdf_data);
                            }
                            if !self.visual_query.instances.is_empty() {
                                ui.label((self.visual_query.instances.len() / self.visual_query.tables_pro_row).to_string());
                                if ui.button(concatcp!(ICON_EXPORT, " Export CSV")).clicked() {
                                    export_csv = true;
                                }
                            }
                        });
                    } else {
                        let label_context = LabelContext::new(self.ui_state.display_language, self.persistent_data.config_data.iri_display, &rdf_data.prefix_manager);
                        let type_str = rdf_data.node_data.type_display(selected_type, &label_context, &rdf_data.node_data.indexers);
                        egui::ComboBox::from_id_salt("selected_type")
                            .selected_text(type_str.as_str())
                            .show_ui(ui, |ui| {
                                for type_idx in self.type_index.types_order.iter() {
                                    let type_str = rdf_data.node_data.type_display(*type_idx, &label_context, &rdf_data.node_data.indexers);
                                    if ui.selectable_label(selected_type == *type_idx, type_str.as_str()).clicked() {
                                        self.ui_state.visual_query.selected_type_iri = Some(*type_idx);
                                    }
                                }
                            });
                        if ui.button("Add Table").clicked() {
                            if let Some(type_to_add) = self.type_index.types.get(&selected_type) {
                                let mut table_query = TableQuery {
                                    type_iri: selected_type,
                                    visible_predicates: type_to_add.instance_view.display_properties.clone(),
                                    ..Default::default()
                                };
                                table_query.refresh_table_data();
                                self.visual_query.selected_table = Some(0);
                                layout_tree(ui.available_width()-PANEL_W, PANEL_H, &mut table_query, Vec2::new(TABLE_W, TABLE_H) , TABLE_H_GAP, TABLE_V_GAP);
                                self.visual_query.root_table = Some (table_query);
                                
                            }
                        }
                    }                    
                }
                if export_csv {
                    self.export_visual_query_dialog();
                }
            }            
        });
        let available_width = ui.available_width();
        ui.allocate_ui(Vec2::new(available_width, PANEL_H), |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.set_height(PANEL_H);
                if let Ok(rdf_data) = self.rdf_data.read() {
                    let label_context = LabelContext::new(self.ui_state.display_language, self.persistent_data.config_data.iri_display, &rdf_data.prefix_manager);
                    egui::SidePanel::right("details_panel")
                        .exact_width(500.0)
                        .show_inside(ui, |ui| {
                            egui::ScrollArea::both().show(ui, |ui| {
                                if let Some(table_query) = self.visual_query.root_table.as_mut() {
                                    show_query_table_details(ui, table_query, &rdf_data, &label_context, &self.type_index, self.visual_query.selected_table);
                                }
                            });
                    });
                    egui::CentralPanel::default().show_inside(ui, |ui| {
                        if let Some(mut table_query) = self.visual_query.root_table.as_mut() {
                            let mut structure_updated = false;
                            let mut selected_table = self.visual_query.selected_table;
                            let offset = ui.next_widget_position();
                            let mut show_context = QueryTableShowContext {
                                label_context: &label_context,
                                rdf_data: &rdf_data,
                                selected_table: &mut selected_table,
                                structure_updated: &mut structure_updated,
                                type_index: &self.type_index,
                                ui: ui,
                                offset: offset,
                            };
                            show_query_table( table_query, &mut show_context);
                            if table_query.to_remove {
                                self.visual_query.clear_instances();
                                self.visual_query.root_table = None;
                            } else if structure_updated {
                                let tables_count = table_query.refresh_table_data();
                                self.visual_query.tables_pro_row = tables_count;
                                layout_tree(ui.available_width()-PANEL_W, PANEL_H, &mut table_query, Vec2::new(TABLE_W, TABLE_H) , TABLE_H_GAP, TABLE_V_GAP);
                                self.visual_query.clear_instances();
                            }
                            if let Some(selected_table) = selected_table {
                                self.visual_query.selected_table = Some(selected_table);
                            }
                        }
                    });
                }
            });
        });

        let needed_len = (self.visual_query.instances.len()/self.visual_query.tables_pro_row + 2) as f32 * ROW_HIGHT;
        let a_height = ui.available_height();
        StripBuilder::new(ui)
            .size(egui_extras::Size::remainder())
            .size(egui_extras::Size::exact(20.0)) // Two resizable panels with equal initial width
            .horizontal(|mut strip| {
                strip.cell(|ui| {
                    self.show_query_result(ctx, ui);
                });
                strip.cell(|ui| {
                    ui.add(ScrollBar::new(
                        &mut self.visual_query.instance_view.pos,
                        &mut self.visual_query.instance_view.drag_pos,
                        needed_len,
                        a_height,
                    ));
                });
            });
        NodeAction::None
    }

    pub fn show_query_result(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) -> NodeAction {
        if self.visual_query.root_table.is_none() {
            return NodeAction::None;
        }
        if let Ok(rdf_data) = self.rdf_data.read() {
            let label_context = LabelContext::new(self.ui_state.display_language, self.persistent_data.config_data.iri_display, &rdf_data.prefix_manager);
            // For simplicity, we will just show the number of instances for now
            let a_height = ui.available_height();
            let available_width = ui.available_width();
            let available_height = ui.available_height();
            let available_rect = ui.max_rect();
            let size = Vec2::new(available_width, available_height);
            let (rect, response) = ui.allocate_at_least(size, Sense::click_and_drag());
            let painter = ui.painter();
            let primary_clicked = response.clicked();
            let secondary_clicked = response.secondary_clicked();
            let mouse_pos = response.hover_pos().unwrap_or(Pos2::new(0.0, 0.0));
            let mut primary_down = false;
            ctx.input(|input| {
                if input.pointer.button_pressed(egui::PointerButton::Primary) {
                    primary_down = true;
                }
            });
            if response.drag_stopped() {
                self.visual_query.instance_view.column_resize = InstanceColumnResize::None;
            }
            let popup_id = ui.make_persistent_id("query_context_menu");
            let mut was_context_click = false;

            if let Some(table_query) = self.visual_query.root_table.as_mut() {
                if response.dragged() {
                    match self.visual_query.instance_view.column_resize {
                        InstanceColumnResize::None => {}
                        InstanceColumnResize::QueryPredicate(start_pos, predicate_index, table_idx) => {
                            ui.output_mut(|o| o.cursor_icon = CursorIcon::ResizeHorizontal);
                            for table_query in table_query.iter_tables_mut() {
                                if table_query.row_index == table_idx {
                                    for column_desc in table_query.visible_predicates.iter_mut() {
                                        if column_desc.predicate_index == predicate_index {
                                            let width = mouse_pos.x - start_pos.x;
                                            if width > CHAR_WIDTH * 2.0 {
                                                column_desc.width = width;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        _ => {
                            
                        }
                    }
                }
                painter.rect_filled(
                    Rect::from_min_size(available_rect.left_top(), Vec2::new(available_width, ROW_HIGHT)),
                    0.0,
                    ui.visuals().code_bg_color,
                );
                let mut xpos = 0.0;
                // Draw columns headers
                for table_query in table_query.iter_tables_mut() {
                    for column_desc in table_query.visible_predicates
                        .iter()
                        .filter(|p| p.visible) {
                        let top_left = available_rect.left_top() + Vec2::new(xpos, 0.0);
                        let predicate_label =
                            rdf_data.node_data.predicate_display(column_desc.predicate_index, &label_context, &rdf_data.node_data.indexers);
                        text_wrapped(
                            predicate_label.as_str(),
                            column_desc.width,
                            painter,
                            top_left,
                            false,
                            true,
                            ui.visuals(),
                        );
                        xpos += column_desc.width + COLUMN_GAP;
                        let column_rect = egui::Rect::from_min_size(top_left, Vec2::new(column_desc.width, ROW_HIGHT));
                        if column_rect.contains(mouse_pos) {
                            if secondary_clicked {
                                was_context_click = true;
                                Popup::open_id(ctx, popup_id);
                                self.visual_query.instance_view.context_menu =
                                    TableContextMenu::QueryColumnMenu(mouse_pos, column_desc.predicate_index, table_query.row_index);
                            } else {
                                ui.output_mut(|o| o.cursor_icon = CursorIcon::ContextMenu);
                            }
                        }
                        let columns_drag_size_rect = egui::Rect::from_min_size(
                            top_left + Vec2::new(column_desc.width - 3.0, 0.0),
                            Vec2::new(6.0, ROW_HIGHT),
                        );
                        if columns_drag_size_rect.contains(mouse_pos) {
                            ui.output_mut(|o| o.cursor_icon = CursorIcon::ResizeHorizontal);
                            if primary_down && matches!(self.visual_query.instance_view.column_resize, InstanceColumnResize::None) {
                                self.visual_query.instance_view.column_resize = InstanceColumnResize::QueryPredicate(
                                    mouse_pos - Vec2::new(column_desc.width, 0.0),
                                    column_desc.predicate_index,
                                    table_query.row_index,
                                );
                            }
                        }
                    }
                }
            }
            // draw rows
            let instance_index = (self.visual_query.instance_view.pos / ROW_HIGHT) as usize;
            let capacity = ((a_height / ROW_HIGHT) as usize).max(2) - 1;
            let tables_per_row = self.visual_query.tables_pro_row;
            let mut start_pos = instance_index;
            let mut ypos = ROW_HIGHT;
            let instance_slice = &self.visual_query.instances[instance_index*tables_per_row..min((instance_index+capacity)*tables_per_row, self.visual_query.instances.len())];
            for instances in instance_slice.chunks(tables_per_row) {
                let mut xpos = 0.0;
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
                if let Some(table_query) = self.visual_query.root_table.as_mut() {
                    for (table_query, instance_index) in table_query.iter_tables().zip(instances) {
                        let node = rdf_data.node_data.get_node_by_index(*instance_index);
                        if let Some((_node_iri, node)) = node {
                        for column_desc in table_query.visible_predicates
                            .iter()
                            .filter(|p| p.visible) {
                                let property = node.get_property_count(column_desc.predicate_index, self.ui_state.display_language);
                                if let Some((property, count)) = property {
                                    let value = property.as_str_ref(&rdf_data.node_data.indexers);
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
                                    text_wrapped(
                                        value,
                                        column_desc.width,
                                        painter,
                                        cell_rect.left_top(),
                                        cell_hovered,
                                        false,
                                        ui.visuals(),
                                    );
                                    if primary_clicked && cell_rect.contains(mouse_pos) {
                                        was_context_click = true;
                                        Popup::open_id(ctx, popup_id);
                                        self.visual_query.instance_view.ref_selection = RefSelection::None;
                                        self.visual_query.instance_view.context_menu =
                                            TableContextMenu::CellMenu(mouse_pos, *instance_index, column_desc.predicate_index);
                                    }
                                }
                                xpos += column_desc.width + COLUMN_GAP;
                                if xpos > available_rect.width() {
                                    break;
                                }
                            }
                        }
                    }
                }
                ypos += ROW_HIGHT;
            }

            // Draw vertical lines
            if let Some(table_query) = &self.visual_query.root_table {
                let mut xpos = 0.0;
                for table_query in table_query.iter_tables() {
                    for column_desc in table_query.visible_predicates
                        .iter()
                        .filter(|p| p.visible)
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
                }
            }

            if !was_context_click && primary_clicked {
                self.visual_query.instance_view.context_menu = TableContextMenu::None;
                Popup::close_id(ctx, popup_id);
            }
            let width = match self.visual_query.instance_view.context_menu {
                TableContextMenu::CellMenu(_, _, _) => {
                     500.0
                },
                TableContextMenu::QueryColumnMenu(_,_ ,_ ) => {
                    if self.visual_query.value_statistics.is_some() {
                        600.0
                    } else {
                        200.0
                    }
                }
                _ => 200.0,
            };
            popup_at(
                ui,
                popup_id,
                self.visual_query.instance_view.context_menu.pos(),
                width,
                |ui| {
                if let Some(value_statistics) = &self.visual_query.value_statistics {
                    if value_statistics.show_ui(ui, &rdf_data) {
                        self.visual_query.value_statistics = None;
                        self.visual_query.instance_view.context_menu = TableContextMenu::None;
                        Popup::close_id(ctx, popup_id);
                    }
                } else {
                    match self.visual_query.instance_view.context_menu {
                        TableContextMenu::CellMenu(_pos, instance_index, predicate) => {
                            let mut close_menu = false;
                            let node = rdf_data.node_data.get_node_by_index(instance_index);
                            if let Some((_node_iri, node)) = node {
                                for (predicate_index, value) in &node.properties {
                                    if predicate == *predicate_index {
                                        ui.label(value.as_str_ref(&rdf_data.node_data.indexers));
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
                                self.visual_query.instance_view.context_menu = TableContextMenu::None;
                                Popup::close_id(ctx, popup_id);
                            }
                        }
                        TableContextMenu::QueryColumnMenu(_pos, predicate, table_idx) => {
                            let mut close_menu = false;
                            if ui.button("Hide Column").clicked() {
                                if let Some(root_table) = self.visual_query.root_table.as_mut() {
                                    for table in root_table.iter_tables_mut() {
                                        if table.row_index == table_idx {
                                            for column_desc in table.visible_predicates.iter_mut() {
                                                if column_desc.predicate_index == predicate {
                                                    column_desc.visible = false;
                                                    break;
                                                }
                                            }
                                            break;
                                        }
                                    }
                                }
                                close_menu = true;
                            }
                            if ui.button("Sort Asc").clicked() {
                                let value_type = self.visual_query.value_type(table_idx, predicate, &self.type_index);
                                self.visual_query.sort_instances(table_idx, predicate, 
                                    &rdf_data, value_type, true, self.ui_state.display_language);
                                close_menu = true;
                            }
                            if ui.button("Sort Desc").clicked() {
                                let value_type = self.visual_query.value_type(table_idx, predicate, &self.type_index);
                                self.visual_query.sort_instances(table_idx, predicate, &rdf_data,value_type, false, self.ui_state.display_language);
                                close_menu = true;
                            }
                            if ui.button("Add Filter").clicked() {
                                self.visual_query.selected_table = Some(table_idx);
                                if let Some(root_table) = self.visual_query.root_table.as_mut() {
                                    for table in root_table.iter_tables_mut() {
                                        if table.row_index == table_idx {
                                            if !table.predicate_filters.iter().any(|f| f.predicate_iri==predicate) {
                                                table.predicate_filters.push(PredicateFilter {
                                                    predicate_iri: predicate,
                                                    ..Default::default()
                                                })
                                            }
                                        }
                                    }
                                }
                                close_menu = true;
                            }
                            if ui.button("Show Value Statistics").clicked() {
                                let value_type = self.visual_query.value_type(table_idx, predicate, &self.type_index);
                                self.visual_query.value_statistics = Some(ValueStatistics::calculate_value_statistics(predicate, value_type, &rdf_data.node_data, 
                                    self.visual_query.instances.iter().skip(table_idx).step_by(self.visual_query.tables_pro_row)));
                            }
                            if close_menu {
                                self.visual_query.instance_view.context_menu = TableContextMenu::None;
                                Popup::close_id(ctx, popup_id);
                            }
                        }
                        _ => {}
                    }
                }
           });
        }       
        NodeAction::None
    }

    pub fn export_visual_query_dialog(&mut self) {
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(path) = FileDialog::new()
            .add_filter("CSV", &["csv"])
            .set_file_name("visual_query.csv")
            .save_file()
        {
            if let Ok(rdf_data) = self.rdf_data.read() {
                let label_context = LabelContext::new(
                    self.ui_state.display_language,
                    self.persistent_data.config_data.iri_display,
                    &rdf_data.prefix_manager,
                );
                let mut wtr = csv::Writer::from_path(path).unwrap();
                let store_res = self.visual_query.export_csv(&mut wtr, &rdf_data.node_data, &label_context);
                match store_res {
                    Err(e) => {
                        self.system_message = SystemMessage::Error(format!("Can not export csv: {}", e));
                    }
                    Ok(_) => {}
                }
            }
        }
        #[cfg(target_arch = "wasm32")]
        if let Ok(rdf_data) = self.rdf_data.read() {
            let label_context = LabelContext::new(
                self.ui_state.display_language,
                self.persistent_data.config_data.iri_display,
                &rdf_data.prefix_manager,
            );
            let mut buf = Vec::new();
            let mut wtr = csv::Writer::from_writer(buf);
            let store_res = self.visual_query.export_csv(&mut wtr, &rdf_data.node_data, &label_context);
            match store_res {
                Err(e) => {
                    self.system_message = SystemMessage::Error(format!("Can not export csv: {}", e));
                }
                Ok(_) => {
                    use crate::support::uitools::web_download;
                    let buf = wtr.into_inner().unwrap();
                    let _ = web_download("visual_query.csv", &buf);
                }
            }
        }
    }
}

struct QueryTableShowContext<'a> {
    ui: &'a mut Ui,
    rdf_data: &'a RdfData,
    label_context: &'a LabelContext<'a>,
    type_index: &'a TypeInstanceIndex,
    structure_updated: &'a mut bool,
    selected_table: &'a mut Option<usize>,
    offset: Pos2,
}

fn show_query_table(table_query: &mut TableQuery, show_context: &mut QueryTableShowContext) {
    if let Some(type_data) = show_context.type_index.types.get(&table_query.type_iri) {
        let selected_table = show_context.selected_table.as_ref().copied();
        let bg_fill = show_context.ui.visuals().selection.bg_fill;
        let resp = show_context.ui.allocate_ui_at_rect(
  Rect::from_min_size(table_query.position+show_context.offset.to_vec2(), Vec2::new(TABLE_W, TABLE_H)),|ui| {
                let frame = if Some(table_query.row_index) == selected_table {
                    Frame::group(ui.style()).fill(bg_fill)
                } else {
                    Frame::group(ui.style())    
                };                
                frame.show(ui, |ui| {
                ui.horizontal(|ui| {
                    let type_str = show_context.rdf_data.node_data.type_display(table_query.type_iri, show_context.label_context, &show_context.rdf_data.node_data.indexers);
                    if ui.label(type_str.as_str()).clicked() {
                        *show_context.selected_table = Some(table_query.row_index);
                    }
                    if ui.button(ICON_CLEAN_ALL).clicked() {
                        table_query.to_remove = true;
                        *show_context.structure_updated = true;
                    }
                    if !type_data.references.is_empty() {
                        ui.menu_button(ICON_LINK,|ui| {
                            if let Some(type_desc) = show_context.type_index.types.get(&table_query.type_iri) {
                                for (predicate_idx, reference_characteristics) in type_desc.references.iter() {
                                    for type_idx in reference_characteristics.types.iter() {
                                        let type_str = show_context.rdf_data.node_data.type_display(*type_idx, 
                                            show_context.label_context, &show_context.rdf_data.node_data.indexers);
                                        let predicate_str = show_context.rdf_data.node_data.predicate_display(*predicate_idx, 
                                            show_context.label_context, &show_context.rdf_data.node_data.indexers);
                                        if ui.button(format!("{} -> {}", type_str.as_str(), predicate_str.as_str())).clicked() {
                                            let ref_table_query = TableQuery {
                                                type_iri: *type_idx,
                                                visible_predicates: show_context.type_index.types.get(type_idx).unwrap().instance_view.display_properties.clone(),
                                                ..Default::default()
                                            };
                                            table_query.references.push(QueryReference {
                                                predicate: *predicate_idx,
                                                table_query: ref_table_query,
                                                ..Default::default()
                                            });
                                            *show_context.structure_updated = true;
                                        }
                                    }
                                }
                            }
                        });
                        
                    }
                    if !type_data.rev_references.is_empty() {
                        ui.menu_button(ICON_REV_LINK,|ui| {
                            if let Some(type_desc) = show_context.type_index.types.get(&table_query.type_iri) {
                                for (predicate_idx, reference_characteristics) in type_desc.rev_references.iter() {
                                    for type_idx in reference_characteristics.types.iter() {
                                        let type_str = show_context.rdf_data.node_data.type_display(*type_idx, show_context.label_context, &show_context.rdf_data.node_data.indexers);
                                        let predicate_str = show_context.rdf_data.node_data.predicate_display(*predicate_idx, 
                                            show_context.label_context, &show_context.rdf_data.node_data.indexers);
                                        if ui.button(format!("{} -> {}", type_str.as_str(), predicate_str.as_str())).clicked() {
                                            let ref_table_query = TableQuery {
                                                type_iri: *type_idx,
                                                visible_predicates: show_context.type_index.types.get(type_idx).unwrap().instance_view.display_properties.clone(),
                                                ..Default::default()
                                            };
                                            table_query.references.push(QueryReference {
                                                predicate: *predicate_idx,
                                                table_query: ref_table_query,
                                                is_outgoing: false,
                                                ..Default::default()
                                            });
                                            *show_context.structure_updated = true;
                                        }
                                    }
                                }
                            }
                        });
                    }
                });
            });

        });
        if !table_query.references.is_empty() {
            let node_rect = resp.response.rect;
            let stroke = Stroke::new(1.0, show_context.ui.visuals().text_color());
            let node_style = NodeStyle {
                font_size: 10.0,
                height: 5.0,
                width: 5.0,
                ..Default::default()
            };
            for reference in table_query.references.iter_mut() {
                let painter = show_context.ui.painter();
                let points = [
                    Pos2::new(
                        node_rect.max.x,
                        node_rect.min.y+node_rect.height()*0.5,
                    ),
                    Pos2::new(
                        show_context.offset.x+reference.table_query.position.x,
                        show_context.offset.y+reference.table_query.position.y+TABLE_H*0.5,
                    ),
                ];
                painter.line(points.to_vec(), stroke);
                let middle_pos = points[0]*0.5+(points[1].to_vec2())*0.5;
                let predicate_str = show_context.rdf_data.node_data.predicate_display(reference.predicate, 
                    &show_context.label_context, &show_context.rdf_data.node_data.indexers);
                draw_node_label(&painter, predicate_str.as_str(), &node_style, 
                    middle_pos, false, false, false, true, 0, show_context.ui.visuals());
                show_query_table(&mut reference.table_query, show_context);
            }
            table_query.references.retain(|r| !r.table_query.to_remove);
        }
    }
}

pub fn show_query_table_details(ui: &mut egui::Ui, root_table: &mut TableQuery, rdf_data: &rdf_data::RdfData, label_context: &LabelContext, 
    _type_index: &TypeInstanceIndex, selected_table: Option<usize>) {
    if let Some(selected_table) = selected_table {
        for table_query in root_table.iter_tables_mut() {
            if table_query.row_index == selected_table {
                ui.horizontal(|ui| {
                    if ui.button("hide all").clicked() {
                        for column_desc in table_query.visible_predicates.iter_mut() {
                            column_desc.visible = false;
                        }
                    }
                    if ui.button("show all").clicked() {
                        for column_desc in table_query.visible_predicates.iter_mut() {
                            column_desc.visible = true;
                        }
                    }
                });
                let mut add_filter: Option<IriIndex> = None;
                for column_desc in table_query.visible_predicates.iter_mut() {
                    let predicate_str = rdf_data.node_data.predicate_display(column_desc.predicate_index, &label_context, &rdf_data.node_data.indexers);
                    ui.horizontal(|ui| {
                        ui.checkbox(&mut column_desc.visible, predicate_str.as_str());
                        if !table_query.predicate_filters.iter().any(|f| f.predicate_iri==column_desc.predicate_index) {
                            if ui.button("Add Filter").clicked() {
                                add_filter = Some(column_desc.predicate_index);
                            }
                        }
                    });
                }
                if let Some(predicate_iri) = add_filter {
                    table_query.predicate_filters.push(crate::domain::visual_query::PredicateFilter {
                        predicate_iri,
                        ..Default::default()
                    });
                }
                // Filters
                for filter in table_query.predicate_filters.iter_mut() {
                    let predicate_str = rdf_data.node_data.predicate_display(filter.predicate_iri, 
                        &label_context, &rdf_data.node_data.indexers);
                    ui.horizontal(|ui| {
                        ui.label(predicate_str.as_str());
                        egui::ComboBox::from_id_salt(format!("filter_type_{}", predicate_str.as_str()))
                            .selected_text(filter.filter_type.to_string())
                            .show_ui(ui, |ui| {
                                use strum::IntoEnumIterator;
                                for filter_type in crate::domain::visual_query::FilterType::iter() {
                                    if ui.selectable_label(filter.filter_type == filter_type, filter_type.to_string().as_str()).clicked() {
                                        filter.filter_type = filter_type;
                                    }
                                }
                            });
                        ui.add(
                            egui::TextEdit::singleline(&mut filter.filter_value)
                                .desired_width(100.0)
                        );
                        if ui.button(ICON_CLEAN_ALL).clicked() {
                            filter.to_remove = true;
                        }
                    });
                }
                table_query.predicate_filters.retain(|f| !f.to_remove);
                break;
            }
        }
    }
}

pub fn layout_tree(
    width: f32,
    height: f32,
    root: &mut TableQuery,
    box_size: Vec2,
    h_gap: f32,
    v_gap: f32,
) {
    // 1️⃣ Measure required size
    let tree_width = measure_width(root, box_size.x, h_gap);
    let tree_height = measure_height(root, box_size.y, v_gap);

    // 2️⃣ Center if it fits, else top-left
    let offset_x = if tree_width < width {
        (width - tree_width) * 0.5
    } else {
        0.0
    };

    let offset_y = if tree_height < height {
        (height - tree_height) * 0.5
    } else {
        0.0
    };

    // 3️⃣ Assign positions
    assign_positions(
        root,
        offset_x,
        offset_y,
        box_size,
        h_gap,
        v_gap,
    );
}

fn measure_width(node: &TableQuery, box_width: f32, h_gap: f32) -> f32 {
    if node.references.is_empty() {
        return box_width;
    }

    let max_child_width = node
        .references
        .iter()
        .map(|c| measure_width(&c.table_query, box_width, h_gap))
        .fold(0.0, f32::max);

    box_width + h_gap + max_child_width
}

fn measure_height(node: &TableQuery, box_height: f32, v_gap: f32) -> f32 {
    if node.references.is_empty() {
        return box_height;
    }

    let mut total = 0.0;

    for (i, child) in node.references.iter().enumerate() {
        if i > 0 {
            total += v_gap;
        }
        total += measure_height(&child.table_query, box_height, v_gap);
    }

    total.max(box_height)
}

fn assign_positions(
    node: &mut TableQuery,
    start_x: f32,
    start_y: f32,
    box_size: Vec2,
    h_gap: f32,
    v_gap: f32,
) -> f32 {
    let subtree_height = measure_height(node, box_size.y, v_gap);

    // Center this node vertically in subtree
    let node_y = start_y + subtree_height * 0.5 - box_size.y * 0.5;
    node.position = Pos2::new(start_x, node_y);

    if node.references.is_empty() {
        return subtree_height;
    }

    let mut current_y = start_y;

    for child in &mut node.references {
        let child_height = measure_height(&child.table_query, box_size.y, v_gap);

        assign_positions(
            &mut child.table_query,
            start_x + box_size.x + h_gap,
            current_y,
            box_size,
            h_gap,
            v_gap,
        );

        current_y += child_height + v_gap;
    }

    subtree_height
}
