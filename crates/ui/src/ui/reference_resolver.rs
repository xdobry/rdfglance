
use std::collections::HashSet;
use rayon::prelude::*;

use crate::{
    IriIndex, RdfGlanceApp, domain::{LabelContext, LabelDisplayValue, Literal, RdfData, reference_resolver::resolve_references, type_index::{TypeInstanceIndex, ValueTypes}}, ui::style::ICON_DELETE, uistate::actions::NodeAction
};

pub struct ReferenceResolver {
    items: Vec<ReferenceResolverItem>,
}

impl Default for ReferenceResolver {
    fn default() -> Self {
        Self {
            items: vec![ReferenceResolverItem::default()]
        }
    }
}

pub struct ReferenceResolverItem {
    from_type: Option<IriIndex>,
    to_type: Option<IriIndex>,
    from_predicate: Option<IriIndex>,
    to_predicate: Option<IriIndex>,
    link_predicate: Option<IriIndex>,
    delete: bool,
}

impl Default for ReferenceResolverItem {
    fn default() -> Self {
        Self {
            from_type: None,
            to_type: None,
            from_predicate: None,
            to_predicate: None,
            link_predicate: None,
            delete: false,
        }
    }
}

struct KeyCandidate {
    type_idx: IriIndex,
    predicate: IriIndex,
    pk_candidate: bool,
    values: HashSet<IriIndex>,
}

