use eframe::Storage;
use egui::{TextBuffer, Ui};
use rdf_glance::IriIndex;
use rdf_glance::uistate::actions::NodeAction;
use rdf_glance::{RdfGlanceApp, uistate::events::AppEvent};
use std::sync::{Arc, RwLock};
use tokio::sync::mpsc::Receiver;

use crate::mcp::UiCommand;
use crate::tool_api::{DatasetProfile, InstanceQueryResult, dataset_profile_from_index};

pub struct RdfGlanceMcpApp {
    ui_rx: Receiver<UiCommand>,
    my_app: RdfGlanceApp,
    dataset_profile: Arc<RwLock<DatasetProfile>>,
}

impl RdfGlanceMcpApp {
    pub fn new(
        storage: Option<&dyn Storage>,
        args: Vec<String>,
        ui_rx: Receiver<UiCommand>,
        dataset_profile: Arc<RwLock<DatasetProfile>>,
    ) -> Self {
        let my_app = RdfGlanceApp::new(storage, args);

        Self {
            ui_rx,
            my_app,
            dataset_profile,
        }
    }

    fn process_mcp_commands(&mut self, _ctx: &egui::Context) {
        while let Ok(command) = self.ui_rx.try_recv() {
            match command {
                UiCommand::ShowTypeInstances { parameters, reply } => {
                    let mut type_index: Option<IriIndex> = None;
                    let mut no_data_lock = false;
                    if let Ok(rdf_data) = self.my_app.rdf_data.read() {
                        type_index = rdf_data.node_data.indexers.type_indexer.get_index_opt(&parameters.type_name);
                    } else {
                        no_data_lock = true;
                    }
                    if let Some(type_index) = type_index {
                        self.my_app.queue_node_action(NodeAction::ShowType(type_index));
                        let _ = reply.send(Ok(InstanceQueryResult { count: 10 }));
                    } else {
                        let message = if no_data_lock {
                            "no data lock".to_string()
                        } else {
                            format!("no type with name {} found. Consult resource again", parameters.type_name)
                        };
                        let _ = reply.send(Err(message));
                    }
                }
            }
        }
    }
}

impl eframe::App for RdfGlanceMcpApp {
    fn ui(&mut self, ui: &mut Ui, frame: &mut eframe::Frame) {
        self.process_mcp_commands(ui.ctx());
        self.my_app.ui(ui, frame);
        if let Some(app_event) = self.my_app.app_event.take() {
            match app_event {
                AppEvent::DataSetChange => {
                    if let Ok(rdf_data) = self.my_app.rdf_data.read() {
                        let profile = dataset_profile_from_index(&rdf_data, &self.my_app.type_index);
                        if let Ok(mut dataset_profile) = self.dataset_profile.write() {
                            *dataset_profile = profile;
                        }
                    }
                }
            }
        }
    }

    fn save(&mut self, storage: &mut dyn Storage) {
        self.my_app.save(storage);
    }
}
