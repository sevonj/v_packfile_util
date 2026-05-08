mod mesh_resources;

use std::time::Duration;
use std::time::Instant;

use eframe::egui_wgpu::RenderState;
use egui::Sense;
use egui::Ui;
use egui::UiBuilder;
use glam::Mat4;
use glam::Vec3;

use mesh_resources::StaticMeshCallback;
use mesh_resources::StaticMeshResource;
use v_types::StaticMesh;

use crate::app::ModelData;
use crate::app::widgets::StatusPage;

const SPIN_ENABLE_DELAY: u64 = 5;

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
    last_instant: Instant,
    last_touch: Instant,
    pub view_mode: ViewMode,
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

        Self {
            angle_y: 0.0,
            angle_x: 0.0,
            spin: true,
            zoom: 1.0 / (radius * 4.0),
            last_instant: Instant::now(),
            last_touch: Instant::now() - Duration::from_secs(SPIN_ENABLE_DELAY),
            view_mode: ViewMode::SampleText,
        }
    }

    pub fn ui(&mut self, ui: &mut Ui, model_data: &ModelData) {
        let rect = ui.available_rect_before_wrap();

        // paint
        {
            let aspect = rect.width() / rect.height();
            let proj = Mat4::perspective_rh(45_f32.to_radians(), aspect, 0.1, 100.0);

            let eye = Mat4::from_rotation_x(self.angle_x).transform_vector3(Vec3::new(
                0.0,
                0.0,
                1.0 / self.zoom,
            ));

            let view = Mat4::look_at_rh(eye, Vec3::ZERO, Vec3::Y);
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

                ui.monospace(model_data.file_path.file_name().unwrap().to_string_lossy());
                ui.monospace(format!(
                    "{} submeshes",
                    model_data.smesh.mesh.submeshes.len()
                ));
                ui.monospace(format!(
                    "{} materials",
                    model_data.smesh.matlib.materials.len()
                ));
                ui.monospace(format!("zoom: {}", self.zoom));
                if ui.checkbox(&mut self.spin, "spin").clicked() {
                    self.last_touch = now - Duration::from_secs(SPIN_ENABLE_DELAY);
                };

                if ui
                    .selectable_value(&mut self.view_mode, ViewMode::SampleText, "sample text")
                    .clicked()
                {
                    self.view_mode = ViewMode::SampleText
                }

                if ui
                    .selectable_value(&mut self.view_mode, ViewMode::BottomText, "bottom text")
                    .clicked()
                {
                    self.view_mode = ViewMode::BottomText
                }

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
