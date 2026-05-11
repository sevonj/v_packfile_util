use std::collections::HashMap;

use egui_wgpu::RenderState;
use egui_wgpu::wgpu;
use wgpu::util::DeviceExt;

const SHADER: &str = include_str!("shad_beautiful.wgsl");

pub struct GpuMesh {
    pub vbufs: Vec<wgpu::Buffer>,
    pub ibuf: wgpu::Buffer,
    pub surfaces: Vec<v_types::Surface>,
}

pub const fn gpu_vbuf_layout(
    vertex_header: &v_types::VertexBuffer,
) -> wgpu::VertexBufferLayout<'_> {
    wgpu::VertexBufferLayout {
        array_stride: vertex_header.stride as wgpu::BufferAddress,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &wgpu::vertex_attr_array![0 => Float32x3],
    }
}

pub fn gpu_geom_pipelines(
    render_state: &RenderState,
    smesh: &v_types::StaticMesh,
    bgl: &wgpu::BindGroupLayout,
) -> HashMap<u16, wgpu::RenderPipeline> {
    let device = &render_state.device;

    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("beautiful_shad"),
        source: wgpu::ShaderSource::Wgsl(SHADER.into()),
    });

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("beautiful_layout"),
        bind_group_layouts: &[Some(bgl)],
        immediate_size: 0,
    });

    let mut gpu_pipelines: HashMap<u16, wgpu::RenderPipeline> = HashMap::new();
    for mesh in &smesh.lod_meshes {
        let geometry = &mesh.gpu_geometry;

        for surf in &geometry.surfaces {
            if gpu_pipelines.contains_key(&surf.material) {
                continue;
            }

            let vertex_header = geometry.vertex_headers.get(surf.vbuf as usize).unwrap();
            gpu_pipelines.insert(
                surf.material,
                device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some(&format!("beautiful_pipeline {:?}", surf.material)),
                    layout: Some(&pipeline_layout),
                    vertex: wgpu::VertexState {
                        module: &shader,
                        entry_point: Some("vs_main"),
                        compilation_options: wgpu::PipelineCompilationOptions::default(),
                        buffers: &[gpu_vbuf_layout(vertex_header)],
                    },
                    fragment: Some(wgpu::FragmentState {
                        module: &shader,
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
                }),
            );
        }
    }
    gpu_pipelines
}

pub fn gpu_geom_lods(
    device: &wgpu::Device,
    smesh: &v_types::StaticMesh,
    gpu_buffers: &[(Vec<&[u8]>, &[u8])],
) -> Vec<GpuMesh> {
    assert_eq!(gpu_buffers.len(), smesh.lod_meshes.len());

    smesh
        .lod_meshes
        .iter()
        .enumerate()
        .zip(gpu_buffers.iter())
        .map(|((i, lod), (vbuf_slices, ibuf_slice))| {
            let vbufs = vbuf_slices
                .iter()
                .enumerate()
                .map(|(ii, bytes)| {
                    device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some(&format!("beautiful_vbuf {i:?}/{ii:?}")),
                        contents: bytes,
                        usage: wgpu::BufferUsages::VERTEX,
                    })
                })
                .collect();

            let ibuf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(&format!("beautiful_ibuf {i:?}")),
                contents: ibuf_slice,
                usage: wgpu::BufferUsages::INDEX,
            });

            GpuMesh {
                vbufs,
                ibuf,
                surfaces: lod.gpu_geometry.surfaces.clone(),
            }
        })
        .collect()
}
