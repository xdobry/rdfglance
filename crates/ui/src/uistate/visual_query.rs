use crate::IriIndex;


pub struct VisualQueryUIState {
    pub selected_type_iri: Option<IriIndex>,
}

impl Default for VisualQueryUIState {
    fn default() -> Self {
        Self {
            selected_type_iri: None,
        }
    }
}

impl VisualQueryUIState {
    pub fn clean(&mut self) {
        self.selected_type_iri = None;
    }
}   