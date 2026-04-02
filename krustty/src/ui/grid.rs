use std::rc::Rc;

use wgpu::util::DeviceExt;

use crate::{
    color::DEFAULT_COLORS,
    ui::{CELL_HEIGHT, CELL_WIDTH, texture::Texture},
};

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ShaderGlobals {
    surface_size: [f32; 2],
    cell_size: [f32; 2],
}

pub struct GridRenderer {
    pub globals: ShaderGlobals,
    pub instances: Vec<CellInstance>,
    instance_buff: wgpu::Buffer,
    device: Rc<wgpu::Device>,
    pipeline: wgpu::RenderPipeline,
    queue: Rc<wgpu::Queue>,
    pub atlas_texture: Texture,
    atlas_bind_group: wgpu::BindGroup,
    view_bind_group: wgpu::BindGroup,
    vertex_buff: wgpu::Buffer,
    globals_buff: wgpu::Buffer,
}

impl GridRenderer {
    pub fn new(
        device: Rc<wgpu::Device>,
        queue: Rc<wgpu::Queue>,
        config: &wgpu::SurfaceConfiguration,
        width: u32,
        height: u32,
    ) -> Self {
        let shader = device.create_shader_module(wgpu::include_wgsl!("../shader.wgsl"));

        let vertex_buff = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(Self::VERTICES),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        // This should match the filterable field of the
                        // corresponding Texture entry above.
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
                label: Some("texture_bind_group_layout"),
            });

        let view_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("View Bind Group Layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[&view_bind_group_layout, &texture_bind_group_layout],
                immediate_size: 0,
            });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[Vertex::desc(), CellInstance::desc()],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview_mask: None,
            cache: None,
        });

        let instances = bytemuck::zeroed_vec(
            (width as usize / CELL_WIDTH as usize) * (height as usize / CELL_HEIGHT as usize),
        );

        let instance_buff = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Instance Buffer"),
            contents: bytemuck::cast_slice(&instances),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });

        let globals = ShaderGlobals {
            surface_size: [width as f32, height as f32],
            cell_size: [CELL_WIDTH, CELL_HEIGHT],
        };

        let globals_buff = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Global Params Buffer"),
            contents: bytemuck::bytes_of(&globals),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let atlas_texture = Texture::new(&device, "Atlas texture", 1024, 1024);

        let atlas_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &texture_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&atlas_texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&atlas_texture.sampler),
                },
            ],
            label: Some("atlas_bind_group"),
        });

        let view_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &view_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(globals_buff.as_entire_buffer_binding()),
            }],
            label: Some("view_bind_group"),
        });

        Self {
            device,
            queue,
            instances,
            instance_buff,
            vertex_buff,
            pipeline,
            atlas_texture,
            atlas_bind_group,
            view_bind_group,
            globals,
            globals_buff,
        }
    }

    pub fn render_pass(&self, view: &wgpu::TextureView, encoder: &mut wgpu::CommandEncoder) {
        self.queue.write_buffer(
            &self.instance_buff,
            0,
            bytemuck::cast_slice(&self.instances),
        );

        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                resolve_target: None,
                depth_slice: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: DEFAULT_COLORS.black[0] as f64 / 255.0,
                        g: DEFAULT_COLORS.black[1] as f64 / 255.0,
                        b: DEFAULT_COLORS.black[2] as f64 / 255.0,
                        a: 1.0,
                    }),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            occlusion_query_set: None,
            timestamp_writes: None,
            multiview_mask: None,
        });
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &self.view_bind_group, &[]);
        render_pass.set_bind_group(1, &self.atlas_bind_group, &[]);
        render_pass.set_vertex_buffer(0, self.vertex_buff.slice(..));
        render_pass.set_vertex_buffer(1, self.instance_buff.slice(..));
        render_pass.draw(0..6, 0..self.instances.len() as u32);
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.globals.surface_size = [width as f32, height as f32];
        self.queue
            .write_buffer(&self.globals_buff, 0, bytemuck::bytes_of(&self.globals));
        self.instances = bytemuck::zeroed_vec(
            (width as usize / CELL_WIDTH as usize) * (height as usize / CELL_HEIGHT as usize),
        );
        self.instance_buff = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Instance Buffer"),
                contents: bytemuck::cast_slice(&self.instances),
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            });
    }

    const VERTICES: &[Vertex] = &[
        Vertex {
            position: [0.0, 0.0],
            tex_cords: [0.0, 0.0],
        },
        Vertex {
            position: [0.0, 1.0],
            tex_cords: [0.0, 1.0],
        },
        Vertex {
            position: [1.0, 0.0],
            tex_cords: [1.0, 0.0],
        },
        Vertex {
            position: [0.0, 1.0],
            tex_cords: [0.0, 1.0],
        },
        Vertex {
            position: [1.0, 1.0],
            tex_cords: [1.0, 1.0],
        },
        Vertex {
            position: [1.0, 0.0],
            tex_cords: [1.0, 0.0],
        },
    ];
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 2],
    tex_cords: [f32; 2],
}

impl Vertex {
    fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x2,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 2]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x2,
                },
            ],
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CellInstance {
    // Where on the screen does this cell go? (e.g., Column 10, Row 5)
    pub screen_pos: [f32; 2],
    // Where in the Texture Atlas is the character? (UV coordinates)
    pub atlas_uv: [f32; 4],
    pub fg_color: [f32; 4],
    pub bg_color: [f32; 4],
}

impl CellInstance {
    fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<CellInstance>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32x2,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 2]>() as wgpu::BufferAddress,
                    shader_location: 3,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 6]>() as wgpu::BufferAddress,
                    shader_location: 4,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 10]>() as wgpu::BufferAddress,
                    shader_location: 5,
                    format: wgpu::VertexFormat::Float32x4,
                },
            ],
        }
    }
}
