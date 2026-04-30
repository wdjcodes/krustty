use std::{
    cell::RefCell,
    collections::HashMap,
    rc::Rc,
    sync::{Arc, Mutex},
};

mod cursor;
mod font;
mod grid;
mod texture;
mod view;

use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::{ElementState, MouseScrollDelta, WindowEvent},
    event_loop::{self, EventLoop, EventLoopProxy},
    keyboard::NamedKey,
    window::{Window, WindowId},
};

use crate::{
    pty::Pty,
    terminal::Terminal,
    ui::{
        cursor::CursorRenderer,
        font::GlyphCache,
        grid::{CellInstance, GridRenderer},
        texture::Texture,
        view::ViewPort,
    },
};

pub struct GpuHandle {
    pub adapter: wgpu::Adapter,
    pub device: Rc<wgpu::Device>,
    pub queue: Rc<wgpu::Queue>,
}

impl GpuHandle {
    pub fn init(instance: &wgpu::Instance, surface: &wgpu::Surface) -> Self {
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::default(),
            compatible_surface: Some(surface),
            force_fallback_adapter: false,
        }))
        .expect("Failed to get gpu adapter");

        let (d, q) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: None,
            required_features: wgpu::Features::empty(),
            experimental_features: wgpu::ExperimentalFeatures::disabled(),
            required_limits: wgpu::Limits::default(),
            memory_hints: Default::default(),
            trace: wgpu::Trace::Off,
        }))
        .expect("Failed to get gpu device and queue");

        let device = Rc::new(d);
        let queue = Rc::new(q);
        Self {
            adapter,
            device,
            queue,
        }
    }
}

pub struct Application {
    windows: HashMap<WindowId, WindowContext>,
    #[allow(unused)]
    pub proxy: EventLoopProxy<Event>,
    gpu: Option<Rc<GpuHandle>>,
    instance: wgpu::Instance,
    pub cache: Rc<RefCell<GlyphCache>>,
    atlas_texture: Option<Rc<Texture>>,
}

pub enum Event {
    WakeUp,
}

impl Application {
    pub fn new(event_loop: &EventLoop<Event>) -> Self {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            ..Default::default()
        });

        let mut glyph_cache = GlyphCache::new(1024, CELL_WIDTH as _, CELL_HEIGHT as _);
        glyph_cache.load_ascii();

        Self {
            windows: Default::default(),
            proxy: event_loop.create_proxy(),
            gpu: None,
            instance,
            cache: Rc::new(RefCell::new(glyph_cache)),
            atlas_texture: None,
        }
    }

    pub fn get_gpu_or_init(&mut self, surface: &wgpu::Surface) -> Rc<GpuHandle> {
        let instance = &self.instance;
        self.gpu
            .get_or_insert_with(|| Rc::new(GpuHandle::init(instance, surface)))
            .clone()
    }

    pub fn get_atlas_or_init(&mut self, device: &wgpu::Device) -> Rc<Texture> {
        self.atlas_texture
            .get_or_insert_with(|| Rc::new(Texture::new(device, "atlas_texture", 1024, 1024)))
            .clone()
    }

    pub fn create_surface<'a>(
        &self,
        target: impl Into<wgpu::SurfaceTarget<'a>>,
    ) -> wgpu::Surface<'a> {
        self.instance.create_surface(target).unwrap()
    }
}

