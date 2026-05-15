// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: sevonj
/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use egui::CentralPanel;
use egui::ScrollArea;
use egui::Vec2b;
use egui::Widget;

use crate::app::Logger;

pub struct LogView<'a> {
    logger: &'a Logger,
}

impl<'a> LogView<'a> {
    pub fn new(logger: &'a Logger) -> Self {
        Self { logger }
    }
}

impl Widget for LogView<'_> {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        CentralPanel::no_frame()
            .show_inside(ui, |ui| {
                ScrollArea::new(Vec2b::new(false, true)).show(ui, |ui| {
                    ui.vertical(|ui| {
                        for line in self.logger.lines().0 {
                            ui.monospace(line);
                        }
                        for line in self.logger.lines().1 {
                            ui.monospace(line);
                        }
                    });
                });
            })
            .response
    }
}
