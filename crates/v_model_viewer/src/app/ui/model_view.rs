mod mesh_resources;

use std::time::Duration;
use std::time::Instant;

use eframe::egui_wgpu::RenderState;
use egui::Button;
use egui::Frame;
use egui::Panel;
use egui::ScrollArea;
use egui::Sense;
use egui::Ui;
use egui::UiBuilder;
use glam::Mat4;
use glam::Vec3;

use mesh_resources::StaticMeshCallback;
use mesh_resources::StaticMeshResource;
use v_types::StaticMesh;

use crate::app::ModelData;
use crate::app::style::OSD_FRAME;
use crate::app::style::OSD_PANEL_FRAME;
use crate::app::widgets::StaticMeshInspector;
use crate::app::widgets::StatusPage;

const SPIN_ENABLE_DELAY: u64 = 5;
const INSPECTOR_W: f32 = 300.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewMode {
    SampleText,
    BottomText,
}

pub struct ModelView {
    pub spin: bool,
    angle_y: f32,
    angle_x: f32,
    zoom: f32,
    camera_pos: Vec3,
    last_instant: Instant,
    last_touch: Instant,
    pub view_mode: ViewMode,
    pub show_bbox: bool,
    pub show_origin: bool,
}

impl ModelView {
    pub fn new(render_state: &RenderState, smesh: &StaticMesh) -> Self {
        let resources = StaticMeshResource::new(render_state, smesh);

        render_state
            .renderer
            .write()
            .callback_resources
            .insert(resources);

        let radius = smesh.header.bounding_center.length() + smesh.header.bounding_radius;
        let camera_pos = Vec3 {
            x: smesh.header.bounding_center.x,
            y: smesh.header.bounding_center.y,
            z: smesh.header.bounding_center.z,
        };

        Self {
            angle_y: 0.0,
            angle_x: 0.0,
            spin: true,
            zoom: 1.0 / (radius * 4.0),
            camera_pos,
            last_instant: Instant::now(),
            last_touch: Instant::now() - Duration::from_secs(SPIN_ENABLE_DELAY),
            view_mode: ViewMode::SampleText,
            show_bbox: true,
            show_origin: false,
        }
    }

