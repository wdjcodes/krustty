use std::{cell::RefCell, rc::Rc, sync::Arc};

use winit::{dpi::PhysicalSize, event_loop::EventLoopProxy, window::Window};

use crate::{
    pty::Pty,
    ui::{Application, Event, GpuHandle, font::GlyphCache, pane::Pane},
};

pub struct WindowContext {
    pub window: Arc<Window>,
    surface: wgpu::Surface<'static>,
    gpu: Rc<GpuHandle>,
    config: wgpu::SurfaceConfiguration,
    // TODO: Probably makes sense to move this to the application? Not sure if it makes sense
    // to have multiple caches.
    cache: Rc<RefCell<GlyphCache>>,
    is_surface_configured: bool,
    new_size: Option<PhysicalSize<u32>>,
    pane: Pane,
}

impl WindowContext {
    pub async fn new(
        window: Arc<Window>,
        app: &mut Application,
        event_loop: EventLoopProxy<Event>,
    ) -> anyhow::Result<Self> {
        let size = window.inner_size();

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

        surface.configure(&gpu.device, &config);

        let pane = Pane::new(
            size.width,
            size.height,
            event_loop,
            gpu.clone(),
            &config,
            app.cache.clone(),
        );

        Ok(Self {
            window,
            surface,
            config,
            is_surface_configured: true,
            cache: app.cache.clone(),
            new_size: None,
            gpu,
            pane,
        })
    }

    fn resize(&mut self, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }

        // Resize window
        self.config.width = width;
        self.config.height = height;
        self.surface.configure(&self.gpu.device, &self.config);
        self.is_surface_configured = true;

        self.pane.resize(width, height);
    }

    pub fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
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

        self.pane.prepare();
        self.cache.borrow_mut().update_atlas_texture(&self.gpu);
        self.pane.render(&view, &mut encoder);

        self.gpu.queue.submit(std::iter::once(encoder.finish()));
        output.present();
        Ok(())
    }

    pub fn close(&mut self) {
        self.pane.pty.close();
    }

    /// returns the pty for the active pane
    pub fn pty(&mut self) -> &mut Pty {
        &mut self.pane.pty
    }

    pub fn request_resize(&mut self, size: PhysicalSize<u32>) {
        self.new_size = Some(size)
    }

    pub fn scroll_lines(&mut self, rows: f32) {
        let view_port = &mut self.pane.view_port;
        view_port.start = (view_port.start + rows as f64).clamp(
            0.0,
            view_port.max_rows.saturating_sub(view_port.height) as f64,
        );
        view_port.scroll_queued += rows as f64;
        if view_port.scroll_queued >= 1.0 || view_port.scroll_queued <= -1.0 {
            self.window.request_redraw();
        }
    }
}
