use std::{borrow::Cow, cmp::min, io};

use const_format::concatcp;
use egui::{Color32, CursorIcon, Key, Pos2, Rect, Sense, Stroke, Vec2};
use egui_extras::StripBuilder;
use oxrdf::vocab::rdf;

use crate::{
    config::{Config, IriDisplay}, graph_algorithms::{GraphAlgorithm, StatisticValue}, nobject::{IriIndex, LabelContext, LangIndex}, style::ICON_EXPORT, table_view::{text_wrapped, text_wrapped_link}, uitools::ScrollBar, GVisualizationStyle, NodeAction, RdfData, RdfGlanceApp, UIState
};

const ROW_HIGHT: f32 = 17.0;
const COLUMN_GAP: f32 = 2.0;
const IRI_WIDTH: f32 = 300.0;
const RESULT_WIDTH: f32 = 100.0;

const FIX_LABELS: [&str; 3] = ["iri", "label", "type"];

pub type NodePosition = u32;

pub struct StatisticsData {
    // Stores the node iri index and its position in SortedNodeLayout structure that is used for graph algorithms
    pub nodes: Vec<(IriIndex, NodePosition)>,
    pub results: Vec<StatisticsResult>,
    pub pos: f32,
    pub drag_pos: Option<f32>,
    pub column_widths: [f32; 3],
    pub data_epoch: u32,
    pub selected_idx: Option<(IriIndex, usize)>,
}

impl Default for StatisticsData {
    fn default() -> Self {
        Self {
            nodes: Vec::new(),
            results: Vec::new(),
            pos: 0.0,
            drag_pos: None,
            // Default widths for iri, label, and type
            column_widths: [IRI_WIDTH, 200.0, 200.0],
            data_epoch: 0,
            selected_idx: None,
        }
    }
}
pub struct StatisticsResult {
    values: Vec<f32>,
    statistic_value: StatisticValue,
}

enum StatisticsTableAction {
    None,
    SortResult(usize),
}

impl StatisticsResult {
    pub fn new_for_alg(values: Vec<f32>, alg: GraphAlgorithm) -> Self {
        Self {
            values,
            statistic_value: alg.get_statistics_values()[0],
        }
    }
    pub fn new_for_values(values: Vec<f32>, statistic_value: StatisticValue) -> Self {
        Self {
            values,
            statistic_value: statistic_value,
        }
    }
    pub fn statistics_value(&self) -> StatisticValue {
        self.statistic_value
    }
    pub fn get_data_vec(&self) -> &Vec<f32> {
        &self.values
    }
    pub fn get_value_str(&self, node_index: usize) -> String {
        let data_vec = self.get_data_vec();
        if node_index < data_vec.len() {
            format!("{:.4}", data_vec[node_index])
        } else {
            "N/A".to_string()
        }
    }
    pub fn swap_values(&mut self, i: usize, j: usize) {
        self.values.swap(i, j);
    }
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
                            use crate::uitools::web_download;
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

pub fn distribute_to_zoom_layers(values: &Vec<f32>) -> Vec<u8> {
    let mut values_with_indices: Vec<_> = values.iter().enumerate().map(|(i, &v)| (v, i)).collect();
    values_with_indices.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
    let mut layers = vec![0u8; values.len()];
    let data_len = values.len();
    let a = if data_len < 12 { 1 } else { 4 };
    if let Ok(q) = compute_q(values.len() as f64, a as f64, 10, 1e-10, 1000) {
        let q = if q < 1.0 { 1.0 } else { q };
        let ranges: Vec<(usize, usize)> = {
            let mut ranges = Vec::new();
            let mut pos = 0;
            let mut start = a as f64;
            for idx in 0..10 {
                let end = if idx == 9 {
                    data_len - 1
                } else {
                    (pos as f64 + start + 0.5) as usize - 1
                };
                if end >= data_len - 1 {
                    ranges.push((pos as usize, data_len - 1));
                    break;
                } else {
                    ranges.push((pos as usize, end));
                }
                pos = end + 1;
                start *= q;
            }
            ranges
        };
        let mut corrected_ranges: Vec<(usize, usize)> = Vec::with_capacity(ranges.len());
        let mut next_start: isize = -1;
        for (idx, &(mut start, mut end)) in ranges.iter().enumerate() {
            if next_start >= 0 {
                start = next_start as usize;
            }
            next_start = -1;
            if end < start {
                end = start;
                next_start = (end + 1) as isize;
                if next_start as usize > data_len - 1 {
                    break;
                }
            }
            if idx > 0 {
                let (last_start, mut last_end) = corrected_ranges.last().copied().unwrap();

                // Compare values at the current start and previous range's end
                if values_with_indices[start].0 == values_with_indices[last_end].0 {
                    if values_with_indices[start].0 == values_with_indices[end].0 {
                        if values_with_indices[last_start].0 == values_with_indices[last_end].0 {
                            next_start = (end + 1) as isize;
                            // Extend previous range
                            corrected_ranges.last_mut().unwrap().1 = end;
                            continue;
                        } else {
                            // shrink previous range from the end
                            while values_with_indices[last_end].0 == values_with_indices[start].0 && last_end > last_start {
                                last_end -= 1;
                            }
                            corrected_ranges.last_mut().unwrap().1 = last_end;
                            start = last_end + 1;
                        }
                    } else {
                        // shift start forward to skip duplicates
                        while values_with_indices[last_end].0 == values_with_indices[start].0 && start <= end {
                            start += 1;
                        }
                        corrected_ranges.last_mut().unwrap().1 = start - 1;
                    }
                }
            }
            corrected_ranges.push((start, end));
        }

        for (layer, (start, end)) in corrected_ranges.iter().enumerate() {
            // println!("Layer {}: {} - {}", layer + 1, start, end);
            for (_value, index) in values_with_indices.iter().skip(*start).take(end - start + 1) {
                layers[*index] = 10 - layer as u8;
            }
        }
    }
    layers
}

fn compute_q(sum: f64, a: f64, n: usize, tol: f64, max_iter: usize) -> Result<f64, String> {
    if n == 0 {
        panic!("n must be > 0");
    }
    if (sum - a).abs() < tol {
        return Ok(1.0); // Spezialfall: Summe = erstes Glied -> q=1
    }

    // f(q) = a*(1 - q^n)/(1 - q) - S
    let f = |q: f64| -> f64 {
        if (q - 1.0).abs() < tol {
            // Limes q -> 1
            a * (n as f64) - sum
        } else {
            a * (1.0 - q.powi(n as i32)) / (1.0 - q) - sum
        }
    };

    let mut low = 0.0_f64;
    let mut high = f64::max(2.0, sum / a);

    for _ in 0..max_iter {
        let mid = (low + high) / 2.0;
        let val = f(mid);
        if val.abs() < tol {
            return Ok(mid);
        }
        if f(low) * val < 0.0 {
            high = mid;
        } else {
            low = mid;
        }
    }
    Err("No solution found max_iter reached".to_string())
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap};

