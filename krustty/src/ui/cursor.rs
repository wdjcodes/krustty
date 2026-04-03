use std::rc::Rc;

use wgpu::util::DeviceExt;

use crate::{
    color::{DEFAULT_COLORS, normalize_with_alpha},
    ui::{CELL_HEIGHT, CELL_WIDTH},
};

pub struct CursorRenderer {
    pub globals: ShaderGlobals,
    instance_buff: wgpu::Buffer,
    _device: Rc<wgpu::Device>,
    pipeline: wgpu::RenderPipeline,
    queue: Rc<wgpu::Queue>,
    globals_bind_group: wgpu::BindGroup,
    vertex_buff: wgpu::Buffer,
    globals_buff: wgpu::Buffer,
}

impl CursorRenderer {
    pub fn new(
        device: Rc<wgpu::Device>,
        queue: Rc<wgpu::Queue>,
        config: &wgpu::SurfaceConfiguration,
        width: u32,
        height: u32,
    ) -> Self {
        let shader = device.create_shader_module(wgpu::include_wgsl!("./shaders/solid_rect.wgsl"));

        let vertex_buff = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(Self::VERTICES),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let globals_bind_group_layout =
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
                label: Some("Solid Rect Pipeline Layout"),
                bind_group_layouts: &[&globals_bind_group_layout],
                immediate_size: 0,
            });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Solid Rect Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[Vertex::desc(), CursorInstance::desc()],
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

        let instance_buff = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Rect: Instance Buffer"),
            contents: &bytemuck::zeroed_vec(std::mem::size_of::<CursorInstance>()),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });

        let globals = ShaderGlobals {
            surface_size: [width as f32, height as f32],
            cell_size: [CELL_WIDTH, CELL_HEIGHT],
        };

        let globals_buff = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Rect: Global Params Buffer"),
            contents: bytemuck::bytes_of(&globals),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let globals_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &globals_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(globals_buff.as_entire_buffer_binding()),
            }],
            label: Some("Rect: globals_bind_group"),
        });

        Self {
            _device: device,
            queue,
            instance_buff,
            vertex_buff,
            pipeline,
            globals_bind_group,
            globals,
            globals_buff,
        }
    }

    pub fn set_cursor(&mut self, col: usize, row: usize) {
        let cursor = CursorInstance {
            screen_pos: [
                col as f32 * CELL_WIDTH,
                self.globals.surface_size[1] - row as f32 * CELL_HEIGHT - CELL_HEIGHT,
            ],
            size: [2.0, CELL_HEIGHT],
            fg_color: normalize_with_alpha(&DEFAULT_COLORS.white, 1.0),
        };

        self.queue
            .write_buffer(&self.instance_buff, 0, bytemuck::bytes_of(&cursor));
    }

    pub fn render_pass(&self, view: &wgpu::TextureView, encoder: &mut wgpu::CommandEncoder) {
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                resolve_target: None,
                depth_slice: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            occlusion_query_set: None,
            timestamp_writes: None,
            multiview_mask: None,
        });
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &self.globals_bind_group, &[]);
        render_pass.set_vertex_buffer(0, self.vertex_buff.slice(..));
        render_pass.set_vertex_buffer(1, self.instance_buff.slice(..));
        render_pass.draw(0..6, 0..1);
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.globals.surface_size = [width as f32, height as f32];
        self.queue
            .write_buffer(&self.globals_buff, 0, bytemuck::bytes_of(&self.globals));
    }

    const VERTICES: &[Vertex] = &[
        Vertex {
            position: [0.0, 0.0],
        },
        Vertex {
            position: [0.0, 1.0],
        },
        Vertex {
            position: [1.0, 0.0],
        },
        Vertex {
            position: [0.0, 1.0],
        },
        Vertex {
            position: [1.0, 1.0],
        },
        Vertex {
            position: [1.0, 0.0],
        },
    ];
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 2],
}

impl Vertex {
    fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[wgpu::VertexAttribute {
                offset: 0,
                shader_location: 0,
                format: wgpu::VertexFormat::Float32x2,
            }],
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CursorInstance {
    /// Pixel position of the top left corner
    pub screen_pos: [f32; 2],
    /// width x height of rect in pixels
    pub size: [f32; 2],
    pub fg_color: [f32; 4],
}

impl CursorInstance {
    fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<CursorInstance>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x2,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 2]>() as wgpu::BufferAddress,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32x2,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 4]>() as wgpu::BufferAddress,
                    shader_location: 3,
                    format: wgpu::VertexFormat::Float32x4,
                },
            ],
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ShaderGlobals {
    surface_size: [f32; 2],
    cell_size: [f32; 2],
}
