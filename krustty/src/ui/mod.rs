use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

mod font;
mod texture;

use rtrb::CopyToUninit;
use wgpu::util::DeviceExt;
use winit::{
    application::ApplicationHandler,
    event::{ElementState, WindowEvent},
    event_loop::{self, EventLoop, EventLoopProxy},
    window::{Window, WindowId},
};

use crate::{
    ansi::AnsiParser, color::DEFAULT_COLORS, pty::Pty, terminal::Terminal, ui::font::GlyphCache,
};

pub struct Application {
    windows: HashMap<WindowId, WindowContext>,
    #[allow(unused)]
    proxy: EventLoopProxy<Event>,
}

pub enum Event {
    #[allow(unused)]
    WakeUp,
    GridUpdate,
}

impl Application {
    pub fn new(event_loop: &EventLoop<Event>) -> Self {
        Self {
            windows: Default::default(),
            proxy: event_loop.create_proxy(),
        }
    }
}

impl ApplicationHandler<Event> for Application {
    fn resumed(&mut self, event_loop: &event_loop::ActiveEventLoop) {
        let window_attributes = Window::default_attributes();
        let window = Arc::new(event_loop.create_window(window_attributes).unwrap());
        self.windows.insert(
            window.id(),
            pollster::block_on(WindowContext::new(window, self.proxy.clone())).unwrap(),
        );
    }

    fn window_event(
        &mut self,
        event_loop: &event_loop::ActiveEventLoop,
        window_id: WindowId,
        event: winit::event::WindowEvent,
    ) {
        let window = self.windows.get_mut(&window_id).unwrap();
        match event {
            WindowEvent::CloseRequested => {
                window.pty.close();
                self.windows.remove(&window_id);
                event_loop.exit()
            }
            WindowEvent::KeyboardInput {
                device_id: _,
                event,
                is_synthetic: _,
            } => {
                // TODO: Refactor and better separate and propagate dependencies so we can get a window id or some
                // other method here to identify the correct window
                if event.state != ElementState::Released {
                    return;
                }

                if let Some(smol_text) = event.text {
                    let text = smol_text.as_bytes();
                    let id = *self.windows.keys().next().unwrap();
                    let window = self.windows.get_mut(&id).unwrap();
                    if let Ok(mut chunk) = window.pty.input.write_chunk_uninit(text.len()) {
                        let (slice1, slice2) = chunk.as_mut_slices();
                        let wrap = slice1.len();
                        text[..wrap].copy_to_uninit(slice1);
                        text[wrap..].copy_to_uninit(slice2);
                        unsafe { chunk.commit(text.len()) };
                    }
                }
            }
            WindowEvent::Resized(size) => window.resize(size.width, size.height),
            WindowEvent::RedrawRequested => {
                let _ = window.render();
            }
            _ => (),
        }
    }

    fn user_event(&mut self, _event_loop: &event_loop::ActiveEventLoop, event: Event) {
        match event {
            Event::WakeUp => (),
            Event::GridUpdate => {
                // TODO: Refactor and better separate and propagate dependencies so we can get a window id or some
                // other method here to identify the correct window
                let id = *self.windows.keys().next().unwrap();
                let window = self.windows.get_mut(&id).unwrap();
                {
                    let term = window.term.lock().expect("Failed to lock terminal");
                    let grid = &term.grid;
                    window.instances.clear();

                    for i in 0..std::cmp::min(24, grid.rows()) {
                        let row = grid.get_row(i);
                        for j in 0..80 {
                            let cell = row.get_cell(j);
                            if let Some(glyph) = window._cache.cache.get(&cell.c) {
                                let atlas_size = window._cache.atlas_size as f32;
                                let ax = glyph.x as f32 / atlas_size;
                                let ay = glyph.y as f32 / atlas_size;
                                let az = ax + CELL_WIDTH / atlas_size;
                                let aw = ay + CELL_HEIGHT / atlas_size;
                                window.instances.push(CellInstance {
                                    screen_pos: [
                                        j as f32 * CELL_WIDTH,
                                        (window.window.inner_size().height as f32
                                            - i as f32 * CELL_HEIGHT)
                                            - CELL_HEIGHT,
                                    ],
                                    atlas_uv: [ax, ay, az, aw],
                                    fg_color: [
                                        cell.fg[0] as f32 / 255.0,
                                        cell.fg[1] as f32 / 255.0,
                                        cell.fg[2] as f32 / 255.0,
                                        1.0,
                                    ],
                                    bg_color: [
                                        cell.bg[0] as f32 / 255.0,
                                        cell.bg[1] as f32 / 255.0,
                                        cell.bg[2] as f32 / 255.0,
                                        1.0,
                                    ],
                                });
                            } else {
                                // println!(
                                //     "Glyph not found: {} {:?}",
                                //     cell.c,
                                //     cell.c.to_string().as_bytes()
                                // );
                            }
                            if cell.c == '\n' {
                                break;
                            }
                        }
                    }
                }
                window.window.request_redraw();
            }
        }
    }
}

