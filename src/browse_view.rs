use const_format::concatcp;
use egui_extras::{Column, StripBuilder, TableBuilder};

use crate::{
    nobject::{IriIndex, LabelContext, Literal, NObject, NodeData}, rdfwrap, style::ICON_GRAPH, ColorCache, LayoutData, NodeAction, RdfGlanceApp
};

impl RdfGlanceApp {
    pub fn show_table(&mut self, ui: &mut egui::Ui) -> NodeAction {
        let mut action_type_index: NodeAction = NodeAction::None;
        ui.horizontal(|ui| {
            ui.horizontal(|ui| {
                if ui.button("\u{2b05}").clicked() && self.nav_pos > 0 {
                    self.nav_pos -= 1;
                    let object_iri_index = self.nav_history[self.nav_pos];
                    self.show_object_by_index(object_iri_index, false);
                }
                if ui.button("\u{27a1}").clicked() && self.nav_pos < self.nav_history.len() - 1 {
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
            if let Some((iri, current_node)) = current_node {
                let full_iri = self.prefix_manager.get_full_opt(iri).unwrap_or(iri.to_owned());
                ui.horizontal(|ui|{
                    ui.strong("full iri:");
                    ui.label(full_iri);
                });
                let button_text = egui::RichText::new(concatcp!(ICON_GRAPH," See in Visual Graph")).size(16.0);
                let nav_but = egui::Button::new(button_text).fill(egui::Color32::LIGHT_GREEN);
                let b_resp = ui.add(nav_but);
                if b_resp.clicked() {
                    action_type_index = NodeAction::ShowVisual(current_iri_index);
                }
                b_resp.on_hover_text("This will add the node to the visual graph and switch to visual graph view. The node will be selected.");
                let label_context = LabelContext::new(self.layout_data.display_language, self.persistent_data.config_data.iri_display, &self.prefix_manager);
                ui.horizontal(|ui|{
                    ui.strong("types:");
                    for type_index in &current_node.types {
                        let type_label = self.node_data.type_display(
                            *type_index,
                            &label_context,
                        );
                        if ui.button(type_label.as_str()).clicked() {
                            action_type_index = NodeAction::ShowType(*type_index);
                        }
                    }
                });
                if current_node.properties.is_empty() {
                    let h = (ui.available_height()-40.0).max(300.0);
                    node_to_click = show_refs_table(ui, current_node, &self.node_data, 
                        &mut *self.rdfwrap, &self.color_cache, &self.layout_data, h);
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
                                    if self.persistent_data.config_data.supress_other_language_data
                                    {
                                        if let Literal::LangString(lang, _) = prop_value {
                                            if *lang != self.layout_data.display_language {
                                                if *lang == 0
                                                    && self.layout_data.display_language != 0
                                                {
                                                    // it is fallback language so display if reall language could not be found
                                                    let mut found = false;
                                                    for (predicate_index2, prop_value2) in
                                                        &current_node.properties
                                                    {
                                                        if predicate_index2 == predicate_index
                                                            && prop_value2 != prop_value
                                                        {
                                                            if let Literal::LangString(lang, _) =
                                                                prop_value2
                                                            {
                                                                if *lang
                                                                    == self
                                                                        .layout_data
                                                                        .display_language
                                                                {
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
                                    let predicate_label = self.node_data.predicate_display(
                                        *predicate_index,
                                        &label_context,
                                    );
                                    ui.label(predicate_label.as_str());
                                    ui.label(prop_value.as_ref());
                                    ui.end_row();
                                }
                            });
                        let h = (ui.available_height()-40.0).max(300.0);
                        node_to_click = show_refs_table(ui, current_node, &self.node_data, 
                            &mut *self.rdfwrap, &self.color_cache, &self.layout_data, h);
                    });
                }
            }
        } else {
            ui.heading("No node selected. Please enter Node IRI or click node link from table or graph view.");
        }
        if let Some(node_to_click) = node_to_click {
            self.show_object_by_index(node_to_click, true);
        }
        return action_type_index;
    }

}

pub fn show_refs_table( ui: &mut egui::Ui, current_node: &NObject, 
    node_data: &NodeData, rdfwrap: &mut dyn rdfwrap::RDFAdapter, 
    color_cache: &ColorCache, 
    layout_data: &LayoutData, h: f32) -> Option<IriIndex> {
    let mut node_to_click: Option<IriIndex> = None;        
    StripBuilder::new(ui)
    .size(egui_extras::Size::exact(600.0))
    .size(egui_extras::Size::exact(600.0)) // Two resizable panels with equal initial width
    .horizontal(|mut strip| {
        strip.cell(|ui| {
            if let Some(node_index) = show_references(
                &node_data,
                rdfwrap,
                &color_cache,
                ui,
                "References",
                &current_node.references,
                &layout_data,
                h,
                "ref",
            ) {
                node_to_click = Some(node_index);
            }
        });
        strip.cell(|ui| {
            ui.push_id("ref2", |ui| {
                if let Some(node_index) = show_references(
                    &node_data,
                    rdfwrap,
                    &color_cache,
                    ui,
                    "Referenced by",
                    &current_node.reverse_references,
                    &layout_data,
                    h,
                    "ref_by",
                ) {
                    node_to_click = Some(node_index);
                }
            });
        });
    });
    return node_to_click;
}

pub fn show_references(
    node_data: &NodeData,
    rdfwrap: &mut dyn rdfwrap::RDFAdapter,
    color_cache: &ColorCache,
    ui: &mut egui::Ui,
    label: &str,
    references: &Vec<(IriIndex, IriIndex)>,
    layout_data: &LayoutData,
    height: f32,
    id_salt: &str,
) -> Option<IriIndex> {
    let mut node_to_click: Option<IriIndex> = None;
    if !references.is_empty() {
        ui.heading(label);
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
            })
            .body(|body| {
                body.rows(text_height, references.len(), |mut row| {
                    let (predicate_index, ref_index) = references.get(row.index()).unwrap();
                    row.col(|ui| {
                        
                        
                        ui.label(
                            rdfwrap.iri2label(node_data.get_predicate(*predicate_index).unwrap()),
                        );
                    });
                    node_data
                        .get_node_by_index(*ref_index)
                        .map(|(ref_iri, ref_node)| {
                            row.col(|ui| {
                                if ui.link(ref_iri).clicked() {
                                    node_to_click = Some(*ref_index);
                                }
                            });
                            row.col(|ui| {
                                let types_label = ref_node
                                    .types
                                    .iter()
                                    .map(|type_index| {
                                        rdfwrap.iri2label(node_data.get_type(*type_index).unwrap())
                                    })
                                    .collect::<Vec<&str>>()
                                    .join(", ");
                                ui.label(types_label);
                            });
                            row.col(|ui| {
                                let label = ref_node.node_label_opt(
                                    &color_cache.label_predicate,
                                    layout_data.display_language,
                                );
                                if let Some(label) = label {
                                    ui.label(label);
                                }
                            });
                        });
                });
            });
    }
    return node_to_click;
}
