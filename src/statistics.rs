use std::cmp::min;

use egui::{Color32, CursorIcon, Pos2, Rect, Sense, Stroke, Vec2};
use egui_extras::StripBuilder;

use crate::{config::IriDisplay, graph_algorithms::GraphAlgorithm, nobject::{IriIndex, LabelContext, NodeData}, prefix_manager::PrefixManager, table_view::{text_wrapped, text_wrapped_link}, uitools::ScrollBar, GVisualizationStyle, NodeAction, RdfData, RdfGlanceApp, UIState};

const ROW_HIGHT: f32 = 17.0;
const CHAR_WIDTH: f32 = 8.0;
const COLUMN_GAP: f32 = 2.0;
const IRI_WIDTH: f32 = 300.0;
const RESULT_WIDTH: f32 = 50.0;

pub struct StatisticsData {
    pub nodes: Vec<IriIndex>,
    pub results: Vec<StatisticsResult>,
    pub pos: f32,
    pub drag_pos: Option<f32>,
    pub iri_width: f32,
    pub label_width: f32,
    pub type_width: f32,
}

impl Default for StatisticsData {
    fn default() -> Self {
        Self {
            nodes: Vec::new(),
            results: Vec::new(),
            pos: 0.0,
            drag_pos: None,
            iri_width: IRI_WIDTH,
            label_width: 200.0,
            type_width: 200.0,
        }
    }
}  
pub enum StatisticsResult {
    BetweennessCentrality(Vec<f32>),
}

impl StatisticsResult {
    pub fn graph_algorithm(&self) -> GraphAlgorithm {
        match self {
            StatisticsResult::BetweennessCentrality(_) => GraphAlgorithm::BetweennessCentrality,
        }
    }
    pub fn get_value_str(&self, node_index: usize) -> String {
        match self {
            StatisticsResult::BetweennessCentrality(values) => {
                if node_index < values.len() {
                    format!("{:.4}", values[node_index])
                } else {
                    "N/A".to_string()
                }
            }
        }
    }
} 

