use bytemuck::Pod;
use bytemuck::Zeroable;

const SHADER: &str = include_str!("shad_solid.wgsl");

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct SolidUniforms {
    pub view: [[f32; 4]; 4],
    pub light: [f32; 3],
    pub _pad: f32,
}

pub struct CpuMesh {
    pub vbufs: Vec<wgpu::Buffer>,
    pub ibuf: wgpu::Buffer,
    pub surfaces: Vec<v_types::Surface>,
    pub base_pipeline_index: usize,
}

pub const fn cpu_vbuf_layout(
    vertex_header: &v_types::VertexBuffer,
) -> wgpu::VertexBufferLayout<'_> {
    wgpu::VertexBufferLayout {
        array_stride: vertex_header.stride as wgpu::BufferAddress,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &wgpu::vertex_attr_array![0 => Float32x3],
    }
}

pub fn solid_bgl(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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
    })
}

pub fn solid_uniform_buf(device: &wgpu::Device) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("static_mesh_cpu_uniforms"),
        size: std::mem::size_of::<SolidUniforms>() as u64,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    })
}

pub fn solid_bind_group(
    uniform_buf: &wgpu::Buffer,
    bgl: &wgpu::BindGroupLayout,
    device: &wgpu::Device,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("static_mesh_cpu_bg"),
        layout: bgl,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: uniform_buf.as_entire_binding(),
        }],
    })
}

pub fn cpu_geom_pipelines(
    render_state: &egui_wgpu::RenderState,
    smesh: &v_types::StaticMesh,
    bgl: &wgpu::BindGroupLayout,
) -> Vec<wgpu::RenderPipeline> {
    let device = &render_state.device;

    let shader_cpu = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("static_mesh_cpu_shad"),
        source: wgpu::ShaderSource::Wgsl(SHADER.into()),
    });

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("static_mesh_cpu_layout"),
        bind_group_layouts: &[Some(bgl)],
        immediate_size: 0,
    });

    let mut cpu_pipelines = vec![];
    for s in smesh.lods.iter().filter(|s| s.cpu_geometry.is_some()) {
        let cpu_data = s.cpu_geometry.as_ref().unwrap();
        for vertex_header in &cpu_data.vertex_headers {
            let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("static_mesh_cpu_pipeline"),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader_cpu,
                    entry_point: Some("vs_main"),
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    buffers: &[cpu_vbuf_layout(vertex_header)],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader_cpu,
                    entry_point: Some("fs_main"),
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
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
    cpu_pipelines
}

pub fn cpu_geom_lods(device: &wgpu::Device, smesh: &v_types::StaticMesh) -> Vec<CpuMesh> {
    let mut base_pipeline_index = 0;
    smesh
        .lods
        .iter()
        .filter(|s: &&v_types::Mesh| s.cpu_geometry.is_some())
        .map(|s| {
            let cpu_data: &v_types::Geometry = s.cpu_geometry.as_ref().unwrap();

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

            let sub = CpuMesh {
                vbufs,
                ibuf,
                surfaces: cpu_data.surfaces.clone(),
                base_pipeline_index,
            };
            base_pipeline_index += cpu_data.vertex_headers.len();
            sub
        })
        .collect()
}