impl ApplicationHandler<Event> for Application {
    fn resumed(&mut self, event_loop: &event_loop::ActiveEventLoop) {
        let window_attributes = Window::default_attributes();
        let window = Arc::new(event_loop.create_window(window_attributes).unwrap());
        let proxy = self.proxy.clone();
        let wc = pollster::block_on(WindowContext::new(window.clone(), self, proxy)).unwrap();
        self.windows.insert(window.id(), wc);
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
            } if event.state == ElementState::Pressed => {
                // TODO: Refactor and better separate and propagate dependencies so we can handle more
                // key events without making match statement even larger
                match event.logical_key {
                    winit::keyboard::Key::Named(NamedKey::ArrowUp) => {
                        window.pty.send_input("\x1b[A")
                    }
                    winit::keyboard::Key::Named(NamedKey::ArrowDown) => {
                        window.pty.send_input("\x1b[B")
                    }
                    winit::keyboard::Key::Named(NamedKey::ArrowLeft) => {
                        window.pty.send_input("\x1b[D")
                    }
                    winit::keyboard::Key::Named(NamedKey::ArrowRight) => {
                        window.pty.send_input("\x1b[C")
                    }
                    winit::keyboard::Key::Named(name) => println!("Unhandled key: {:?}", name),
                    _ => (),
                }
                if let Some(text) = event.text {
                    let id = *self.windows.keys().next().unwrap();
                    let window = self.windows.get_mut(&id).unwrap();
                    window.pty.send_input(&text);
                }
            }
            WindowEvent::Resized(size) => window.new_size = Some(size),
            WindowEvent::RedrawRequested => {
                let _ = window.render();
            }
            WindowEvent::MouseWheel {
                delta: MouseScrollDelta::LineDelta(_cols, rows),
                ..
            } => {
                let view_port = &mut window.view_port;
                view_port.start = (view_port.start + rows as f64).clamp(
                    0.0,
                    view_port.max_rows.saturating_sub(view_port.height) as f64,
                );
                view_port.scroll_queued += rows as f64;
                if view_port.scroll_queued >= 1.0 || view_port.scroll_queued <= -1.0 {
                    window.window.request_redraw();
                }
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

const CELL_WIDTH: f32 = 10.0;
const CELL_HEIGHT: f32 = 20.0;

struct WindowContext {
    window: Arc<Window>,
    surface: wgpu::Surface<'static>,
    gpu: Rc<GpuHandle>,
    grid_render: GridRenderer,
    config: wgpu::SurfaceConfiguration,
    // TODO: Probably makes sense to move this to the application? Not sure if it makes sense
    // to have multiple caches.
    cache: Rc<RefCell<GlyphCache>>,
    cursor_render: CursorRenderer,
    is_surface_configured: bool,
    new_size: Option<PhysicalSize<u32>>,
    view_port: ViewPort,
    pty: Pty,
    pub term: Arc<Mutex<Terminal>>,
}

impl WindowContext {
    pub async fn new(
        window: Arc<Window>,
        app: &mut Application,
        event_loop: EventLoopProxy<Event>,
    ) -> anyhow::Result<Self> {
        let size = window.inner_size();
        let rows = (size.height / CELL_HEIGHT as u32) as usize;
        let cols = (size.width / CELL_WIDTH as u32) as usize;
        let term = Arc::new(Mutex::new(Terminal::new(event_loop.clone(), cols, rows)));
        let shell = std::env::var("SHELL").unwrap_or("bash".to_string());
        let pty = Pty::spawn(&shell, term.clone(), cols as u16, rows as u16)
            .expect("Failed to spawn pty");

        let view_port = ViewPort {
            height: rows,
            start: 0.0,
            max_rows: 0,
            scroll_queued: 0.0,
        };

        let surface = app.create_surface(window.clone());

        let gpu = app.get_gpu_or_init(&surface);

        let surface_caps = surface.get_capabilities(&gpu.adapter);
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

        let size = window.inner_size();

        let text_render = GridRenderer::new(
            gpu.device.clone(),
            gpu.queue.clone(),
            &config,
            app.get_atlas_or_init(&gpu.device),
        );

        let cursor_render = CursorRenderer::new(
            gpu.device.clone(),
            gpu.queue.clone(),
            &config,
            size.width,
            size.height,
        );

        surface.configure(&gpu.device, &config);

        Ok(Self {
            window,
            surface,
            config,
            is_surface_configured: true,
            cache: app.cache.clone(),
            term,
            pty,
            grid_render: text_render,
            cursor_render,
            new_size: None,
            view_port,
            gpu,
        })
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }
        self.config.width = width;
        self.config.height = height;
        self.surface.configure(&self.gpu.device, &self.config);
        self.is_surface_configured = true;
        let cols = width as usize / CELL_WIDTH as usize;
        let rows = height as usize / CELL_HEIGHT as usize;
        let mut term = self
            .term
            .lock()
            .expect("Failed to lock terminal during resize");
        self.view_port.height = rows;
        term.grid.resize(cols, rows);
        let _ = self.pty.resize(cols as u16, rows as u16);
        self.grid_render.resize(width, height);
        self.cursor_render.resize(width, height);
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
            .gpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        let term = self
            .term
            .lock()
            .expect("Failed to lock terminal while rendering");
        let grid = &term.grid;
        let instances = &mut self.grid_render.instances;
        instances.clear();

        let view_port = &mut self.view_port;

        view_port.max_rows = term.grid.rows();
        view_port.start = view_port.start.clamp(
            0.0,
            view_port.max_rows.saturating_sub(view_port.height) as f64,
        );
        view_port.scroll_queued -= view_port.scroll_queued.trunc();
        let start_row = view_port.start.floor() as usize;
        for i in start_row..std::cmp::min(start_row + view_port.height, grid.rows()) {
            let row = grid.get_row(i);
            for j in 0..row.cells.len() {
                let cell = row.get_cell(j);
                if cell.c == '\n' {
                    break;
                }
                let mut cache = self.cache.borrow_mut();
                let atlas_size = cache.atlas_size() as f32;
                let glyph = cache.get(cell.c);
                let ax = glyph.x as f32 / atlas_size;
                let ay = glyph.y as f32 / atlas_size;
                let az = ax + CELL_WIDTH / atlas_size;
                let aw = ay + CELL_HEIGHT / atlas_size;
                instances.push(CellInstance {
                    screen_pos: [
                        j as f32,
                        // TODO: Change this when a proper viewport is added
                        (view_port.height + start_row - i - 1) as f32,
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
        }
        if self.cache.borrow().is_dirty() {
            self.grid_render
                .atlas_texture
                .write_texture(&self.gpu.queue, self.cache.borrow().atlas_data());
            self.cache.borrow_mut().clean();
        }
        self.cursor_render
            .set_cursor(grid.cursor.col, grid.cursor.row);
        self.grid_render.render_pass(&view, &mut encoder);
        self.cursor_render.render_pass(&view, &mut encoder);
        self.gpu.queue.submit(std::iter::once(encoder.finish()));
        output.present();
        Ok(())
    }
}
