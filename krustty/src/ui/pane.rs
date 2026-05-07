use std::{
    cell::RefCell,
    rc::Rc,
    sync::{Arc, Mutex},
};

use palette::WithAlpha;
use winit::event_loop::EventLoopProxy;

use crate::{
    pty::Pty,
    terminal::Terminal,
    ui::{
        Event, GpuHandle,
        cursor::CursorRenderer,
        font::GlyphCache,
        grid::{CellInstance, GridRenderer},
        view::ViewPort,
    },
};

pub struct Pane {
    cursor_render: CursorRenderer,
    grid_render: GridRenderer,
    pub view_port: ViewPort,
    pub pty: Pty,
    pub term: Arc<Mutex<Terminal>>,
    cache: Rc<RefCell<GlyphCache>>,
}

const CELL_WIDTH: f32 = 10.0;
const CELL_HEIGHT: f32 = 20.0;

impl Pane {
    pub fn new(
        width: u32,
        height: u32,
        event_loop: EventLoopProxy<Event>,
        gpu: Rc<GpuHandle>,
        config: &wgpu::SurfaceConfiguration,
        cache: Rc<RefCell<GlyphCache>>,
    ) -> Self {
        let rows = (height / CELL_HEIGHT as u32) as usize;
        let cols = (width / CELL_WIDTH as u32) as usize;
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

        let grid_render = GridRenderer::new(
            width,
            height,
            gpu.device.clone(),
            gpu.queue.clone(),
            config,
            cache.borrow_mut().get_atlas_or_init(&gpu.device),
        );

        let cursor_render =
            CursorRenderer::new(width, height, gpu.device.clone(), gpu.queue.clone(), config);

        Self {
            cursor_render,
            grid_render,
            view_port,
            pty,
            term,
            cache,
        }
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        let rows = (height / CELL_HEIGHT as u32) as usize;
        let cols = (width / CELL_WIDTH as u32) as usize;
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

    pub fn prepare(&mut self) {
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
                    fg_color: cell.fg.with_alpha(1.0).into_linear().into(),
                    bg_color: cell.bg.with_alpha(1.0).into_linear().into(),
                });
            }
        }

        self.cursor_render
            .set_cursor(view_port.grid_to_viewport(&grid.cursor));
    }

    pub fn render(&mut self, view: &wgpu::TextureView, encoder: &mut wgpu::CommandEncoder) {
        self.grid_render.render_pass(view, encoder);
        self.cursor_render.render_pass(view, encoder);
    }
}
