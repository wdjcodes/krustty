use std::{
    cell::RefCell,
    rc::Rc,
    sync::{Arc, Mutex},
};

use palette::WithAlpha;
use winit::event_loop::EventLoopProxy;

use crate::{
    grid::{CellFlags, Cursor},
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
            offset: 0,
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
        view_port.apply_scroll(term.grid.rows());
        let start_row = grid
            .rows()
            .saturating_sub(view_port.offset + view_port.height);
        log::debug!(
            "Rendering rows: {}..{}",
            start_row,
            std::cmp::min(start_row + view_port.height, grid.rows().saturating_sub(1))
        );
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
                let mut fg_color = cell.fg.with_alpha(1.0).into_linear().into();
                let mut bg_color = cell.bg.with_alpha(1.0).into_linear().into();
                if cell.flags.contains(CellFlags::INVERSE) {
                    std::mem::swap(&mut fg_color, &mut bg_color);
                }
                instances.push(CellInstance {
                    screen_pos: [j as f32, (i - start_row) as f32],
                    atlas_uv: [ax, ay, az, aw],
                    fg_color,
                    bg_color,
                });
            }
        }

        if let Some(row) = view_port.grid_to_viewport(grid.cursor.row, grid.rows()) {
            self.cursor_render.set_cursor(Some(Cursor {
                col: grid.cursor.col,
                row,
                will_wrap: false,
            }));
        } else {
            self.cursor_render.set_cursor(None);
        }
    }

    pub fn render(&mut self, view: &wgpu::TextureView, encoder: &mut wgpu::CommandEncoder) {
        self.grid_render.render_pass(view, encoder);
        self.cursor_render.render_pass(view, encoder);
    }
}
