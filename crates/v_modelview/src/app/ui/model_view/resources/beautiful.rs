use std::collections::HashMap;

use egui_wgpu::RenderState;
use egui_wgpu::wgpu;
use wgpu::util::DeviceExt;

const SHADER_0UV: &str = include_str!("shad_0uv.wgsl");
const SHADER_1UV: &str = include_str!("shad_1uv.wgsl");
const TEX_UVCHECK: &[u8] = include_bytes!("../../../../../assets/tex_uvcheck.png");

pub struct GpuMesh {
    pub vbufs: Vec<wgpu::Buffer>,
    pub ibuf: wgpu::Buffer,
    pub surfaces: Vec<v_types::Surface>,
}

fn attr_generator(h: &v_types::VertexBuffer) -> Vec<wgpu::VertexAttribute> {
    let mut attrs = vec![];

    attrs.push(wgpu::VertexAttribute {
        format: wgpu::VertexFormat::Float32x3,
        offset: 0,
        shader_location: 0,
    });

    let uv_off = 12 + h.attr_len() as u64;
    if h.num_uv_channels > 0 {
        attrs.push(wgpu::VertexAttribute {
            format: wgpu::VertexFormat::Sint16x2,
            offset: uv_off,
            shader_location: 1,
        });
    }

    attrs
}

pub fn load_tex_uvcheck(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    bgl: &wgpu::BindGroupLayout,
) -> wgpu::BindGroup {
    let img = image::load_from_memory(TEX_UVCHECK).unwrap().into_rgba8();

    let (w, h) = img.dimensions();
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("tex_uvcheck"),
        size: wgpu::Extent3d {
            width: w,
            height: h,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });

    queue.write_texture(
        texture.as_image_copy(),
        &img,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(4 * w),
            rows_per_image: None,
        },
        wgpu::Extent3d {
            width: w,
            height: h,
            depth_or_array_layers: 1,
        },
    );

    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("tex_uvcheck_sampler"),
        address_mode_u: wgpu::AddressMode::Repeat,
        address_mode_v: wgpu::AddressMode::Repeat,
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        ..Default::default()
    });

    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("tex_uvcheck_bg"),
        layout: bgl,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(&sampler),
            },
        ],
    })
}

pub fn texture_bgl(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("beautiful_tex_bgl"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            },
        ],
    })
}

pub const fn gpu_vbuf_layout<'a>(
    vertex_header: &v_types::VertexBuffer,
    attributes: &'a [wgpu::VertexAttribute],
) -> wgpu::VertexBufferLayout<'a> {
    wgpu::VertexBufferLayout {
        array_stride: vertex_header.stride as wgpu::BufferAddress,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes,
    }
}

pub fn gpu_geom_pipelines(
    render_state: &RenderState,
    smesh: &v_types::StaticMesh,
    common_bgl: &wgpu::BindGroupLayout,
    tex_bgl: &wgpu::BindGroupLayout,
) -> HashMap<u16, wgpu::RenderPipeline> {
    let device = &render_state.device;

    let shader_0uv = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("beautiful_shad_0uv"),
        source: wgpu::ShaderSource::Wgsl(SHADER_0UV.into()),
    });

    let shader_1uv = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("beautiful_shad_1uv"),
        source: wgpu::ShaderSource::Wgsl(SHADER_1UV.into()),
    });

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("beautiful_layout"),
        bind_group_layouts: &[Some(common_bgl), Some(tex_bgl)],
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

            let attrs = attr_generator(vertex_header);
            let layout = gpu_vbuf_layout(vertex_header, &attrs);

            let shader = match vertex_header.num_uv_channels {
                0 => &shader_0uv,
                _ => &shader_1uv,
            };

            gpu_pipelines.insert(
                surf.material,
                device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some(&format!("beautiful_pipeline {:?}", surf.material)),
                    layout: Some(&pipeline_layout),
                    vertex: wgpu::VertexState {
                        module: shader,
                        entry_point: Some("vs_main"),
                        compilation_options: wgpu::PipelineCompilationOptions::default(),
                        buffers: &[layout],
                    },
                    fragment: Some(wgpu::FragmentState {
                        module: shader,
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