const CELL_WIDTH: f32 = 12.0;
const CELL_HEIGHT: f32 = 20.0;

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

const VERTICES: &[Vertex] = &[
    Vertex {
        position: [0.0, 0.0],
        tex_cords: [0.0, 0.0],
    },
    Vertex {
        position: [0.0, CELL_HEIGHT],
        tex_cords: [0.0, 1.0],
    },
    Vertex {
        position: [CELL_WIDTH, 0.0],
        tex_cords: [1.0, 0.0],
    },
    Vertex {
        position: [0.0, CELL_HEIGHT],
        tex_cords: [0.0, 1.0],
    },
    Vertex {
        position: [CELL_WIDTH, CELL_HEIGHT],
        tex_cords: [1.0, 1.0],
    },
    Vertex {
        position: [CELL_WIDTH, 0.0],
        tex_cords: [1.0, 0.0],
    },
];

struct WindowContext {
    window: Arc<Window>,
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    pipeline: wgpu::RenderPipeline,
    vertex_buff: wgpu::Buffer,
    instances: Vec<CellInstance>,
    instance_buff: wgpu::Buffer,
    _glyph_atlas: texture::Texture,
    _cache: GlyphCache,
    atlas_bind_group: wgpu::BindGroup,
    view_bind_group: wgpu::BindGroup,
    is_surface_configured: bool,
    pty: Pty,
    pub term: Arc<Mutex<Terminal>>,
}

impl WindowContext {
    pub async fn new(
        window: Arc<Window>,
        event_loop: EventLoopProxy<Event>,
    ) -> anyhow::Result<Self> {
        let size = window.inner_size();
        let term = Arc::new(Mutex::new(Terminal::new(event_loop.clone())));
        let pty = Pty::spawn("zsh", AnsiParser::new(term.clone())).expect("Failed to spawn pty");

        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            ..Default::default()
        });

        let surface = instance.create_surface(window.clone())?;

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await?;

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: None,
                required_features: wgpu::Features::empty(),
                experimental_features: wgpu::ExperimentalFeatures::disabled(),
                required_limits: wgpu::Limits::default(),
                memory_hints: Default::default(),
                trace: wgpu::Trace::Off,
            })
            .await?;

        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(surface_caps.formats[0]);
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };

        let shader = device.create_shader_module(wgpu::include_wgsl!("../shader.wgsl"));

        let vertex_buff = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(VERTICES),
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

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
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
                    // 4.
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

        let mut glyph_cache = GlyphCache::new(1024, CELL_WIDTH as _, CELL_HEIGHT as _);
        glyph_cache.load_ascii();

        let instances = bytemuck::zeroed_vec(80 * 24);

        let instance_buff = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Instance Buffer"),
            contents: bytemuck::cast_slice(&instances),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });

        let size = window.inner_size();

        let view_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("View Params Buffer"),
            contents: bytemuck::cast_slice(&[size.width as f32, size.height as f32]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let texture = texture::Texture::from_bytes(
            &device,
            &queue,
            &glyph_cache.pixel_data,
            "Atlas texture",
            glyph_cache.atlas_size,
            glyph_cache.atlas_size,
        );

        let atlas_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &texture_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&texture.sampler),
                },
            ],
            label: Some("atlas_bind_group"),
        });

        let view_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &view_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(view_buffer.as_entire_buffer_binding()),
            }],
            label: Some("atlas_bind_group"),
        });

        Ok(Self {
            window,
            surface,
            device,
            queue,
            config,
            pipeline: render_pipeline,
            vertex_buff,
            instance_buff,
            is_surface_configured: false,
            _glyph_atlas: texture,
            atlas_bind_group,
            _cache: glyph_cache,
            view_bind_group,
            instances,
            term,
            pty,
        })
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.config.width = width;
            self.config.height = height;
            self.surface.configure(&self.device, &self.config);
            self.is_surface_configured = true;
        }
    }

    fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        // We can't render unless the surface is configured
        if !self.is_surface_configured {
            return Ok(());
        }

        let output = self.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });
        {
            self.queue.write_buffer(
                &self.instance_buff,
                0,
                bytemuck::cast_slice(&self.instances),
            );

            {
                let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("Render Pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &view,
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
        }
        println!("Rendering");
        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
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