    pub fn ui(&mut self, ui: &mut Ui, model_data: &ModelData) {
        let rect = ui.available_rect_before_wrap();

        // paint
        {
            let view_off_px = -(rect.width() - (rect.width() - INSPECTOR_W)) / 2.0;
            let view_off_n = view_off_px / (rect.width() / 2.0);
            let view_off_xf = Mat4::from_translation(Vec3::new(view_off_n, 0.0, 0.0));

            let aspect = rect.width() / rect.height();
            let proj = view_off_xf
                * Mat4::perspective_rh(
                    45_f32.to_radians(),
                    aspect,
                    0.1 / self.zoom,
                    100.0 / self.zoom,
                );

            let eye = Mat4::from_rotation_x(self.angle_x)
                .transform_vector3(self.camera_pos + Vec3::new(0.0, 0.0, 1.0 / self.zoom));

            let view = Mat4::look_at_rh(eye, self.camera_pos, Vec3::Y);
            let model = Mat4::from_rotation_y(self.angle_y);
            let light = Vec3::new(1.0, -1.0, 1.0);

            match self.view_mode {
                ViewMode::SampleText => {
                    if model_data.smesh.mesh.has_cpu_geometry() {
                        ui.painter().add(egui_wgpu::Callback::new_paint_callback(
                            rect,
                            StaticMeshCallback {
                                view: proj * view * model,
                                light,
                                show_cpu_geom: true,
                                show_bbox: self.show_bbox,
                                show_origin: self.show_origin,
                            },
                        ));
                    }
                }
                ViewMode::BottomText => (),
            }
        }

        let now = Instant::now();
        let delta_t = (now - self.last_instant).as_secs_f32();
        self.last_instant = now;

        let ui_builder = UiBuilder::new().sense(Sense::click_and_drag());
        let response = ui
            .scope_builder(ui_builder, |ui| {
                ui.set_width(ui.available_width());
                ui.set_height(ui.available_height());

                Panel::right("modelview_inspector")
                    .frame(OSD_PANEL_FRAME)
                    .exact_size(INSPECTOR_W)
                    .resizable(false)
                    .show_separator_line(false)
                    .show_inside(ui, |ui| {
                        // This scope exists to eat input events that would otherwise go to the parent scope
                        ui.scope_builder(UiBuilder::new().sense(Sense::click_and_drag()), |ui| {
                            ui.label("Inspector");
                            let panel_h = ui.available_height();

                            ScrollArea::vertical().show(ui, |ui| {
                                ui.set_width(ui.available_width());
                                ui.set_height(ui.available_height());
                                ui.add(StaticMeshInspector::new(&model_data.smesh));

                                // More comfortable scrolling
                                ui.add_space(panel_h - 32.0);
                            });
                        });
                    });

                Panel::bottom("bottom_infobar")
                    .frame(Frame::NONE.inner_margin(4.0))
                    .show_separator_line(false)
                    .show_inside(ui, |ui| {
                        ui.monospace(model_data.file_path.file_name().unwrap().to_string_lossy());
                    });

                Panel::top("top_toolbar")
                    .frame(Frame::NONE.inner_margin(4.0))
                    .show_separator_line(false)
                    .show_inside(ui, |ui| {
                        OSD_FRAME.show(ui, |ui| {
                            ui.horizontal(|ui| {
                                if ui
                                    .selectable_value(
                                        &mut self.view_mode,
                                        ViewMode::SampleText,
                                        "sample text",
                                    )
                                    .clicked()
                                {
                                    self.view_mode = ViewMode::SampleText
                                }

                                if ui
                                    .selectable_value(
                                        &mut self.view_mode,
                                        ViewMode::BottomText,
                                        "bottom text",
                                    )
                                    .clicked()
                                {
                                    self.view_mode = ViewMode::BottomText
                                }

                                ui.separator();

                                if ui
                                    .add(Button::new("BBOX").selected(self.show_bbox))
                                    .clicked()
                                {
                                    self.show_bbox = !self.show_bbox;
                                }

                                if ui
                                    .add(Button::new("ORIGIN").selected(self.show_origin))
                                    .clicked()
                                {
                                    self.show_origin = !self.show_origin;
                                }

                                if ui.add(Button::new("SPIN").selected(self.spin)).clicked() {
                                    self.spin = !self.spin;
                                    self.last_touch = now - Duration::from_secs(SPIN_ENABLE_DELAY);
                                };
                            });
                        });
                    });

                Frame::NONE.inner_margin(4).show(ui, |ui| {
                    ui.monospace(format!(
                        "{} submeshes",
                        model_data.smesh.mesh.submeshes.len()
                    ));
                    ui.monospace(format!(
                        "{} materials",
                        model_data.smesh.matlib.materials.len()
                    ));
                    ui.monospace(format!("zoom: {}", self.zoom));
                });

                match self.view_mode {
                    ViewMode::SampleText => {
                        if !model_data.smesh.mesh.has_cpu_geometry() {
                            ui.add(StatusPage::new(
                                "Model Has No CPU Geometry",
                                "Nothing to show",
                            ));
                        }
                    }
                    ViewMode::BottomText => {
                        ui.add(StatusPage::new("Not implemented", "Not implemented"));
                    }
                }
            })
            .response;

        let drag = response.drag_motion();

        if drag.length() > 0.0 {
            self.last_touch = now;
            self.angle_y += drag.x * 0.01;
            self.angle_x -= drag.y * 0.01;
            self.angle_x = self.angle_x.clamp(-1.57, 1.57)
        }

        if self.spin && (now - self.last_touch).as_secs() >= SPIN_ENABLE_DELAY {
            self.angle_y += delta_t;
        }

        if response.hovered() {
            let mouse_wheel = ui.input(|i| {
                i.events.iter().find_map(|e| match e {
                    egui::Event::MouseWheel { delta, .. } => Some(delta.y),
                    _ => None,
                })
            });

            if let Some(delta) = mouse_wheel {
                if delta > 0.0 {
                    self.zoom *= 1.0 + delta * 0.2;
                } else {
                    self.zoom /= 1.0 - delta * 0.2;
                }
            }
        }

        ui.request_repaint();
    }
}
