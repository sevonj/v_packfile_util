// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: sevonj
/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::hash::Hash;

use egui::Align;
use egui::CursorIcon;
use egui::Frame;
use egui::Label;
use egui::Layout;
use egui::Response;
use egui::RichText;
use egui::Sense;
use egui::UiBuilder;
use egui::Widget;

pub struct ActionRow<'a> {
    title: &'a str,
    subtitle: Option<&'a str>,
    id: egui::Id,
}

impl<'a> ActionRow<'a> {
    pub fn new(title: &'a str, subtitle: Option<&'a str>, id_salt: impl Hash) -> Self {
        Self {
            title,
            subtitle,
            id: egui::Id::new(id_salt),
        }
    }
}

impl Widget for ActionRow<'_> {
    fn ui(self, ui: &mut egui::Ui) -> Response {
        ui.style_mut().spacing.item_spacing.x = 1.0;
        let sense = Sense::union(Sense::click(), Sense::hover());

        let response = ui
            .scope_builder(UiBuilder::new().id(self.id).sense(sense), |ui| {
                let style = (*ui.ctx().global_style()).clone();
                let response = ui.response();
                let fill = style.interact(&response).weak_bg_fill;

                Frame::default()
                    .corner_radius(8.0)
                    .inner_margin(8.0)
                    .fill(fill)
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());

                        ui.horizontal(|ui| {
                            ui.add_space(8.0);

                            if let Some(subtitle) = self.subtitle {
                                ui.vertical(|ui| {
                                    ui.set_height(36.0);
                                    ui.add(
                                        Label::new(RichText::new(self.title).strong())
                                            .selectable(false),
                                    );
                                    ui.add(Label::new(RichText::new(subtitle)).selectable(false));
                                });
                            } else {
                                ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                                    ui.set_height(36.0);
                                    ui.add(
                                        Label::new(RichText::new(self.title).strong())
                                            .selectable(false),
                                    );
                                });
                            }
                        });
                    });
            })
            .response;

        response.on_hover_cursor(CursorIcon::PointingHand)
    }
}