impl ReferenceResolver {
    pub fn clean(&mut self) {
        self.items.clear();
        self.items.push(ReferenceResolverItem::default());
    }
    pub fn has_defined(&self) -> bool {
        self.items.iter().any(|f| f.is_defined())
    }
    pub fn compute_references(&mut self, rdf_data: &RdfData, type_index: &TypeInstanceIndex) {
        // find primary keys candidates
        // primary keys are always cardinality=1 and unique, there are of type ShortString (so indexed deduplicated string)
        let key_candidates: Vec<KeyCandidate> = type_index.types_order.par_iter()
            .filter_map(|type_idx| type_index.types.get(type_idx).map(|td| (type_idx,td)))
            .map(|(type_idx,type_data)| {
                let mut local_key_candidates: Vec<KeyCandidate> = type_data.properties.iter()
                    .filter_map(|(prop_index,prop_characteristics)| {
                    if prop_characteristics.value_types ==  ValueTypes::SHORT_STRING {
                        Some(KeyCandidate { 
                            type_idx: *type_idx,
                            predicate: *prop_index, 
                            pk_candidate: prop_characteristics.min_cardinality == 1 && prop_characteristics.max_cardinality == 1, 
                            values: HashSet::new(), 
                        })
                    } else {
                        None
                    }
                }).collect();
                if local_key_candidates.len()>0 {
                    // we iterate all instances only if there are candidates to collect values
                    for inst_index in type_data.instances.iter() {
                        if let Some((_iri, node)) = rdf_data.node_data.get_node_by_index(*inst_index) {
                            for (pred_index, literal) in node.properties.iter() {
                                if let Literal::StringShort(str_index) = literal {
                                    if let Some(key_candidate) = local_key_candidates.iter_mut().find(|kc| kc.predicate == *pred_index) {
                                        if !key_candidate.values.insert(*str_index) {
                                            // the value is not unique
                                            key_candidate.pk_candidate = false;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                local_key_candidates
        }).flatten().collect();
        // find foreign keys candidates and matching primary keys
        // foreign keys are always in set of primary key and ShortString
        let new_items: Vec<_> = key_candidates.par_iter().filter(|fk| !fk.pk_candidate).filter_map(|fk| {
            // we have fk candidate, so search for suitable pk
            let fk_proposes: Vec<(usize, ReferenceResolverItem)> = key_candidates.iter().filter(|pk| pk.pk_candidate && fk.values.is_subset(&pk.values)).map(|pk| {
                (pk.values.len()-fk.values.len(),ReferenceResolverItem { 
                    from_type: Some(fk.type_idx), 
                    to_type: Some(pk.type_idx),
                    from_predicate: Some(fk.predicate),
                    to_predicate: Some(pk.predicate),
                    link_predicate: Some(pk.predicate),
                    delete: false
                })
            }).collect();
            // take the pk candidate where the set of pk and fk are most similar the len difference is smallest
            let best_found = fk_proposes.into_iter().min_by_key(|(key,_) | *key);
            best_found.map(|e| e.1)
        }).collect();
        self.items.extend(new_items);
    }
}

impl ReferenceResolverItem {
    pub fn clean(&mut self) {
        self.from_type = None;
        self.to_type = None;
        self.from_predicate = None;
        self.to_predicate = None;
        self.link_predicate = None;
    }

    pub fn is_defined(&self) -> bool {
        self.from_type.is_some() && self.to_type.is_some() && self.from_predicate.is_some() && self.to_predicate.is_some() && self.link_predicate.is_some()
    }

    pub fn show_ui(&mut self, ui: &mut egui::Ui, rdf_data: &RdfData, type_index: &TypeInstanceIndex, label_context: &LabelContext) {
        ui.horizontal(|ui| {
            type_select(ui, &mut self.from_type,
                &mut self.from_predicate,
                    "from type", "from pred", &label_context, &rdf_data, type_index);
            type_select(ui, &mut self.to_type, 
                &mut self.to_predicate, 
                "to type", "to_pred",
                &label_context, &rdf_data, type_index);

            let link_pred_label = if let Some(pred_index) = self.link_predicate {
                rdf_data.node_data.predicate_display(pred_index, &label_context, &rdf_data.node_data.indexers)
            } else {
                LabelDisplayValue::FullRef("<None>")
            };
            egui::ComboBox::from_label("link predicate")
                .width(120.0)
                .selected_text(link_pred_label.as_str())
                .show_ui(ui, |ui| {
                    for pred_index in type_index.predicates.iter() {
                        let pred_label = rdf_data.node_data.predicate_display(*pred_index, &label_context, &rdf_data.node_data.indexers);
                        if ui.selectable_label(  self.link_predicate==Some(*pred_index), pred_label.as_str()).clicked() {
                            self.link_predicate = Some(*pred_index);                               
                        }
                    }
            });
            if ui.button(ICON_DELETE).clicked() {
                self.delete = true;
            }
        });
    }
}

impl RdfGlanceApp {
    pub fn show_reference_resolver(&mut self, ui: &mut egui::Ui) -> NodeAction {  
        ui.heading("Reference Resolver");
        ui.label("The tool creates object properties (links) from data properties organized after primary and foreign key tables principe. Useful if non rdf data were imported.");
        egui::ScrollArea::vertical().show(ui,|ui| {
            if let Ok(rdf_data) = self.rdf_data.read() {           
                let label_context = LabelContext::new(
                    self.ui_state.display_language,
                    self.persistent_data.config_data.iri_display,
                    &rdf_data.prefix_manager,
                );
                if ui.button("Compute possible references from data").clicked() {
                    self.reference_resolver.compute_references( &rdf_data, &self.type_index);
                }
                for (idx,reference_resolver_item) in self.reference_resolver.items.iter_mut().enumerate() {
                    ui.push_id(idx, |ui| {
                        reference_resolver_item.show_ui(ui, &rdf_data, &self.type_index, &label_context);
                    });
                }
                if self.reference_resolver.items.len() > 1 {
                    self.reference_resolver.items.retain(|f| !f.delete);
                }
                if let Some(last_item) = self.reference_resolver.items.last() {
                    if last_item.is_defined() {
                        self.reference_resolver.items.push(ReferenceResolverItem::default());
                    }
                }
            }
            let has_defined = self.reference_resolver.has_defined();
            ui.add_enabled_ui(has_defined, |ui| {
                if ui.button("Resolve References").clicked() {
                    if let Ok(mut rdf_data) = self.rdf_data.write() { 
                        for resolver_item in self.reference_resolver.items.iter() {
                            if let Some(from_type) = resolver_item.from_type && let Some(from_predicate) = resolver_item.from_predicate 
                                && let Some(to_type) = resolver_item.to_type && let Some(to_predicate) = resolver_item.to_predicate 
                                && let Some(link_predicate) = resolver_item.link_predicate {
                                resolve_references(&mut rdf_data.node_data, &self.type_index, from_type, from_predicate, 
                                    to_type, to_predicate, link_predicate);
                            }
                        }
                        self.reference_resolver.clean();
                        self.type_index.update(&rdf_data.node_data);
                    }
                }            
            });

        });
        NodeAction::None
    }
}

fn type_select(ui: &mut egui::Ui, type_ref: &mut Option<IriIndex>, predicate_ref: &mut Option<IriIndex>, 
    label: &str,label_pred: &str, label_context: &LabelContext, rdf_data: &RdfData, type_index: &TypeInstanceIndex) {
    let selected_label = if let Some(type_index) = type_ref {
        rdf_data.node_data.type_display(*type_index, &label_context, &rdf_data.node_data.indexers)
    } else {
        LabelDisplayValue::FullRef("<None>")
    };
    egui::ComboBox::from_label(label)
        .width(120.0)
        .selected_text(selected_label.as_str())
        .show_ui(ui, |ui| {
            for type_idx in type_index.types_order.iter() {
                let type_label = rdf_data.node_data.type_display(*type_idx, &label_context, &rdf_data.node_data.indexers);
                if ui.selectable_label(  *type_ref==Some(*type_idx), type_label.as_str()).clicked() {
                    *type_ref = Some(*type_idx);
                    *predicate_ref = None;
                }
            }
        });

    let selected_pred_label = if let Some(pred_index) = predicate_ref {
        rdf_data.node_data.predicate_display(*pred_index, &label_context, &rdf_data.node_data.indexers)
    } else {
        LabelDisplayValue::FullRef("<None>")
    };
    egui::ComboBox::from_label(label_pred)
        .width(120.0)
        .selected_text(selected_pred_label.as_str())
        .show_ui(ui, |ui| {
            if let Some(type_idx) = type_ref {
                if let Some(type_data) = type_index.types.get(type_idx) {
                    for predicate_idx in type_data.properties.keys() {
                        let type_label = rdf_data.node_data.predicate_display(*predicate_idx, &label_context, &rdf_data.node_data.indexers);
                        if ui.selectable_label(  *predicate_ref==Some(*predicate_idx), type_label.as_str()).clicked() {
                            *predicate_ref = Some(*predicate_idx);
                        }
                    }                   
                }
            }
        });
}
