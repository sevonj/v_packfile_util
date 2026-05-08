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

pub struct ModelData {
    pub smesh: StaticMesh,
    pub file_path: PathBuf,
}

// use crate::app::ui::ModelView;
use crate::app::widgets::LogView;
use crate::app::widgets::StatusPage;
use data::AppState;
use data::AppTab;

pub struct VModelViewer {
    state: AppState,
    model_data: Option<ModelData>,
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
            model_data: None,
            // model_view,
        };
        this.log_text(String::from("Hello there!"));
        this
    }

    fn is_file_open(&self) -> bool {
        self.model_data.is_some()
    }

    fn pick_model_file(&self) -> Option<PathBuf> {
        FileDialog::new()
            .add_filter("SR2 Model Files", &["cmesh_pc", "smesh_pc"])
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

        self.close_file();

        let buf = std::fs::read(&file_path)?;
        let mut offset = 0;
        let smesh = match StaticMesh::from_data(&buf, &mut offset) {
            Ok(smesh) => smesh,
            Err(e) => {
                println!("{file_path:?} off: {offset:#X?}");
                return Err(e);
            }
        };

        self.model_data = Some(ModelData { smesh, file_path });

        // let dump_path = file_path
        //     .with_added_extension("cpu")
        //     .with_added_extension("obj");
        // let contents = smesh.dump_wavefront_cpu();
        // if !contents.is_empty() {
        //     std::fs::write(dump_path, contents.as_bytes())?;
        // }
        Ok(())
    }

    fn close_file(&mut self) {
        self.log_text("Closing file".to_string());
        self.model_data = None;
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
                if let Some(model_data) = &self.model_data {
                    ui.monospace(model_data.file_path.file_name().unwrap().to_string_lossy());
                    ui.label("todo");
                    // if let Some(model_view) = self.model_view.as_mut() {
                    //     model_view.ui(ui);
                    // } else {
                    //     ui.label("Failed to set up 3D view.");
                    // }
                } else {
                    ui.add(StatusPage::status_no_file());
                }
            }
            AppTab::Log => {
                ui.add(LogView::new(&self.state.log));
            }
        }

        ui.input(|i| {
            //let Some(file) = i.raw.dropped_files.first() else {
            //    return;
            //};
            //let Some(file_path) = file.path.clone() else {
            //    return;
            //};

            for file in &i.raw.dropped_files {
                if let Some(file_path) = file.path.clone() {
                    self.try_open_model(file_path);
                }
            }
        });
    }
}
