mod data;
mod ui;
mod widgets;

use std::path::PathBuf;

use eframe::App;
use eframe::CreationContext;
use egui::Ui;
use rfd::FileDialog;
use v_types::StaticMesh;
use v_types::VolitionError;

// use crate::app::ui::ModelView;
use crate::app::widgets::LogView;
use data::AppState;
use data::AppTab;

pub struct VModelViewer {
    state: AppState,
    // model_view: Option<ModelView>,
}

impl VModelViewer {
    pub fn new(cc: &CreationContext<'_>) -> Self {
        cc.egui_ctx.set_theme(eframe::egui::Theme::Dark);

        // let model_view = cc
        //     .wgpu_render_state
        //     .as_ref()
        //     .map(|render_state| ModelView::placeholder(render_state));

        let mut this = Self {
            state: AppState::default(),
            // model_view,
        };
        this.log_text(String::from("Hello there!"));
        this
    }

    fn is_file_open(&self) -> bool {
        false
    }

    fn pick_model_file(&self) -> Option<PathBuf> {
        FileDialog::new()
            .add_filter("SR2 Model Files", &["cmesh", "smesh"])
            .pick_file()
    }

    fn prompt_open_file(&mut self) {
        let Some(file_path) = self.pick_model_file() else {
            return;
        };
        self.try_open_model(file_path);
    }

    fn try_open_model(&mut self, file_path: PathBuf) {
        if let Err(e) = self.open_model(file_path) {
            self.log_err(&e);
            //self.toast_err(e.to_string());
        }
    }

    fn open_model(&mut self, file_path: PathBuf) -> Result<(), VolitionError> {
        self.log_text(format!("Opening {file_path:?}"));
        let buf = std::fs::read(&file_path)?;
        let mut data_offset = 0;
        let smesh = match StaticMesh::from_data(&buf, &mut data_offset) {
            Ok(smesh) => smesh,
            Err(e) => {
                println!("off: {data_offset:#X?}");
                return Err(e);
            }
        };
        self.log_text(format!("submeshes: {:#?}", smesh.mesh.submeshes.len()));
        let dump_path = file_path
            .with_added_extension("cpu")
            .with_added_extension("obj");
        println!("dumping: {dump_path:#?}");
        std::fs::write(dump_path, smesh.dump_wavefront_cpu().as_bytes())?;

        Ok(())
    }

    fn close_file(&mut self) {
        //  if !self.is_session_open() {
        //      return;
        //  }
        //  self.log_text("Closing session".to_string());
        //  self.session = Session::placeholder();
    }

    fn log_err(&mut self, e: &VolitionError) {
        self.log_text(e.to_string())
    }

    fn log_text(&mut self, text: String) {
        println!("{text}");
        while self.state.log.len() > 99 {
            self.state.log.pop_front();
        }
        self.state.log.push_back(text);
    }
}

impl App for VModelViewer {
    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        self.menu_bar(ui);
        ui.add_space(1.0);

        match self.state.tab {
            AppTab::View => {
                // if let Some(model_view) = self.model_view.as_mut() {
                //     model_view.ui(ui);
                // } else {
                //     ui.label("Failed to set up 3D view.");
                // }
                ui.label("todo");
            }
            AppTab::Log => {
                ui.add(LogView::new(&self.state.log));
            }
        }

        ui.input(|i| {
            let Some(file) = i.raw.dropped_files.first() else {
                return;
            };
            let Some(file_path) = file.path.clone() else {
                return;
            };
            self.try_open_model(file_path);
        });
    }
}
