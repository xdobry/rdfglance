use crate::{
    nobject::{IriIndex, Literal, NodeData}, rdfwrap, ColorCache, LayoutData, NodeAction, VisualRdfApp
};

impl VisualRdfApp {
    pub fn show_table(&mut self, ui: &mut egui::Ui) -> NodeAction {
        let mut action_type_index: NodeAction = NodeAction::None;
        ui.horizontal(|ui| {
            ui.horizontal(|ui| {
                if ui.button("<").clicked() && self.nav_pos > 0 {
                    self.nav_pos -= 1;
                    let object_iri_index = self.nav_history[self.nav_pos];
                    self.show_object_by_index(object_iri_index, false);
                }
                if ui.button(">").clicked() && self.nav_pos < self.nav_history.len() - 1 {
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
        let mut node_to_click: Option<IriIndex> = None;
        if let Some(current_iri_index) = self.current_iri {
            let current_node = self.node_data.get_node_by_index(current_iri_index);
            if let Some((iri,current_node)) = current_node {
                ui.label(iri);
                if ui.button("Visual Graph").clicked() {
                    action_type_index = NodeAction::ShowVisual(current_iri_index);
                }
                for type_index in &current_node.types {
                    if ui
                        .button(
                            self.rdfwrap
                                .iri2label(self.node_data.get_type(*type_index).unwrap()),
                        )
                        .clicked()
                    {
                        action_type_index = NodeAction::ShowType(*type_index);
                    }
                }
                egui::ScrollArea::vertical().show(ui, |ui| {
                    if !current_node.properties.is_empty() {
                        ui.heading("Data Property");
                        let avialable_width = (ui.available_width()-100.0).max(200.0);
                        egui::Grid::new("properties")
                            .striped(true)
                            .max_col_width(avialable_width)
                            .show(ui, |ui| {
                            for (predicate_index, prop_value) in &current_node.properties {
                                if self.persistent_data.config_data.supress_other_language_data {
                                    if let Literal::LangString(lang, _) = prop_value {
                                        if *lang != self.layout_data.display_language {
                                            if *lang == 0 && self.layout_data.display_language != 0 {
                                                // it is fallback language so display if reall language could not be found
                                                let mut found = false;
                                                for (predicate_index2, prop_value2) in &current_node.properties {
                                                    if predicate_index2 == predicate_index && prop_value2 != prop_value {
                                                        if let Literal::LangString(lang, _) = prop_value2 {
                                                            if *lang == self.layout_data.display_language {
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
                                ui.label(self.rdfwrap.iri2label(
                                    self.node_data.get_predicate(*predicate_index).unwrap(),
                                ));
                                ui.label(prop_value.as_ref());
                                ui.end_row();
                            }
                        });
                    }
                    // I could not separate new self method because the borrow checker
                    // so new method that need to pass all needed substructures
                    if let Some(node_index) = show_references(
                        &self.node_data,
                        &mut *self.rdfwrap,
                        &self.color_cache,
                        ui,
                        "References",
                        &current_node.references,
                        &self.layout_data,
                    ) {
                        node_to_click = Some(node_index);
                    }
                    if let Some(node_index) = show_references(
                        &self.node_data,
                        &mut *self.rdfwrap,
                        &self.color_cache,
                        ui,
                        "Referenced by",
                        &current_node.reverse_references,
                        &self.layout_data,
                    ) {
                        node_to_click = Some(node_index);
                    }
                });
            }
        }
        if let Some(node_to_click) = node_to_click {
            self.show_object_by_index(node_to_click, true);
        }
        return action_type_index;
    }
}

pub fn show_references(
    node_data: &NodeData,
    rdfwrap: &mut dyn rdfwrap::RDFAdapter,
    color_cache: &ColorCache,
    ui: &mut egui::Ui,
    label: &str,
    references: &Vec<(IriIndex, IriIndex)>,
    layout_data: &LayoutData,
) -> Option<IriIndex> {
    let mut node_to_click: Option<IriIndex> = None;
    if !references.is_empty() {
        ui.heading(label);
        for (predicate_index, ref_index) in references.iter() {
            ui.horizontal(|ui| {
                ui.label(rdfwrap.iri2label(node_data.get_predicate(*predicate_index).unwrap()));
                node_data.get_node_by_index(*ref_index).map(|(ref_iri,ref_node)| {
                    if ui.button(ref_iri).clicked() {
                        node_to_click = Some(*ref_index);
                    }
                    ref_node.types.iter().for_each(|type_index| {
                        ui.label(rdfwrap.iri2label(node_data.get_type(*type_index).unwrap()));
                    });
                    let label = ref_node.node_label_opt(&color_cache.label_predicate,layout_data.display_language);
                    if let Some(label) = label {
                        ui.label(label);
                    }
                });
            });
        }
    }
    return node_to_click;
}