    use crate::statistics::*;
    use rand::{seq::SliceRandom, Rng};
    
    fn gen_test_data(desc: &Vec<(u32,f32,f32)>) -> Vec<f32> {
        let mut res = Vec::new();
        let mut start = 1.0;
        for (count ,d,start_diff) in desc {
            start -= start_diff;
            for _ in 0..*count {
                start -= *d;
                res.push(start);
            }
        }
        res
    }

    fn prepare_dist(desc: &Vec<(u32,f32,f32)>) -> (Vec<u8>,BTreeMap<u8,u32>,u8,u8) {
        let test_data = gen_test_data(desc);
        prepare_dist_data(&test_data)
    }

    fn prepare_dist_data(test_data: &Vec<f32>) -> (Vec<u8>,BTreeMap<u8,u32>,u8,u8) {
        let layers = distribute_to_zoom_layers(&test_data);
        assert_eq!(test_data.len(), layers.len());
        let mut max = 0;
        let mut min = 10;
        let mut hist: BTreeMap<u8,u32> = BTreeMap::new();
        for (l,v) in layers.iter().zip(test_data.iter()) {
            // println!("Layer: {} {}", l,v);
            if *l > max {
                max = *l;
            }
            if *l < min {
                min = *l;
            }
            *hist.entry(*l).or_insert(0) += 1;
        }
        (layers, hist, min, max)
    }
    
    #[test]
    fn test_distriute_to_zoo_layers() {
        // cargo test test_distriute_to_zoo_layers -- --nocapture

        let data = vec![
            (10, 0.0, 0.0),
            (2, 0.01, 0.0),
            (3, 0.01, 0.0),
            (4, 0.01, 0.0),
            (5, 0.01, 0.0),
        ];
        let (_layers, hist, min, max) = prepare_dist(&data);
        assert_eq!(max, 10);
        assert_eq!(min, 6);
        assert_eq!(5, hist.len());

        let mut data = Vec::new();
        for _ in 0..1000 {
            data.push((1, 0.0001, 0.0));
        }
        let (layers, hist, min, max) = prepare_dist(&data);
        assert_eq!(max, 10);
        assert_eq!(min, 1);
        assert_eq!(hist.get(&10), Some(&4));
        assert_eq!(10, hist.len());
        layers.windows(2).for_each(|w| {
            assert!(w[0] >= w[1]);
        });

        let data = vec![(5,0.0, 0.0)];
        let (_layers, hist, min, max) = prepare_dist(&data);
        assert_eq!(max, 10);
        assert_eq!(min, 10);
        assert_eq!(1, hist.len());

        let data = vec![(1,0.0, 0.0),(5,0.0, 0.1)];
        let (layers, hist, min, max) = prepare_dist(&data);
        assert_eq!(max, 10);
        assert_eq!(min, 9);
        assert_eq!(2, hist.len());
        assert_eq!(hist.get(&10), Some(&1));
        assert_eq!(hist.get(&9), Some(&5));
        layers.windows(2).for_each(|w| {
            assert!(w[0] >= w[1]);
        });

        let data = vec![(5,0.0, 0.0),(1,0.1, 0.0)];
        let (layers, hist, min, max) = prepare_dist(&data);
        assert_eq!(max, 10);
        assert_eq!(min, 9);
        assert_eq!(2, hist.len());
        assert_eq!(hist.get(&10), Some(&5));
        assert_eq!(hist.get(&9), Some(&1));
        layers.windows(2).for_each(|w| {
            assert!(w[0] >= w[1]);
        });

        let mut data = Vec::new();
        let mut rng = rand::rng();
        for _ in 0..1000 {
            data.push((rng.random_range(1..5), 0.0001, 0.0));
        }
        let mut test_data = gen_test_data(&data);
        test_data.shuffle(&mut rng);
        let (layers, hist, min, max) = prepare_dist_data(&test_data);
        assert_eq!(max, 10);
        assert_eq!(min, 1);
        assert_eq!(10, hist.len());
        let mut test_data_with_index = test_data.iter().enumerate().map(|(i,v)| (v,i)).collect::<Vec<(&f32,usize)>>();
        test_data_with_index.sort_by(|a,b| b.0.partial_cmp(a.0).unwrap());
        let layers_sorted = test_data_with_index.iter().map(|(_v,i)| layers[*i]).collect::<Vec<u8>>();
        layers_sorted.windows(2).for_each(|w| {
            assert!(w[0] >= w[1]);
        });

    }
}