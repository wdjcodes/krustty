use std::{
    collections::HashMap,
    rc::Rc,
    sync::{Arc, Mutex},
};

mod cursor;
mod font;
mod grid;
mod texture;

use rtrb::CopyToUninit;
use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::{ElementState, WindowEvent},
    event_loop::{self, EventLoop, EventLoopProxy},
    window::{Window, WindowId},
};

use crate::{
    pty::Pty,
    terminal::Terminal,
    ui::{
        cursor::CursorRenderer,
        font::GlyphCache,
        grid::{CellInstance, GridRenderer},
    },
};

pub struct Application {
    windows: HashMap<WindowId, WindowContext>,
    #[allow(unused)]
    proxy: EventLoopProxy<Event>,
}

pub enum Event {
    WakeUp,
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
            WindowEvent::Resized(size) => window.new_size = Some(size),
            WindowEvent::RedrawRequested => {
                let _ = window.render();
            }
            _ => (),
        }
    }

    fn user_event(&mut self, _event_loop: &event_loop::ActiveEventLoop, event: Event) {
        match event {
            Event::WakeUp => {
                let id = *self.windows.keys().next().unwrap();
                let window = self.windows.get_mut(&id).unwrap();
                window.window.request_redraw();
            }
        }
    }
}

const CELL_WIDTH: f32 = 12.0;
const CELL_HEIGHT: f32 = 20.0;

struct WindowContext {
    window: Arc<Window>,
    surface: wgpu::Surface<'static>,
    device: Rc<wgpu::Device>,
    queue: Rc<wgpu::Queue>,
    grid_render: GridRenderer,
    config: wgpu::SurfaceConfiguration,
    cache: GlyphCache,
    cursor_render: CursorRenderer,
    is_surface_configured: bool,
    new_size: Option<PhysicalSize<u32>>,
    pty: Pty,
    pub term: Arc<Mutex<Terminal>>,
}

impl WindowContext {
    pub async fn new(
        window: Arc<Window>,
        event_loop: EventLoopProxy<Event>,
    ) -> anyhow::Result<Self> {
        let size = window.inner_size();
        let rows = (size.height / CELL_HEIGHT as u32) as usize;
        let cols = (size.width / CELL_WIDTH as u32) as usize;
        let term = Arc::new(Mutex::new(Terminal::new(event_loop.clone(), cols, rows)));
        let pty =
            Pty::spawn("zsh", term.clone(), cols as u16, rows as u16).expect("Failed to spawn pty");

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

        let (d, q) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: None,
                required_features: wgpu::Features::empty(),
                experimental_features: wgpu::ExperimentalFeatures::disabled(),
                required_limits: wgpu::Limits::default(),
                memory_hints: Default::default(),
                trace: wgpu::Trace::Off,
            })
            .await?;
        let device = Rc::new(d);
        let queue = Rc::new(q);

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
        let mut glyph_cache = GlyphCache::new(1024, CELL_WIDTH as _, CELL_HEIGHT as _);
        glyph_cache.load_ascii();

        let size = window.inner_size();

        let text_render = GridRenderer::new(
            device.clone(),
            queue.clone(),
            &config,
            size.width,
            size.height,
        );

        text_render
            .atlas_texture
            .write_texture(&queue, &glyph_cache.pixel_data);

        let cursor_render = CursorRenderer::new(
            device.clone(),
            queue.clone(),
            &config,
            size.width,
            size.height,
        );

        surface.configure(&device, &config);
        // self.is_surface_configured = true;

        Ok(Self {
            window,
            surface,
            device,
            queue,
            config,
            is_surface_configured: true,
            cache: glyph_cache,
            term,
            pty,
            grid_render: text_render,
            cursor_render,
            new_size: None,
        })
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }
        self.config.width = width;
        self.config.height = height;
        self.surface.configure(&self.device, &self.config);
        self.is_surface_configured = true;
        println!("Trying to lock for resize");
        let mut term = self
            .term
            .lock()
            .expect("Failed to lock terminal during resize");
        println!("Locked for resize");
        let cols = width as usize / CELL_WIDTH as usize;
        let rows = height as usize / CELL_HEIGHT as usize;
        println!("Resizing grid");
        term.grid.resize(cols, rows);
        println!("Grid resize completed");
        let _ = self.pty.resize(cols as u16, rows as u16);
        println!("Resizing grid renderer");
        self.grid_render.resize(width, height);
        self.cursor_render.resize(width, height);
        println!("Resize complete");
    }

    fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        // We can't render unless the surface is configured
        if !self.is_surface_configured {
            return Ok(());
        }
        if let Some(size) = self.new_size.take() {
            self.resize(size.width, size.height);
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

        println!("Locking for render");
        let term = self
            .term
            .lock()
            .expect("Failed to lock terminal while rendering");
        let grid = &term.grid;
        let instances = &mut self.grid_render.instances;
        instances.clear();

        for i in 0..std::cmp::min(term.grid.height, grid.rows()) {
            let row = grid.get_row(i);
            for j in 0..row.cells.len() {
                let cell = row.get_cell(j);
                // print!("{}", cell.c);
                if let Some(glyph) = self.cache.cache.get(&cell.c) {
                    let atlas_size = self.cache.atlas_size as f32;
                    let ax = glyph.x as f32 / atlas_size;
                    let ay = glyph.y as f32 / atlas_size;
                    let az = ax + CELL_WIDTH / atlas_size;
                    let aw = ay + CELL_HEIGHT / atlas_size;
                    instances.push(CellInstance {
                        screen_pos: [
                            j as f32,
                            // TODO: Change this when a proper viewport is added
                            (term.grid.height - i - 1) as f32,
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
                }
                if cell.c == '\n' {
                    break;
                }
            }
            // println!();
        }
        self.cursor_render
            .set_cursor(grid.cursor.col, grid.cursor.row);
        self.grid_render.render_pass(&view, &mut encoder);
        self.cursor_render.render_pass(&view, &mut encoder);
        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();
        println!("rendering complete");
        Ok(())
    }
}
