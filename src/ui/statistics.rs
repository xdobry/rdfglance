use std::{borrow::Cow, cmp::min, io};

use const_format::concatcp;
use egui::{Color32, CursorIcon, Key, Pos2, Rect, Sense, Stroke, Vec2};
use egui_extras::StripBuilder;

use crate::{
    uistate::actions::NodeAction, RdfGlanceApp, uistate::UIState, 
    domain::{LabelContext, LangIndex, RdfData,
        config::{Config, IriDisplay}, 
        statistics::StatisticsData,
        graph_styles::GVisualizationStyle
    }, 
    support::uitools::ScrollBar, 
    ui::{
        style::ICON_EXPORT, 
        table_view::{text_wrapped, text_wrapped_link}
    }
};

const ROW_HIGHT: f32 = 17.0;
const COLUMN_GAP: f32 = 2.0;
const RESULT_WIDTH: f32 = 100.0;

const FIX_LABELS: [&str; 3] = ["iri", "label", "type"];

enum StatisticsTableAction {
    None,
    SortResult(usize),
}

impl RdfGlanceApp {
    pub fn show_statistics(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) -> NodeAction {
        if self.statistics_data.is_some() {
            ui.horizontal(|ui| {
                ui.label("Statistics Data Available");
                if ui
                    .button(concatcp!(ICON_EXPORT, " Export CSV"))
                    .on_hover_text("Export as CSV file")
                    .clicked()
                {
                    if let Ok(rdf_data) = self.rdf_data.read() {
                        #[cfg(not(target_arch = "wasm32"))]
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("CSV File", &["csv"])
                            .set_file_name("statistics.csv")
                            .save_file()
                        {
                            let mut wtr = csv::Writer::from_path(path).unwrap();
                            let store_res = self.statistics_data.as_ref().unwrap().export_csv_writer(
                                &rdf_data,
                                &mut wtr,
                                self.persistent_data.config_data.iri_display,
                                &self.visualization_style,
                                self.ui_state.display_language,
                            );
                            match store_res {
                                Err(e) => {
                                    self.system_message = crate::SystemMessage::Error(format!("Can not save statistics: {}", e));
                                }
                                Ok(_) => {}
                            }
                        }
                        #[cfg(target_arch = "wasm32")]
                        {
                            use crate::support::uitools::web_download;
                            let buf = Vec::new();
                            let mut wtr = csv::Writer::from_writer(buf);
                            let _ = self.statistics_data.as_ref().unwrap().export_csv_writer(&rdf_data,
                                &mut wtr,
                                self.persistent_data.config_data.iri_display,
                                &self.visualization_style,
                                self.ui_state.display_language);
                            let buf = wtr.into_inner().unwrap();
                            let _ = web_download("statistics.csv",&buf);
                        }
                    }
                }
            });
            self.show_statistics_data(ctx, ui)
        } else {
            ui.label("No Statistics Data yet. Add some nodes to visual graph and run statistics algorithms on this");
            NodeAction::None
        }
    }

    pub fn show_statistics_data(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) -> NodeAction {
        let mut instance_action = NodeAction::None;
        if let Some(statistics_data) = self.statistics_data.as_mut() {
            let needed_len = (statistics_data.nodes.len() + 2) as f32 * ROW_HIGHT;
            let a_height = ui.available_height();
            StripBuilder::new(ui)
                .size(egui_extras::Size::remainder())
                .size(egui_extras::Size::exact(20.0)) // Two resizable panels with equal initial width
                .horizontal(|mut strip| {
                    strip.cell(|ui| {
                        if let Ok(mut rdf_data) = self.rdf_data.write() {
                            statistics_data.instance_table(
                                ui,
                                ctx,
                                &mut rdf_data,
                                &mut instance_action,
                                &self.ui_state,
                                self.persistent_data.config_data.iri_display,
                                &self.visualization_style,
                                &self.persistent_data.config_data,
                            );
                        }
                    });
                    strip.cell(|ui| {
                        ui.add(ScrollBar::new(
                            &mut statistics_data.pos,
                            &mut statistics_data.drag_pos,
                            needed_len,
                            a_height,
                        ));
                    });
                });
        }
        instance_action
    }
}

