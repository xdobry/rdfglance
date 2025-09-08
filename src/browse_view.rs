use const_format::concatcp;
use egui::Key;
use egui_extras::{Column, StripBuilder, TableBuilder};

use crate::{
    GVisualizationStyle, NodeAction, RdfGlanceApp, RefSelection, UIState,
    nobject::{IriIndex, LabelContext, Literal, NObject, NodeData},
    style::{ICON_FILTER, ICON_GRAPH},
    uitools::primary_color,
};

#[derive(PartialEq)]
pub enum ReferenceAction {
    None,
    ShowNode(IriIndex),
    Filter(IriIndex, Vec<IriIndex>),
}

impl RdfGlanceApp {
    pub fn show_table(&mut self, ui: &mut egui::Ui) -> NodeAction {
        let mut action_type_index: NodeAction = NodeAction::None;
        ui.horizontal(|ui| {
            ui.horizontal(|ui| {
                if (ui.button("\u{2b05}").clicked()
                    || ui.input(|i| i.modifiers.alt && i.key_pressed(egui::Key::ArrowLeft)))
                    && self.nav_pos > 0
                {
                    self.nav_pos -= 1;
                    let object_iri_index = self.nav_history[self.nav_pos];
                    self.show_object_by_index(object_iri_index, false);
                }
                if (ui.button("\u{27a1}").clicked()
                    || ui.input(|i| i.modifiers.alt && i.key_pressed(egui::Key::ArrowRight)))
                    && self.nav_pos < self.nav_history.len() - 1
                {
                    self.nav_pos += 1;
                    let object_iri_index = self.nav_history[self.nav_pos];
                    self.show_object_by_index(object_iri_index, false);
                }
            });
            egui::TextEdit::singleline(&mut self.object_iri).show(ui);
            if ui.button("Load Object").clicked() {
                println!("load object: {}", self.object_iri);
                self.show_object();
            }
        });
        let mut node_to_click: ReferenceAction = ReferenceAction::None;
        if let Some(current_iri_index) = self.current_iri {
            if let Ok(rdf_data) = self.rdf_data.read() {
                let current_node = rdf_data.node_data.get_node_by_index(current_iri_index);
                if let Some((iri, current_node)) = current_node {
                    let full_iri = rdf_data.prefix_manager.get_full_opt(iri).unwrap_or(iri.clone());
                    ui.horizontal(|ui| {
                        ui.strong("full iri:");
                        ui.label(full_iri);
                    });
                    let button_text = egui::RichText::new(concatcp!(ICON_GRAPH, " See in Visual Graph (G)")).size(16.0);
                    let nav_but = egui::Button::new(button_text).fill(primary_color(ui.visuals()));
                    let b_resp = ui.add(nav_but);
                    if b_resp.clicked() || ui.input(|i| i.key_pressed(egui::Key::G)) {
                        action_type_index = NodeAction::ShowVisual(current_iri_index);
                    }
                    b_resp.on_hover_text("This will add the node to the visual graph and switch to visual graph view. The node will be selected.");
                    let label_context = LabelContext::new(
                        self.ui_state.display_language,
                        self.persistent_data.config_data.iri_display,
                        &rdf_data.prefix_manager,
                    );
                    ui.horizontal(|ui| {
                        ui.strong("types:");
                        for type_index in &current_node.types {
                            let type_label = rdf_data.node_data.type_display(
                                *type_index,
                                &label_context,
                                &rdf_data.node_data.indexers,
                            );
                            if ui.button(type_label.as_str()).clicked() {
                                action_type_index = NodeAction::ShowType(*type_index);
                            }
                        }
                    });
                    if current_node.properties.is_empty() {
                        let h = (ui.available_height() - 40.0).max(300.0);
                        node_to_click = show_refs_table(
                            ui,
                            current_node,
                            &rdf_data.node_data,
                            &self.visualization_style,
                            &self.ui_state,
                            h,
                            &label_context,
                            &mut self.ref_selection,
                        );
                    } else {
                        egui::ScrollArea::vertical()
                            .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
                            .show(ui, |ui| {
                                ui.heading("Data Property");
                                let avialable_width = (ui.available_width() - 100.0).max(200.0);
                                egui::Grid::new("properties")
                                    .striped(true)
                                    .max_col_width(avialable_width)
                                    .show(ui, |ui| {
                                        for (predicate_index, prop_value) in &current_node.properties {
                                            if self.persistent_data.config_data.suppress_other_language_data {
                                                if let Literal::LangString(lang, _) = prop_value {
                                                    if *lang != self.ui_state.display_language {
                                                        if *lang == 0 && self.ui_state.display_language != 0 {
                                                            // it is fallback language so display if reall language could not be found
                                                            let mut found = false;
                                                            for (predicate_index2, prop_value2) in
                                                                &current_node.properties
                                                            {
                                                                if predicate_index2 == predicate_index
                                                                    && prop_value2 != prop_value
                                                                {
                                                                    if let Literal::LangString(lang, _) = prop_value2 {
                                                                        if *lang == self.ui_state.display_language {
                                                                            found = true;
                                                                            break;
                                                                        }
                                                                    }
                                                                }
                                                            }
                                                            if found {
                                                                continue;
                                                            }
                                                        } else {
                                                            continue;
                                                        }
                                                    }
                                                }
                                            }
                                            let predicate_label = rdf_data.node_data.predicate_display(
                                                *predicate_index,
                                                &label_context,
                                                &rdf_data.node_data.indexers,
                                            );
                                            ui.label(predicate_label.as_str());
                                            ui.label(prop_value.as_str_ref(&rdf_data.node_data.indexers));
                                            ui.end_row();
                                        }
                                    });
                                let h = (ui.available_height() - 40.0).max(300.0);
                                node_to_click = show_refs_table(
                                    ui,
                                    current_node,
                                    &rdf_data.node_data,
                                    &self.visualization_style,
                                    &self.ui_state,
                                    h,
                                    &label_context,
                                    &mut self.ref_selection,
                                );
                            });
                    }
                }
            }
        } else {
            ui.heading("No node selected. Please enter Node IRI or click node link from table or graph view.");
        }
        match node_to_click {
            ReferenceAction::None => {}
            ReferenceAction::ShowNode(iri_index) => {
                action_type_index = NodeAction::BrowseNode(iri_index);
            }
            ReferenceAction::Filter(node_type, instances) => {
                action_type_index = NodeAction::ShowTypeInstances(node_type, instances);
            }
        }
        action_type_index
    }
}

