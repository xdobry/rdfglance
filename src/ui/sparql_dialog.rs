pub struct SparqlDialog {
    endpoint: String,
    current_combo: usize,
}

impl SparqlDialog {
    pub fn new(last_endpoints: &[String]) -> Self {
        Self {
            current_combo: 0,
            endpoint: if !last_endpoints.is_empty() {
                last_endpoints[0].clone()
            } else {
                String::new()
            },
        }
    }

    pub fn show(
        &mut self,
        ctx: &egui::Context,
        last_endpoints: &[Box<str>],
    ) -> (bool, Option<String>) {
        let mut close_dialog = false;
        let mut is_cancelled = false;

        egui::Window::new("Use SPARQL Endpoint")
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                ui.label("SPARQL Endpoint:");
                ui.text_edit_singleline(&mut self.endpoint);
                if !last_endpoints.is_empty() {
                    ui.label("last used endpoints:");
                    egui::ComboBox::from_id_salt("editable_combo")
                        .selected_text(&last_endpoints[self.current_combo])
                        .show_ui(ui, |ui| {
                            for (index, last_endpoint) in last_endpoints.iter().enumerate() {
                                if ui.selectable_value(&mut self.current_combo, index, last_endpoint).clicked() {
                                    self.endpoint = last_endpoint.to_string();
                                }
                            }
                        });
                }
                ui.horizontal(|ui| {
                    ui.add_enabled_ui(!self.endpoint.is_empty(), |ui| {
                        if ui.button("Connect").clicked() {
                            close_dialog = true; // Mark dialog for closing
                        }
                    });
                    if ui.button("Cancel").clicked() {
                        close_dialog = true; // Mark dialog for closing
                        is_cancelled = true;
                    }
                });
            });

        if close_dialog {
            if is_cancelled {
                (close_dialog, None)
            } else {
                (close_dialog, Some(self.endpoint.clone()))
            }
        } else {
            (close_dialog, None)
        }
    }
}