impl StatisticsData {
    pub fn instance_table(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        rfd_data: &mut RdfData,
        instance_action: &mut NodeAction,
        layout_data: &UIState,
        iri_display: IriDisplay,
        styles: &GVisualizationStyle,
        config: &Config,
    ) {
        let mut instance_index = (self.pos / ROW_HIGHT) as usize;
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
        let mut table_action = StatisticsTableAction::None;

        painter.rect_filled(
            Rect::from_min_size(available_rect.left_top(), Vec2::new(available_width, ROW_HIGHT)),
            0.0,
            ui.visuals().code_bg_color,
        );

        let mut primary_down = false;
        let mut sort_idx: Option<usize> = None;

        ctx.input(|i| {
            if i.pointer.button_pressed(egui::PointerButton::Primary) {
                primary_down = true;
            }
            if let Some((selected_iri, idx)) = self.selected_idx {
                if idx > 0 && i.modifiers.is_none() && i.key_pressed(Key::ArrowUp) {
                    let new_idx = idx - 1;
                    self.selected_idx = Some((self.nodes[new_idx].0, new_idx));
                    if new_idx < instance_index {
                        instance_index = new_idx;
                        self.pos = (instance_index as f32) * ROW_HIGHT;
                    }
                } else if idx < self.nodes.len() - 1 && i.modifiers.is_none() && i.key_pressed(Key::ArrowDown) {
                    let new_idx = idx + 1;
                    self.selected_idx = Some((self.nodes[new_idx].0, new_idx));
                    if new_idx >= instance_index + capacity - 1 {
                        instance_index = new_idx + 1 - capacity;
                        self.pos = (instance_index as f32) * ROW_HIGHT;
                    }
                } else if i.key_pressed(Key::Home) {
                    let selected_view_index: i64 = idx as i64 - instance_index as i64;
                    self.pos = 0.0;
                    instance_index = 0;
                    if selected_view_index >= 0 && selected_view_index < capacity as i64 {
                        let new_idx = selected_view_index as usize + instance_index;
                        self.selected_idx = Some((self.nodes[new_idx].0, new_idx));
                    }
                } else if i.key_pressed(Key::End) {
                    let selected_view_index: i64 = idx as i64 - instance_index as i64;
                    let needed_len = (self.nodes.len() + 2) as f32 * ROW_HIGHT;
                    self.pos = needed_len - a_height;
                    instance_index = (self.pos / ROW_HIGHT) as usize;
                    if selected_view_index >= 0 && selected_view_index < capacity as i64 {
                        let new_idx = selected_view_index as usize + instance_index;
                        self.selected_idx = Some((self.nodes[new_idx].0, new_idx));
                    }
                } else if i.key_pressed(Key::PageUp) {
                    let selected_view_index: i64 = idx as i64 - instance_index as i64;
                    self.pos -= a_height - ROW_HIGHT;
                    if self.pos < 0.0 {
                        self.pos = 0.0;
                    }
                    instance_index = (self.pos / ROW_HIGHT) as usize;
                    if selected_view_index >= 0 && selected_view_index < capacity as i64 {
                        let new_idx = selected_view_index as usize + instance_index;
                        self.selected_idx = Some((self.nodes[new_idx].0, new_idx));
                    }
                } else if i.key_pressed(Key::PageDown) {
                    let selected_view_index: i64 = idx as i64 - instance_index as i64;
                    let needed_len = (self.nodes.len() + 2) as f32 * ROW_HIGHT;
                    self.pos += a_height - ROW_HIGHT;
                    if self.pos > needed_len - a_height {
                        self.pos = needed_len - a_height;
                    }
                    instance_index = (self.pos / ROW_HIGHT) as usize;
                    if selected_view_index >= 0 && selected_view_index < capacity as i64 {
                        let new_idx = selected_view_index as usize + instance_index;
                        self.selected_idx = Some((self.nodes[new_idx].0, new_idx));
                    }
                } else if i.key_pressed(Key::Enter) {
                    *instance_action = NodeAction::BrowseNode(selected_iri);
                } else if i.modifiers.is_none() && i.key_pressed(Key::G) {
                    *instance_action = NodeAction::ShowVisual(selected_iri);
                }
            }

            if i.modifiers.is_none() && i.key_pressed(Key::Num1) {
                sort_idx = Some(0);
            } else if i.modifiers.is_none() && i.key_pressed(Key::Num2) {
                sort_idx = Some(1);
            } else if i.modifiers.is_none() && i.key_pressed(Key::Num3) {
                sort_idx = Some(2);
            } else if i.modifiers.is_none() && i.key_pressed(Key::Num4) {
                sort_idx = Some(3);
            } else if i.modifiers.is_none() && i.key_pressed(Key::Num5) {
                sort_idx = Some(4);
            } else if i.modifiers.is_none() && i.key_pressed(Key::Num6) {
                sort_idx = Some(5);
            } else if i.modifiers.is_none() && i.key_pressed(Key::Num7) {
                sort_idx = Some(6);
            } else if i.modifiers.is_none() && i.key_pressed(Key::Num8) {
                sort_idx = Some(7);
            } else if i.modifiers.is_none() && i.key_pressed(Key::Num9) {
                sort_idx = Some(8);
            }
        });
        if let Some(sort_idx) = sort_idx {
            if sort_idx < self.results.len() {
                table_action = StatisticsTableAction::SortResult(sort_idx);
            }
        }

        for ((_i, &label), width) in FIX_LABELS.iter().enumerate().zip(self.column_widths.iter()) {
            painter.text(
                available_rect.left_top() + Vec2::new(xpos, 0.0),
                egui::Align2::LEFT_TOP,
                label,
                font_id.clone(),
                ui.visuals().strong_text_color(),
            );
            xpos += width + COLUMN_GAP;
        }

        let label_context = LabelContext::new(layout_data.display_language, iri_display, &rfd_data.prefix_manager);
        for (result_idx, statistics_result) in self.results.iter().enumerate() {
            let top_left = available_rect.left_top() + Vec2::new(xpos, 0.0);
            let result_label = statistics_result.statistics_value().to_string();
            let result_rect = egui::Rect::from_min_size(top_left, Vec2::new(xpos + RESULT_WIDTH, ROW_HIGHT));
            let cell_hovered = if result_rect.contains(mouse_pos) {
                ui.output_mut(|o| o.cursor_icon = CursorIcon::PointingHand);
                true
            } else {
                false
            };
            text_wrapped(
                result_label.as_str(),
                RESULT_WIDTH,
                painter,
                top_left,
                cell_hovered,
                true,
                ui.visuals(),
            );
            if primary_down && result_rect.contains(mouse_pos) {
                table_action = StatisticsTableAction::SortResult(result_idx);
            }
            xpos += RESULT_WIDTH + COLUMN_GAP;
        }

        let mut ypos = ROW_HIGHT;
        let mut start_pos = instance_index;

        for node_index in instance_index..min(instance_index + capacity, self.nodes.len()) {
            let instance_index = &self.nodes[node_index];
            let node = rfd_data.node_data.get_node_by_index(instance_index.0);
            if let Some((node_iri, node)) = node {
                if matches!(self.selected_idx, Some((a, _)) if a == instance_index.0) {
                    painter.rect_filled(
                        Rect::from_min_size(
                            available_rect.left_top() + Vec2::new(0.0, ypos),
                            Vec2::new(available_width, ROW_HIGHT),
                        ),
                        0.0,
                        ui.visuals().selection.bg_fill,
                    );
                } else if start_pos % 2 == 0 {
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

                xpos = 0.0;

                // Draw fixed labels
                for ((i, _label), width) in FIX_LABELS.iter().enumerate().zip(self.column_widths.iter()) {
                    let label_top_left = available_rect.left_top() + Vec2::new(xpos, ypos);
                    let label_rect =
                        egui::Rect::from_min_size(label_top_left, Vec2::new(width + COLUMN_GAP, ROW_HIGHT));
                    if i == 0 {
                        let mut cell_hovered = false;
                        if label_rect.contains(mouse_pos) {
                            ui.output_mut(|o| o.cursor_icon = CursorIcon::PointingHand);
                            cell_hovered = true;
                        }
                        text_wrapped_link(
                            &rfd_data.prefix_manager.get_prefixed(node_iri),
                            *width,
                            painter,
                            label_top_left,
                            cell_hovered,
                            ui.visuals(),
                        );
                        if primary_clicked && label_rect.contains(mouse_pos) {
                            *instance_action = NodeAction::BrowseNode(instance_index.0);
                        } else if secondary_clicked && label_rect.contains(mouse_pos) {
                            *instance_action = NodeAction::ShowVisual(instance_index.0);
                        }
                    } else {
                        let label: Cow<'_, str> = if i == 1 {
                            Cow::Borrowed(node.node_label(
                                node_iri,
                                styles,
                                config.short_iri,
                                layout_data.display_language,
                                &rfd_data.node_data.indexers,
                            ))
                        } else {
                            let mut types_label = String::new();
                            node.types.iter().for_each(|type_index| {
                                if !types_label.is_empty() {
                                    types_label.push_str(", ");
                                }
                                types_label.push_str(
                                    rfd_data
                                        .node_data
                                        .type_display(*type_index, &label_context, &rfd_data.node_data.indexers)
                                        .as_str(),
                                );
                            });
                            Cow::Owned(types_label)
                        };
                        text_wrapped(
                            &label,
                            *width,
                            painter,
                            label_rect.left_top(),
                            false,
                            false,
                            ui.visuals(),
                        );
                    }
                    xpos += width + COLUMN_GAP;
                }

                // Draw results
                for result in self.results.iter() {
                    let value_str = result.get_value_str(node_index);
                    let cell_rect = egui::Rect::from_min_size(
                        available_rect.left_top() + Vec2::new(xpos, ypos),
                        Vec2::new(RESULT_WIDTH, ROW_HIGHT),
                    );
                    text_wrapped(
                        value_str.as_str(),
                        RESULT_WIDTH,
                        painter,
                        cell_rect.left_top(),
                        false,
                        false,
                        ui.visuals(),
                    );
                    xpos += RESULT_WIDTH + COLUMN_GAP;
                    if xpos > available_rect.width() {
                        break;
                    }
                }
                ypos += ROW_HIGHT;
            }
        }
        // Draw vertical lines
        xpos = 0.0;
        for width in self.column_widths.iter() {
            xpos += width + COLUMN_GAP;
            painter.line(
                [
                    Pos2::new(available_rect.left() + xpos - COLUMN_GAP, available_rect.top()),
                    Pos2::new(available_rect.left() + xpos - COLUMN_GAP, available_rect.top() + ypos),
                ]
                .to_vec(),
                Stroke::new(1.0, Color32::DARK_GRAY),
            );
        }
        for _result in self.results.iter() {
            xpos += RESULT_WIDTH;
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
        match table_action {
            StatisticsTableAction::None => {}
            StatisticsTableAction::SortResult(column_index) => {
                if column_index < self.results.len() {
                    let data_vec = self.results[column_index].get_data_vec();
                    let mut values_with_indices: Vec<_> =
                        data_vec.iter().enumerate().map(|(i, &v)| (v, i as u32)).collect();
                    values_with_indices.sort_unstable_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
                    self.reorder_in_place(&values_with_indices);
                    if let Some((selected_iri, pos)) = self.selected_idx {
                        if pos == 0 && !self.nodes.is_empty() {
                            self.selected_idx = Some((self.nodes[0].0, 0));
                        } else {
                            let new_pos = self.nodes.iter().position(|(iri, _)| *iri == selected_iri);
                            if let Some(new_pos) = new_pos {
                                self.selected_idx = Some((selected_iri, new_pos));
                            } else {
                                self.selected_idx = None;
                            }
                        }
                    }
                }
            }
        }
    }

    fn reorder_in_place<T: Clone>(&mut self, new_indexes: &[(T, u32)]) {
        // Reorder the values in place based on the new indexes
        let nodes_len = self.nodes.len();
        assert_eq!(nodes_len, new_indexes.len());
        let mut visited = fixedbitset::FixedBitSet::with_capacity(nodes_len);

        for i in 0..nodes_len {
            if visited[i] || new_indexes[i].1 as usize == i {
                continue;
            }
            let mut current = i;
            while !visited[current] {
                visited.insert(current);
                let next = new_indexes[current].1 as usize;
                if next != i {
                    self.nodes.swap(current, next);
                    for result in self.results.iter_mut() {
                        result.swap_values(current, next);
                    }
                }
                current = next;
            }
        }
    }

    fn export_csv_writer<W: io::Write>(&self, rdf_data: &RdfData, wtr: &mut csv::Writer<W>,
        iri_display: IriDisplay,
        styles: &GVisualizationStyle,
        lang_index: LangIndex,
        ) -> Result<(), Box<dyn std::error::Error>> {
        for title in vec!["iri", "label", "type"] {
            wtr.write_field(title)?;
        }
        let label_context = LabelContext::new(lang_index, iri_display, &rdf_data.prefix_manager);
        for result in self.results.iter() {
            wtr.write_field(result.statistics_value().to_string().as_str())?;
        }
        wtr.write_record(None::<&[u8]>)?;
        for (idx, (iri_index, _pos)) in self.nodes.iter().enumerate() {
            if let Some((iri,node)) = rdf_data.node_data.get_node_by_index(*iri_index) {
                let iri_ref: &str = iri;
                wtr.write_field(iri_ref)?;
                let label = node.node_label(
                    iri,
                    styles,
                    false,
                    lang_index,
                    &rdf_data.node_data.indexers,
                );
                wtr.write_field(label)?;
                let types = node.highest_priority_types(styles);
                if types.is_empty() {
                    wtr.write_field("")?;
                } else {
                    for type_index in types.iter() {
                         wtr.write_field(
                            rdf_data
                                .node_data
                                .type_display(*type_index, &label_context, &rdf_data.node_data.indexers)
                                .as_str(),
                        )?;
                        break;
                    }
                }
                for result in self.results.iter() {
                    wtr.write_field(result.get_value_str(idx).as_str())?;
                }
                wtr.write_record(None::<&[u8]>)?;
            }
        }
        wtr.flush()?;
        Ok(())
    }
}

