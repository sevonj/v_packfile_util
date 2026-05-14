const SHADER: &str = include_str!("shad_wireframe.wgsl");
const STRIDE: u64 = (3 + 4) * 4;
const LAYOUT: wgpu::VertexBufferLayout<'_> = wgpu::VertexBufferLayout {
    array_stride: STRIDE as wgpu::BufferAddress,
    step_mode: wgpu::VertexStepMode::Vertex,
    attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x4],
};
const AXIS_GIZMO_LINES: [[f32; 3 + 4]; 6] = [
    [0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0],
    [-1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0],
    [0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 1.0],
    [0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 1.0],
    [0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 1.0],
    [0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 1.0],
];

pub fn wframe_pipeline(render_state: &egui_wgpu::RenderState) -> wgpu::RenderPipeline {
    let device = &render_state.device;

    let wframe_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("wireframe_shad"),
        source: wgpu::ShaderSource::Wgsl(SHADER.into()),
    });

    let bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("wireframe_bgl"),
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

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("wireframe_layout"),
        bind_group_layouts: &[Some(&bgl)],
        immediate_size: 0,
    });

    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("wireframe_pipeline"),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &wframe_shader,
            entry_point: Some("vs_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            buffers: &[LAYOUT],
        },
        fragment: Some(wgpu::FragmentState {
            module: &wframe_shader,
            entry_point: Some("fs_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
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
    })
}

pub fn bbox_vbuf(bbox: &v_types::AABB, device: &wgpu::Device) -> wgpu::Buffer {
    use wgpu::util::DeviceExt;

    let verts = {
        let aabb: &v_types::AABB = bbox;
        const COLOR: [f32; 4] = [1.0, 0.6, 0.0, 1.0];
        let min = [aabb.min.x, aabb.min.y, aabb.min.z];
        let max: [f32; 3] = [aabb.max.x, aabb.max.y, aabb.max.z];
        let v = |bits: u8| -> [f32; 3 + 4] {
            let coord = [
                if bits & 0b100 != 0 { max[0] } else { min[0] },
                if bits & 0b010 != 0 { max[1] } else { min[1] },
                if bits & 0b001 != 0 { max[2] } else { min[2] },
            ];
            let mut whole = [0.0; 3 + 4];
            let (one, two) = whole.split_at_mut(3);
            one.copy_from_slice(&coord);
            two.copy_from_slice(&COLOR);
            whole
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
        ]
    };

    device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("aabb_vbuf"),
        contents: bytemuck::cast_slice(&verts),
        usage: wgpu::BufferUsages::VERTEX,
    })
}

pub fn axis_vbuf(device: &wgpu::Device) -> wgpu::Buffer {
    use wgpu::util::DeviceExt;

    device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("aabb_vbuf"),
        contents: bytemuck::cast_slice(&AXIS_GIZMO_LINES),
        usage: wgpu::BufferUsages::VERTEX,
    })
}
