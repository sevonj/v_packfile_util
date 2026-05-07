// SPDX-License-Identifier: AGPL-3.0-or-later

use std::hash::Hash;

use egui::Button;
use egui::Frame;
use egui::Label;
use egui::Painter;
use egui::RichText;
use egui::Sense;
use egui::Stroke;
use egui::Ui;
use egui::UiBuilder;
use egui::Widget;
use egui::pos2;

pub struct Tab<'a> {
    text: &'a str,
    id: egui::Id,
    is_current: bool,
    is_closed: Option<&'a mut bool>,
}

impl<'a> Tab<'a> {
    pub fn new(text: &'a str, is_current: bool, id_salt: impl Hash) -> Self {
        Self {
            text,
            id: egui::Id::new(id_salt),
            is_current,
            is_closed: None,
        }
    }

    #[allow(dead_code)]
    pub fn closable(mut self, signal: &'a mut bool) -> Self {
        self.is_closed = Some(signal);
        self
    }

    pub fn value<Value: PartialEq>(
        ui: &mut Ui,
        current_value: &mut Value,
        tab_value: Value,
        text: &'a str,
        id_salt: impl Hash,
    ) -> egui::Response {
        let mut response = ui.add(Self::new(text, *current_value == tab_value, id_salt));
        if response.clicked() && *current_value != tab_value {
            *current_value = tab_value;
            response.mark_changed();
        }
        response
    }
}

impl Widget for Tab<'_> {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        ui.style_mut().spacing.item_spacing.x = 1.0;
        let sense = Sense::union(Sense::click(), Sense::hover());

        ui.scope_builder(UiBuilder::new().id(self.id).sense(sense), |ui| {
            let style = (*ui.ctx().global_style()).clone();
            let response = ui.response();
            let fill = if self.is_current {
                style.interact(&response).bg_fill
            //} else if response.hovered() {
            //    style.interact(&response).weak_bg_fill
            } else {
                style.visuals.faint_bg_color
            };

            Frame::group(&style)
                .inner_margin(4.)
                .outer_margin(0.)
                .corner_radius(0.)
                .stroke(Stroke::NONE)
                .fill(fill)
                .show(ui, |ui| {
                    ui.style_mut().spacing.item_spacing.x = 0.0;
                    ui.add_space(4.0);
                    ui.add(
                        Label::new(
                            RichText::new(self.text).color(style.interact(&response).text_color()),
                        )
                        .selectable(false),
                    );

                    if let Some(is_closed) = self.is_closed {
                        ui.add_space(6.0);

                        if ui
                            .add(Button::new(RichText::new("❌").size(14.0)).frame(false))
                            .on_hover_text("Close Tab")
                            .clicked()
                        {
                            *is_closed = true;
                        }
                    }
                    ui.add_space(2.0);
                });

            if self.is_current {
                let rect = response.rect;
                let painter = Painter::new(ui.ctx().clone(), ui.layer_id(), rect);
                let a = pos2(rect.min.x, rect.max.y);
                let b = rect.max;
                painter.line(vec![a, b], Stroke::new(4., style.visuals.selection.bg_fill));
            }

            response
        })
        .inner
    }
}
