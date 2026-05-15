// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: sevonj
/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

mod data;
mod style;
mod ui;
mod widgets;

use std::fs::File;
use std::io::BufWriter;
use std::io::Write;
use std::path::PathBuf;

use eframe::App;
use eframe::CreationContext;
use egui::CentralPanel;
use egui::Color32;
use egui::Frame;
use egui::Ui;
use egui_extras::install_image_loaders;
use rfd::FileDialog;
use v_types::StaticMesh;
use v_types::VolitionError;

use crate::app::ui::ModelView;
use crate::app::widgets::LogView;
use crate::app::widgets::StatusPage;
use crate::model_data::ModelData;
use data::AppState;
use data::AppTab;

pub struct VModelViewer {
    state: AppState,
    model_data: Option<ModelData>,
    model_view: Option<ModelView>,
}

impl VModelViewer {
    pub fn new(cc: &CreationContext<'_>) -> Self {
        cc.egui_ctx.set_theme(eframe::egui::Theme::Dark);

        let mut this = Self {
            state: AppState::default(),
            model_data: None,
            model_view: None,
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

    fn prompt_save(&mut self) {
        let Some(model_data) = &self.model_data else {
            return;
        };

        let Some(g_smesh) = &model_data.g_smesh else {
            return;
        };

        let Some(cpu_path) = FileDialog::new()
            .set_file_name(
                model_data
                    .file_path
                    .with_extension("obj")
                    .file_name()
                    .unwrap()
                    .to_string_lossy(),
            )
            .add_filter("SR2 Model Files", &["cmesh_pc", "smesh_pc"])
            .save_file()
        else {
            return;
        };

        let cpu_ext = cpu_path
            .extension()
            .and_then(|p| p.to_str())
            .unwrap_or("smesh_pc");
        let gpu_path = cpu_path.with_extension(format!("g_{cpu_ext}"));

        let cpu_file = match File::create(cpu_path) {
            Ok(ok) => ok,
            Err(e) => {
                self.log_err(&e.into());
                return;
            }
        };
        let gpu_file = match File::create(gpu_path) {
            Ok(ok) => ok,
            Err(e) => {
                self.log_err(&e.into());
                return;
            }
        };

        let mut cw = BufWriter::new(cpu_file);
        if let Err(e) = model_data.smesh.write(&mut cw, &mut 0) {
            self.log_err(&e.into());
            return;
        }
        if let Err(e) = cw.flush() {
            self.log_err(&e.into());
            return;
        }

        let mut gw = BufWriter::new(gpu_file);
        if let Err(e) = gw.write_all(g_smesh) {
            self.log_err(&e.into());
            return;
        }
        if let Err(e) = gw.flush() {
            self.log_err(&e.into());
        }
    }

    fn prompt_replace_with_gltf(&mut self) {
        let Some(file_path) = FileDialog::new()
            .add_filter("GLTF Binary", &["glb"])
            .pick_file()
        else {
            return;
        };

        self.replace_with_gltf(file_path);
    }

    fn replace_with_gltf(&mut self, file_path: PathBuf) {
        let Some(model_data) = self.model_data.as_mut() else {
            return;
        };
        let (document, buffers, _images) = match gltf::import(file_path) {
            Ok(ok) => ok,
            Err(e) => {
                self.log_text(e.to_string());
                return;
            }
        };
        model_data.replace_with_gltf(&document, &buffers);
        self.model_view = None;
    }

    fn prompt_dump_wavefront(&mut self, separate_surfaces: bool) {
        let Some(model_data) = &self.model_data else {
            return;
        };

        let Some(file_path) = FileDialog::new()
            .set_file_name(
                model_data
                    .file_path
                    .with_extension("obj")
                    .file_name()
                    .unwrap()
                    .to_string_lossy(),
            )
            .add_filter("Wavefront .obj", &["obj"])
            .save_file()
        else {
            return;
        };

        let contents = model_data
            .smesh
            .dump_wavefront(model_data.g_smesh.as_deref(), separate_surfaces);
        if !contents.is_empty()
            && let Err(e) = std::fs::write(file_path, contents.as_bytes())
        {
            self.log_err(&e.into());
        }
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
        let mut model_data = ModelData {
            smesh,
            g_smesh: None,
            file_path,
        };
        model_data.try_load_g_smesh();
        self.model_data = Some(model_data);
        Ok(())
    }

    fn close_file(&mut self) {
        self.log_text("Closing file".to_string());
        self.model_data = None;
        self.model_view = None;
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
    fn ui(&mut self, ui: &mut Ui, frame: &mut eframe::Frame) {
        install_image_loaders(ui.ctx());

        self.menu_bar(ui);
        ui.add_space(1.0);

        let fill = Color32::from_hex("#3F3F3F").unwrap();
        CentralPanel::default()
            .frame(Frame::NONE.fill(fill))
            .show_inside(ui, |ui| match self.state.tab {
                AppTab::View => {
                    if let Some(model_data) = &self.model_data {
                        if self.model_view.is_none()
                            && let Some(render_state) = frame.wgpu_render_state()
                        {
                            self.model_view = Some(ModelView::new(
                                render_state,
                                &model_data.smesh,
                                model_data.g_smesh.as_deref(),
                            ));

                            // Clear state on model change. Otherwise old collapsingheader states, etc. will affect the new ui
                            ui.data_mut(|w| w.clear());
                        }
                        if let Some(model_view) = self.model_view.as_mut() {
                            model_view.ui(ui, model_data);
                        } else {
                            ui.label("Failed to set up 3D view.");
                        }
                    } else {
                        ui.add(StatusPage::status_no_file());
                    }
                }
                AppTab::Log => {
                    ui.add(LogView::new(&self.state.log));
                }
            });

        ui.input(|i| {
            let Some(file) = i.raw.dropped_files.first() else {
                return;
            };
            let Some(file_path) = file.path.clone() else {
                return;
            };
            let Some(ext) = file_path.extension().and_then(|e| e.to_str()) else {
                return;
            };

            match ext {
                "smesh_pc" | "cmesh_pc" => self.try_open_model(file_path),
                "glb" => self.replace_with_gltf(file_path),
                _ => (),
            }
        });
    }
}