pub fn show_refs_table(
    ui: &mut egui::Ui,
    current_node: &NObject,
    node_data: &NodeData,
    color_cache: &GVisualizationStyle,
    layout_data: &UIState,
    h: f32,
    label_context: &LabelContext,
    ref_selection: &mut RefSelection,
) -> ReferenceAction {
    let mut node_to_click: ReferenceAction = ReferenceAction::None;
    if !matches!(ref_selection, RefSelection::None) {
        ui.input(|i| {
            if i.key_pressed(Key::ArrowDown) {
                ref_selection.move_down(&current_node);
            } else if i.key_pressed(Key::ArrowUp) {
                ref_selection.move_up();
            } else if i.key_pressed(Key::ArrowRight) {
                ref_selection.move_right(&current_node);
            } else if i.key_pressed(Key::ArrowLeft) {
                ref_selection.move_left(&current_node);
            }
        });
    }
    StripBuilder::new(ui)
        .size(egui_extras::Size::exact(600.0))
        .size(egui_extras::Size::exact(600.0)) // Two resizable panels with equal initial width
        .horizontal(|mut strip| {
            strip.cell(|ui| {
                let ref_result = show_references(
                    node_data,
                    color_cache,
                    ui,
                    "References",
                    &current_node.references,
                    layout_data,
                    h,
                    "ref",
                    label_context,
                    ref_selection.ref_index(false),
                );
                if ref_result != ReferenceAction::None {
                    node_to_click = ref_result;
                }
            });
            strip.cell(|ui| {
                ui.push_id("ref2", |ui| {
                    let ref_result = show_references(
                        node_data,
                        color_cache,
                        ui,
                        "Referenced by",
                        &current_node.reverse_references,
                        layout_data,
                        h,
                        "ref_by",
                        label_context,
                        ref_selection.ref_index(true),
                    );
                    if ref_result != ReferenceAction::None {
                        node_to_click = ref_result;
                    }
                });
            });
        });
    node_to_click
}

