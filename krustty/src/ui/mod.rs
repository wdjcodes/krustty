use std::{cell::RefCell, collections::HashMap, rc::Rc, sync::Arc};

mod cursor;
mod font;
mod grid;
mod pane;
mod texture;
mod window;

use log::info;
use winit::{
    application::ApplicationHandler,
    event::{ElementState, MouseScrollDelta, WindowEvent},
    event_loop::{self, EventLoop, EventLoopProxy},
    keyboard::NamedKey,
    window::{Window, WindowId},
};

use crate::ui::{font::GlyphCache, window::WindowContext};

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
}

pub enum Event {
    WakeUp,
    SendPtyResponse,
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
        }
    }

    pub fn get_gpu_or_init(&mut self, surface: &wgpu::Surface) -> Rc<GpuHandle> {
        let instance = &self.instance;
        self.gpu
            .get_or_insert_with(|| Rc::new(GpuHandle::init(instance, surface)))
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
                window.close();
                self.windows.remove(&window_id);
                if self.windows.is_empty() {
                    event_loop.exit()
                }
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
                        window.pty().send_input(b"\x1b[A")
                    }
                    winit::keyboard::Key::Named(NamedKey::ArrowDown) => {
                        window.pty().send_input(b"\x1b[B")
                    }
                    winit::keyboard::Key::Named(NamedKey::ArrowLeft) => {
                        window.pty().send_input(b"\x1b[D")
                    }
                    winit::keyboard::Key::Named(NamedKey::ArrowRight) => {
                        window.pty().send_input(b"\x1b[C")
                    }
                    winit::keyboard::Key::Named(name) => info!("Unhandled key: {:?}", name),
                    _ => (),
                }
                if let Some(text) = event.text {
                    let id = *self.windows.keys().next().unwrap();
                    let window = self.windows.get_mut(&id).unwrap();
                    window.pty().send_input(text.as_bytes());
                }
            }
            WindowEvent::Resized(size) => window.request_resize(size),
            WindowEvent::RedrawRequested => {
                let _ = window.render();
            }
            WindowEvent::MouseWheel {
                delta: MouseScrollDelta::LineDelta(_cols, rows),
                ..
            } => {
                window.scroll_lines(rows);
            }
            _ => (),
        }
    }

    fn user_event(&mut self, _event_loop: &event_loop::ActiveEventLoop, event: Event) {
        let id = *self.windows.keys().next().unwrap();
        let window = self.windows.get_mut(&id).unwrap();
        match event {
            Event::WakeUp => {
                window.window.request_redraw();
            }
            Event::SendPtyResponse => {
                let mut term = window.pane.term.lock().expect("failed to lock term");
                window.pane.pty.send_input(&term.take_response());
            }
        }
    }
}

const CELL_WIDTH: f32 = 10.0;
const CELL_HEIGHT: f32 = 20.0;
