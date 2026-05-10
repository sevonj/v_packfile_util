// SPDX-License-Identifier: AGPL-3.0-or-later

use std::collections::VecDeque;

use egui::CentralPanel;
use egui::ScrollArea;
use egui::Vec2b;
use egui::Widget;

pub struct LogView<'a> {
    log: &'a VecDeque<String>,
}

impl<'a> LogView<'a> {
    pub fn new(log: &'a VecDeque<String>) -> Self {
        Self { log }
    }
}

impl Widget for LogView<'_> {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        CentralPanel::no_frame()
            .show_inside(ui, |ui| {
                ScrollArea::new(Vec2b::new(false, true)).show(ui, |ui| {
                    ui.vertical(|ui| {
                        for line in self.log {
                            ui.monospace(line);
                        }
                    });
                });
            })
            .response
    }
}