pub fn show_references(
    node_data: &NodeData,
    color_cache: &GVisualizationStyle,
    ui: &mut egui::Ui,
    label: &str,
    references: &[(IriIndex, IriIndex)],
    layout_data: &UIState,
    height: f32,
    id_salt: &str,
    label_context: &LabelContext,
    selected_idx: Option<usize>,
) -> ReferenceAction {
    let mut node_to_click: ReferenceAction = ReferenceAction::None;
    if !references.is_empty() {
        ui.heading(label);
        let mut has_enter = false;
        let mut has_find = false;
        ui.input(|i| {
            if i.key_pressed(Key::Enter) {
                has_enter = true;
            } else if i.key_pressed(Key::F) {
                has_find = true;
            }
        });
        let text_height = egui::TextStyle::Body
            .resolve(ui.style())
            .size
            .max(ui.spacing().interact_size.y);

        let table: TableBuilder<'_> = TableBuilder::new(ui)
            .striped(true)
            .id_salt(id_salt)
            .resizable(true)
            .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
            .column(Column::exact(100.0).at_least(30.0).at_most(300.0))
            .column(Column::exact(200.0).at_least(30.0).at_most(300.0))
            .column(Column::exact(100.0).at_least(30.0).at_most(300.0))
            .column(Column::exact(100.0).at_least(30.0).at_most(300.0))
            .column(Column::exact(20.0))
            .min_scrolled_height(height)
            .max_scroll_height(height);


        table
            .header(20.0, |mut header| {
                header.col(|ui| {
                    ui.strong("Predicate");
                });
                header.col(|ui| {
                    ui.strong("Iri");
                });
                header.col(|ui| {
                    ui.strong("Type");
                });
                header.col(|ui| {
                    ui.strong("Label");
                });
                header.col(|ui| {
                    ui.strong("F");
                });
            })
            .body(|body| {
                body.rows(text_height, references.len(), |mut row| {
                    let (predicate_index, ref_index) = references.get(row.index()).unwrap();
                    row.col(|ui| {
                        let predicate_label =
                            node_data.predicate_display(*predicate_index, label_context, &node_data.indexers);
                        ui.label(predicate_label.as_str());
                    });
                    let mut row_selected = false;
                    if selected_idx == Some(row.index()) {
                        row.set_selected(true);
                        row_selected = true;
                        if has_enter {
                            node_to_click = ReferenceAction::ShowNode(*ref_index);
                        }
                    }
                    if let Some((ref_iri, ref_node)) = node_data.get_node_by_index(*ref_index) {
                        row.col(|ui| {
                            if ui.link(ref_iri).clicked() {
                                node_to_click = ReferenceAction::ShowNode(*ref_index);
                            }
                        });
                        row.col(|ui| {
                            let mut types_label: String = String::new();
                            ref_node.types.iter().for_each(|type_index| {
                                if !types_label.is_empty() {
                                    types_label.push_str(", ");
                                }
                                types_label.push_str(
                                    node_data
                                        .type_display(*type_index, label_context, &node_data.indexers)
                                        .as_str(),
                                );
                            });
                            ui.label(types_label);
                        });
                        row.col(|ui| {
                            let label =
                                ref_node.node_label_opt(color_cache, layout_data.display_language, &node_data.indexers);
                            if let Some(label) = label {
                                ui.label(label);
                            }
                        });
                        row.col(|ui| {
                            if ref_node.types.is_empty() {
                                ui.label("?");
                            } else {
                                if (row_selected && has_find) || ui.button(ICON_FILTER).clicked() {
                                    let node_type = ref_node.types.first().unwrap();
                                    // collected all instance of same predicate and type
                                    let instances: Vec<IriIndex> = references
                                        .iter()
                                        .filter(|(pred, iref_index)| {
                                            if pred == predicate_index {
                                                if let Some((_iri, nobject)) = node_data.get_node_by_index(*iref_index)
                                                {
                                                    nobject.types.contains(node_type)
                                                } else {
                                                    false
                                                }
                                            } else {
                                                false
                                            }
                                        })
                                        .map(|(_pred, iref_index)| *iref_index)
                                        .collect();
                                    node_to_click = ReferenceAction::Filter(*node_type, instances);
                                }
                            }
                        });
                    }
                });
            });
    }
    node_to_click
}
