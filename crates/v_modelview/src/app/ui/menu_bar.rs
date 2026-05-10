// SPDX-License-Identifier: AGPL-3.0-or-later

use egui::Button;
use egui::Frame;
use egui::Panel;
use egui::Ui;

use crate::VModelViewer;
use crate::app::AppTab;

impl VModelViewer {
    pub(crate) fn menu_bar(&mut self, ui: &mut Ui) -> egui::Response {
        Panel::top("menu_bar")
            .resizable(false)
            .show_separator_line(false)
            .frame(Frame::side_top_panel(ui.style()).inner_margin(4.0))
            .show_inside(ui, |ui| {
                ui.horizontal(|ui| {
                    self.file_menu(ui);
                    self.dump_menu(ui);

                    ui.separator();

                    ui.selectable_value(
                        &mut self.state.tab,
                        AppTab::View,
                        AppTab::View.to_string(),
                    );
                    ui.selectable_value(&mut self.state.tab, AppTab::Log, AppTab::Log.to_string());
                });
            })
            .response
    }

    fn file_menu(&mut self, ui: &mut Ui) {
        ui.menu_button("File", |ui| {
            if ui.add(Button::new("Open")).clicked() {
                self.prompt_open_file();
            }

            if ui
                .add_enabled(self.is_file_open(), Button::new("Close"))
                .clicked()
            {
                self.close_file();
            }

            ui.separator();

            if ui.add(Button::new("Quit")).clicked() {
                ui.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        });
    }

    fn dump_menu(&mut self, ui: &mut Ui) {
        ui.menu_button("Dump", |ui| {
            if !self.is_file_open() {
                ui.disable();
            }

            let has_cpu_geom = self
                .model_data
                .as_ref()
                .is_some_and(|model_data| model_data.smesh.mesh_header.has_cpu_geometry());
            if ui
                .add_enabled(has_cpu_geom, Button::new("Dump CPU Geometry"))
                .clicked()
            {
                self.prompt_dump_cpu();
            }
        });
    }
}