impl RdfGlanceApp {
    pub fn show_statistics(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) -> NodeAction {
        if self.statistics_data.is_some() {
            ui.label("Statistics Data Available");
            self.show_statistics_data(ctx, ui)
        } else {
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
                                &mut self.ui_state,
                                self.persistent_data.config_data.iri_display,
                                &self.visualization_style
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
        &self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        rfd_data: &mut RdfData,
        instance_action: &mut NodeAction,
        layout_data: &UIState,
        iri_display: IriDisplay,
        styles: &GVisualizationStyle
    ) {
        let instance_index = (self.pos / ROW_HIGHT) as usize;
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

        painter.rect_filled(
            Rect::from_min_size(available_rect.left_top(), Vec2::new(available_width, ROW_HIGHT)),
            0.0,
            ui.visuals().code_bg_color,
        );

        painter.text(
            available_rect.left_top(),
            egui::Align2::LEFT_TOP,
            "iri",
            font_id.clone(),
            ui.visuals().strong_text_color(),
        );


        let mut primary_down = false;
        ctx.input(|input| {
            if input.pointer.button_pressed(egui::PointerButton::Primary) {
                primary_down = true;
            }
        });
        painter.text(
            available_rect.left_top() + Vec2::new(self.iri_width, 0.0),
            egui::Align2::LEFT_TOP,
            "label",
            font_id.clone(),
            ui.visuals().strong_text_color(),
        );
        painter.text(
            available_rect.left_top() + Vec2::new(self.iri_width+self.label_width, 0.0),
            egui::Align2::LEFT_TOP,
            "type",
            font_id.clone(),
            ui.visuals().strong_text_color(),
        );

        xpos += self.iri_width + self.type_width + self.label_width;

        let label_context = LabelContext::new(layout_data.display_language, iri_display, &rfd_data.prefix_manager);
        for statistics_result in self
            .results
            .iter()
        {
            let top_left = available_rect.left_top() + Vec2::new(xpos, 0.0);
            let result_label = statistics_result.graph_algorithm().to_string();
            text_wrapped(result_label.as_str(),RESULT_WIDTH, painter, top_left, false, true, ui.visuals());
            xpos += RESULT_WIDTH + COLUMN_GAP;
        }

        let mut ypos = ROW_HIGHT;
        let mut start_pos = instance_index;

        for node_index in instance_index..min(instance_index + capacity, self.nodes.len()) {
            let instance_index = &self.nodes[node_index];
            let node = rfd_data.node_data.get_node_by_index(*instance_index);
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

                let iri_top_left = available_rect.left_top() + Vec2::new(0.0, ypos);

                let iri_rect = egui::Rect::from_min_size(
                    iri_top_left,
                    Vec2::new(self.iri_width, ROW_HIGHT),
                );

                let mut cell_hovered = false;
                if iri_rect.contains(mouse_pos) {
                    ui.output_mut(|o| o.cursor_icon = CursorIcon::PointingHand);
                    cell_hovered = true;
                }

                text_wrapped_link(
                    &rfd_data.prefix_manager.get_prefixed(node_iri),
                    self.iri_width,
                    painter,
                    iri_top_left,
                    cell_hovered,
                    ui.visuals()
                );



                let mut xpos = self.iri_width + self.type_width + self.label_width;

                if primary_clicked && iri_rect.contains(mouse_pos) {
                    *instance_action = NodeAction::BrowseNode(*instance_index);
                } else if secondary_clicked && iri_rect.contains(mouse_pos) {
                    *instance_action = NodeAction::ShowVisual(*instance_index);
                }
                let node_label = node.node_label(node_iri, styles, layout_data.short_iri, layout_data.display_language, &rfd_data.node_data.indexers);
                let label_rect = egui::Rect::from_min_size(
                    available_rect.left_top() + Vec2::new(self.iri_width, ypos),
                    Vec2::new(self.type_width, ROW_HIGHT),
                );
                text_wrapped(node_label, self.label_width, painter, label_rect.left_top(), false, false, ui.visuals());

                let type_rect = egui::Rect::from_min_size(
                    available_rect.left_top() + Vec2::new(self.iri_width+self.label_width, ypos),
                    Vec2::new(self.type_width, ROW_HIGHT),
                );
                let mut types_label: String = String::new();
                node.types.iter().for_each(|type_index| {
                    if !types_label.is_empty() {
                        types_label.push_str(", ");
                    }
                    types_label.push_str(
                        rfd_data.node_data
                            .type_display(*type_index, &label_context, &rfd_data.node_data.indexers)
                            .as_str(),
                    );
                });
                text_wrapped(&types_label, self.type_width, painter, type_rect.left_top(), false, false, ui.visuals());              


                for result in self.results.iter() {
                    let value_str = result.get_value_str(node_index);
                    let cell_rect = egui::Rect::from_min_size(
                        available_rect.left_top() + Vec2::new(xpos, ypos),
                        Vec2::new(RESULT_WIDTH, ROW_HIGHT),
                    );
                    let mut cell_hovered = false;
                    if cell_rect.contains(mouse_pos) {
                        cell_hovered = true;
                    }
                    text_wrapped(value_str.as_str(), RESULT_WIDTH, painter, cell_rect.left_top(), cell_hovered, false, ui.visuals());
                    xpos += RESULT_WIDTH + COLUMN_GAP;
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
                    available_rect.left() + self.iri_width - COLUMN_GAP,
                    available_rect.top(),
                ),
                Pos2::new(
                    available_rect.left() + self.iri_width - COLUMN_GAP,
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
                        + self.iri_width
                        + self.label_width
                        + -COLUMN_GAP,
                    available_rect.top(),
                ),
                Pos2::new(
                    available_rect.left() + self.iri_width + self.label_width
                        - COLUMN_GAP,
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
                        + self.iri_width
                        + self.type_width
                        + self.label_width
                        + -COLUMN_GAP,
                    available_rect.top(),
                ),
                Pos2::new(
                    available_rect.left() + self.iri_width + self.type_width + self.label_width
                        - COLUMN_GAP,
                    available_rect.top() + ypos,
                ),
            ]
            .to_vec(),
            Stroke::new(1.0, Color32::DARK_GRAY),
        );
        xpos = self.iri_width + self.type_width + self.label_width;
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
    } 
}