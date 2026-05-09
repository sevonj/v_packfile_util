use bytemuck::Pod;
use bytemuck::Zeroable;
use egui_wgpu::CallbackResources;
use egui_wgpu::CallbackTrait;
use egui_wgpu::RenderState;
use egui_wgpu::wgpu;
use glam::Mat4;
use glam::Vec3;
use wgpu::PipelineCompilationOptions;

const CPU_SHADER: &str = include_str!("shad.wgsl");
const CPU_V_LAYOUT: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
    array_stride: 12 as wgpu::BufferAddress,
    step_mode: wgpu::VertexStepMode::Vertex,
    attributes: &wgpu::vertex_attr_array![0 => Float32x3],
};

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct Uniforms {
    view: [[f32; 4]; 4],
    light: [f32; 3],
    _pad: f32,
}

struct CpuSubmesh {
    vbuf: wgpu::Buffer,
    ibuf: wgpu::Buffer,
    surfaces: Vec<v_types::Surface>,
}

pub struct StaticMeshResource {
    pipeline: wgpu::RenderPipeline,
    uniform_buf: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    submeshes: Vec<CpuSubmesh>,
}

impl StaticMeshResource {
    pub fn new(render_state: &RenderState, smesh: &v_types::StaticMesh) -> Self {
        let device = &render_state.device;

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("static_mesh_cpu_shad"),
            source: wgpu::ShaderSource::Wgsl(CPU_SHADER.into()),
        });

        let uniform_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("static_mesh_cpu_uniforms"),
            size: std::mem::size_of::<Uniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("static_mesh_cpu_bgl"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("static_mesh_cpu_bg"),
            layout: &bgl,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buf.as_entire_binding(),
            }],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("static_mesh_cpu_layout"),
            bind_group_layouts: &[Some(&bgl)],
            immediate_size: 0,
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("static_mesh_cpu_pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: PipelineCompilationOptions::default(),
                buffers: &[CPU_V_LAYOUT],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: render_state.target_format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleStrip,
                strip_index_format: Some(wgpu::IndexFormat::Uint16),
                cull_mode: Some(wgpu::Face::Back),
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: Some(true),
                depth_compare: Some(wgpu::CompareFunction::Less),
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });

        let submeshes = smesh
            .mesh
            .submeshes
            .iter()
            .filter(|s: &&v_types::Submesh| s.cpu.is_some())
            .map(|s| {
                let cpu_head = s.cpu.as_ref().unwrap();

                use wgpu::util::DeviceExt;
                let vbuf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("cpu_vbuf"),
                    contents: &s.cpu_vdata,
                    usage: wgpu::BufferUsages::VERTEX,
                });
                let ibuf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("cpu_ibuf"),
                    contents: &s.cpu_idata,
                    usage: wgpu::BufferUsages::INDEX,
                });

                CpuSubmesh {
                    vbuf,
                    ibuf,
                    surfaces: cpu_head.surfaces.clone(),
                }
            })
            .collect();

        Self {
            pipeline,
            uniform_buf,
            bind_group,
            submeshes,
        }
    }

    pub fn update_uniforms(&self, queue: &wgpu::Queue, cb: &StaticMeshCallback) {
        let view = cb.view.to_cols_array_2d();
        let light = cb.view.transform_vector3(cb.light).to_array();

        queue.write_buffer(
            &self.uniform_buf,
            0,
            bytemuck::bytes_of(&Uniforms {
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

        rpass.set_pipeline(&res.pipeline);
        rpass.set_bind_group(0, &res.bind_group, &[]);

        for sub in &res.submeshes {
            rpass.set_vertex_buffer(0, sub.vbuf.slice(..));
            rpass.set_index_buffer(sub.ibuf.slice(..), wgpu::IndexFormat::Uint16);

            for surf in &sub.surfaces {
                let indices = surf.start_index..(surf.start_index + surf.num_indices as u32);
                let base_vertex = surf.start_vertex as i32;
                rpass.draw_indexed(indices, base_vertex, 0..1);
            }
        }
    }
}
