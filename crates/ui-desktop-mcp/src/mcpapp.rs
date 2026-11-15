
use eframe::Storage;
use egui::Ui;
use rdf_glance::RdfGlanceApp;
use tokio::sync::mpsc::Receiver;

use crate::mcp::UiCommand;

pub struct RdfGlanceMcpApp {
    ui_rx: Receiver<UiCommand>,
    my_app: RdfGlanceApp,
}

impl RdfGlanceMcpApp {
    pub fn new(storage: Option<&dyn Storage>, args: Vec<String>, ui_rx: Receiver<UiCommand> ) -> Self {
        let my_app = RdfGlanceApp::new(storage, args);

        Self {
            ui_rx,
            my_app,
        }
    }

    fn process_mcp_commands(
        &mut self,
        ctx: &egui::Context,
    ) {
        while let Ok(command) = self.ui_rx.try_recv() {
            match command {
                UiCommand::SetTitle { title, reply } => {
                    //self.my_app.set_title(title.clone());

                    ctx.send_viewport_cmd(
                        egui::ViewportCommand::Title(title)
                    );

                    let _ = reply.send(Ok(()));
                }
            }
        }
    }
}

impl eframe::App for RdfGlanceMcpApp {
    fn ui(&mut self, ui: &mut Ui, frame: &mut eframe::Frame) { 
        self.process_mcp_commands(ui.ctx());

        self.my_app.ui(ui, frame);
    }

    fn save(&mut self, storage: &mut dyn Storage) {
        self.my_app.save(storage);
    }
}

