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
const fn cpu_vbuf_layout(
    vertex_header: &v_types::VertexBufferHeader,
) -> wgpu::VertexBufferLayout<'_> {
    wgpu::VertexBufferLayout {
        array_stride: vertex_header.stride as wgpu::BufferAddress,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &wgpu::vertex_attr_array![0 => Float32x3],
    }
}

fn bbox_lines(aabb: &v_types::AABB) -> [[f32; 3]; 24 + 6] {
    let min = [aabb.min.x, aabb.min.y, aabb.min.z];
    let max = [aabb.max.x, aabb.max.y, aabb.max.z];

    let v = |bits: u8| -> [f32; 3] {
        [
            if bits & 0b100 != 0 { max[0] } else { min[0] },
            if bits & 0b010 != 0 { max[1] } else { min[1] },
            if bits & 0b001 != 0 { max[2] } else { min[2] },
        ]
    };

    [
        // bottom
        v(0b000),
        v(0b100),
        v(0b100),
        v(0b101),
        v(0b101),
        v(0b001),
        v(0b001),
        v(0b000),
        // top
        v(0b010),
        v(0b110),
        v(0b110),
        v(0b111),
        v(0b111),
        v(0b011),
        v(0b011),
        v(0b010),
        // inbetween
        v(0b000),
        v(0b010),
        v(0b100),
        v(0b110),
        v(0b101),
        v(0b111),
        v(0b001),
        v(0b011),
        // origin
        [0.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 1.0],
    ]
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct Uniforms {
    view: [[f32; 4]; 4],
    light: [f32; 3],
    _pad: f32,
}

struct CpuSubmesh {
    vbufs: Vec<wgpu::Buffer>,
    ibuf: wgpu::Buffer,
    surfaces: Vec<v_types::Surface>,
    base_pipeline_index: usize,
}

pub struct StaticMeshResource {
    uniform_buf: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    cpu_pipelines: Vec<wgpu::RenderPipeline>,
    cpu_submeshes: Vec<CpuSubmesh>,
    bbox_pipeline: wgpu::RenderPipeline,
    bbox_vbuf: wgpu::Buffer,
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

        let mut cpu_pipelines = vec![];
        for s in smesh.mesh.submeshes.iter().filter(|s| s.cpu.is_some()) {
            let cpu_data = s.cpu.as_ref().unwrap();
            for vertex_header in &cpu_data.vertex_headers {
                let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some("static_mesh_cpu_pipeline"),
                    layout: Some(&pipeline_layout),
                    vertex: wgpu::VertexState {
                        module: &shader,
                        entry_point: Some("vs_main"),
                        compilation_options: PipelineCompilationOptions::default(),
                        buffers: &[cpu_vbuf_layout(vertex_header)],
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
                cpu_pipelines.push(pipeline);
            }
        }

        let mut base_pipeline_index = 0;
        let cpu_submeshes = smesh
            .mesh
            .submeshes
            .iter()
            .filter(|s: &&v_types::Submesh| s.cpu.is_some())
            .map(|s| {
                let cpu_data: &v_types::SubmeshData = s.cpu.as_ref().unwrap();

                use wgpu::util::DeviceExt;

                let mut offset = 0usize;
                let vbufs = cpu_data
                    .vertex_headers
                    .iter()
                    .map(|h| {
                        let byte_len = h.num_vertices as usize * h.stride as usize;
                        let slice = &s.cpu_vdata[offset..offset + byte_len];
                        offset += byte_len;
                        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                            label: Some("cpu_vbuf"),
                            contents: slice,
                            usage: wgpu::BufferUsages::VERTEX,
                        })
                    })
                    .collect();

                let ibuf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("cpu_ibuf"),
                    contents: &s.cpu_idata,
                    usage: wgpu::BufferUsages::INDEX,
                });

                let sub = CpuSubmesh {
                    vbufs,
                    ibuf,
                    surfaces: cpu_data.surfaces.clone(),
                    base_pipeline_index,
                };
                base_pipeline_index += cpu_data.vertex_headers.len();
                sub
            })
            .collect();

        let bbox_vbuf = {
            use wgpu::util::DeviceExt;
            let verts = bbox_lines(&smesh.mesh.header.aabb);
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("aabb_vbuf"),
                contents: bytemuck::cast_slice(&verts),
                usage: wgpu::BufferUsages::VERTEX,
            })
        };

        let bbox_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("aabb_pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: PipelineCompilationOptions::default(),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: 12,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0 => Float32x3],
                }],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_bbox"),
                compilation_options: PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: render_state.target_format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::LineList,
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

        Self {
            uniform_buf,
            bind_group,
            cpu_pipelines,
            cpu_submeshes,
            bbox_pipeline,
            bbox_vbuf,
        }
    }

    pub fn update_uniforms(&self, queue: &wgpu::Queue, cb: &StaticMeshCallback) {
        let view = cb.view.to_cols_array_2d();
        let light = cb.view.inverse().transform_vector3(cb.light).to_array();

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
    pub show_cpu_geom: bool,
    pub show_bbox: bool,
    pub show_origin: bool,
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

        if self.show_cpu_geom {
            rpass.set_bind_group(0, &res.bind_group, &[]);
            for sub in &res.cpu_submeshes {
                rpass.set_index_buffer(sub.ibuf.slice(..), wgpu::IndexFormat::Uint16);

                for surf in &sub.surfaces {
                    let vbuf = &sub.vbufs[surf.vbuf as usize];
                    rpass.set_pipeline(
                        &res.cpu_pipelines[sub.base_pipeline_index + surf.vbuf as usize],
                    );
                    rpass.set_vertex_buffer(0, vbuf.slice(..));
                    let indices = surf.start_index..(surf.start_index + surf.num_indices as u32);
                    let base_vertex = surf.start_vertex as i32;
                    rpass.draw_indexed(indices, base_vertex, 0..1);
                }
            }
        }

        if self.show_bbox {
            rpass.set_pipeline(&res.bbox_pipeline);
            rpass.set_bind_group(0, &res.bind_group, &[]);
            rpass.set_vertex_buffer(0, res.bbox_vbuf.slice(..));
            rpass.draw(0..24, 0..1);
        }

        if self.show_origin {
            rpass.set_pipeline(&res.bbox_pipeline);
            rpass.set_bind_group(0, &res.bind_group, &[]);
            rpass.set_vertex_buffer(0, res.bbox_vbuf.slice(..));
            rpass.draw(24..30, 0..1);
        }
    }
}
