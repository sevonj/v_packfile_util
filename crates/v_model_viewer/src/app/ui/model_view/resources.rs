mod solid;
mod wireframe;

use egui_wgpu::CallbackResources;
use egui_wgpu::CallbackTrait;
use egui_wgpu::RenderState;
use egui_wgpu::wgpu;
use glam::Mat4;
use glam::Vec3;

use crate::app::ui::model_view::resources::solid::SolidUniforms;

pub struct StaticMeshResource {
    solid_uniform_buf: wgpu::Buffer,
    solid_bind_group: wgpu::BindGroup,
    cpu_geom_pipelines: Vec<wgpu::RenderPipeline>,
    cpu_geom_lods: Vec<solid::CpuMesh>,
    wframe_pipeline: wgpu::RenderPipeline,
    bbox_vbuf: wgpu::Buffer,
    axis_vbuf: wgpu::Buffer,
}

impl StaticMeshResource {
    pub fn new(render_state: &RenderState, smesh: &v_types::StaticMesh) -> Self {
        let device = &render_state.device;

        let solid_bgl = solid::solid_bgl(device);
        let solid_uniform_buf = solid::solid_uniform_buf(device);
        let solid_bind_group = solid::solid_bind_group(&solid_uniform_buf, &solid_bgl, device);

        let cpu_geom_pipelines = solid::cpu_geom_pipelines(render_state, smesh, &solid_bgl);
        let cpu_geom_lods = solid::cpu_geom_lods(device, smesh);

        let wframe_pipeline = wireframe::wframe_pipeline(render_state);
        let bbox_vbuf = wireframe::bbox_vbuf(&smesh.mesh_header.bbox, device);
        let axis_vbuf = wireframe::axis_vbuf(device);

        Self {
            solid_uniform_buf,
            solid_bind_group,
            cpu_geom_pipelines,
            cpu_geom_lods,
            wframe_pipeline,
            bbox_vbuf,
            axis_vbuf,
        }
    }

    pub fn update_uniforms(&self, queue: &wgpu::Queue, cb: &StaticMeshCallback) {
        let view = cb.view.to_cols_array_2d();
        let light = cb.view.inverse().transform_vector3(cb.light).to_array();

        queue.write_buffer(
            &self.solid_uniform_buf,
            0,
            bytemuck::bytes_of(&SolidUniforms {
                view,
                light,
                _pad: 0.0,
            }),
        );
    }
}

pub struct StaticMeshCallback {
    pub view: Mat4,
    pub light: Vec3,
    pub show_cpu_geom: bool,
    pub show_bbox: bool,
    pub show_origin: bool,
    pub visible_lod: usize,
}

impl CallbackTrait for StaticMeshCallback {
    fn prepare(
        &self,
        _device: &wgpu::Device,
        queue: &wgpu::Queue,
        _screen_descriptor: &egui_wgpu::ScreenDescriptor,
        _encoder: &mut wgpu::CommandEncoder,
        resources: &mut CallbackResources,
    ) -> Vec<wgpu::CommandBuffer> {
        if let Some(res) = resources.get::<StaticMeshResource>() {
            res.update_uniforms(queue, self);
        }
        vec![]
    }

    fn paint(
        &self,
        _info: egui::PaintCallbackInfo,
        rpass: &mut wgpu::RenderPass<'static>,
        resources: &CallbackResources,
    ) {
        let Some(res) = resources.get::<StaticMeshResource>() else {
            return;
        };

        if self.show_cpu_geom
            && let Some(sub) = res.cpu_geom_lods.get(self.visible_lod)
        {
            rpass.set_bind_group(0, &res.solid_bind_group, &[]);
            rpass.set_index_buffer(sub.ibuf.slice(..), wgpu::IndexFormat::Uint16);

            for surf in &sub.surfaces {
                let vbuf = &sub.vbufs[surf.vbuf as usize];
                rpass.set_pipeline(
                    &res.cpu_geom_pipelines[sub.base_pipeline_index + surf.vbuf as usize],
                );
                rpass.set_vertex_buffer(0, vbuf.slice(..));
                let indices = surf.start_index..(surf.start_index + surf.num_indices as u32);
                let base_vertex = surf.start_vertex as i32;
                rpass.draw_indexed(indices, base_vertex, 0..1);
            }
        }

        if self.show_bbox {
            rpass.set_pipeline(&res.wframe_pipeline);
            rpass.set_bind_group(0, &res.solid_bind_group, &[]);
            rpass.set_vertex_buffer(0, res.bbox_vbuf.slice(..));
            rpass.draw(0..24, 0..1);
        }

        if self.show_origin {
            rpass.set_pipeline(&res.wframe_pipeline);
            rpass.set_bind_group(0, &res.solid_bind_group, &[]);
            rpass.set_vertex_buffer(0, res.axis_vbuf.slice(..));
            rpass.draw(0..6, 0..1);
        }
    }
}
