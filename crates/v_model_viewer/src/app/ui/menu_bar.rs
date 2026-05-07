// SPDX-License-Identifier: AGPL-3.0-or-later

use egui::Button;
use egui::Color32;
use egui::Frame;
use egui::Layout;
use egui::Panel;
use egui::Ui;
use egui::vec2;

use crate::VModelViewer;
use crate::app::AppTab;

use crate::app::widgets::Tab;

const PANEL_HEIGHT: f32 = 48.0;
const PANEL_CORNER_RADIUS: f32 = 1.0;

impl VModelViewer {
    pub(crate) fn menu_bar(&mut self, ui: &mut Ui) -> egui::Response {
        let fill = Color32::from_hex("#333").unwrap();

        Panel::top("menu_bar")
            .resizable(false)
            .exact_size(PANEL_HEIGHT)
            .show_separator_line(false)
            .frame(
                Frame::default()
                    .inner_margin(vec2(8., 2.))
                    .fill(fill)
                    .corner_radius(PANEL_CORNER_RADIUS),
            )
            .show_inside(ui, |ui| {
                // let bar_rect = ui.content_rect();
                // Image::from(egui::include_image!(
                //     "../../../assets/tex_toolbar_gradient.svg"
                // ))
                // .corner_radius(PANEL_CORNER_RADIUS)
                // .paint_at(ui, bar_rect.with_max_y(bar_rect.min.y + PANEL_HEIGHT));

                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.style_mut().spacing.item_spacing.x = 2.0;
                        self.file_menu(ui);
                    });

                    ui.with_layout(Layout::left_to_right(egui::Align::Max), |ui| {
                        Tab::value(
                            ui,
                            &mut self.state.tab,
                            AppTab::View,
                            &AppTab::View.to_string(),
                            "tab_session",
                        );
                        Tab::value(
                            ui,
                            &mut self.state.tab,
                            AppTab::Log,
                            &AppTab::Log.to_string(),
                            "tab_log",
                        );
                    });
                });
            })
            .response
    }

    fn file_menu(&mut self, ui: &mut Ui) {
        ui.menu_button("File", |ui| {
            if ui.add(Button::new("Open")).clicked() {
                self.prompt_open_file();
            }

            // if ui
            //     .add_enabled(
            //         self.can_save(),
            //         Button::new("Save")
            //     )
            //     .clicked()
            // {
            //     self.prompt_save_file();
            // }

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
}
