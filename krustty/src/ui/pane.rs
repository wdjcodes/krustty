use std::{
    cell::RefCell,
    rc::Rc,
    sync::{Arc, Mutex},
};

use palette::WithAlpha;
use winit::event_loop::EventLoopProxy;

use crate::{
    pty::Pty,
    term::{Terminal, cursor::Cursor, grid::CellFlags},
    ui::{
        Event, GpuHandle,
        cursor::CursorRenderer,
        font::GlyphCache,
        grid::{CellInstance, GridRenderer},
    },
};

pub struct Pane {
    cursor_render: CursorRenderer,
    grid_render: GridRenderer,
    pub pty: Pty,
    pub term: Arc<Mutex<Terminal>>,
    cache: Rc<RefCell<GlyphCache>>,
    scroll_queued: f64,
    /// The number of rows scrolled into the scrollback buffer
    scroll_rows: usize,
    /// The height of the viewport in rows
    height_rows: usize,
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
            pty,
            term,
            cache,
            scroll_queued: 0.0,
            scroll_rows: 0,
            height_rows: rows,
        }
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        let rows = (height / CELL_HEIGHT as u32) as usize;
        let cols = (width / CELL_WIDTH as u32) as usize;
        let mut term = self
            .term
            .lock()
            .expect("Failed to lock terminal during resize");

        term.resize(rows, cols);
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

        // Apply queued scroll
        if self.scroll_queued > 1.0 || self.scroll_queued < -1.0 {
            let rows = self.scroll_queued.trunc();
            self.scroll_queued -= rows;
            self.scroll_rows = self
                .scroll_rows
                .saturating_add_signed(rows as isize)
                .clamp(0, term.grid.rows().saturating_sub(self.height_rows));
        }

        let bottom_row = grid.rows().saturating_sub(self.scroll_rows);
        let top_row = bottom_row.saturating_sub(self.height_rows);
        log::debug!(
            "Rendering rows: {}..{}",
            bottom_row,
            std::cmp::min(top_row, bottom_row)
        );

        for i in top_row..std::cmp::min(bottom_row, grid.rows()) {
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
                    screen_pos: [j as f32, (i - top_row) as f32],
                    atlas_uv: [ax, ay, az, aw],
                    fg_color,
                    bg_color,
                });
            }
        }

        let mut cursor: Cursor = term.cursor;

        if cursor.row() <= self.height_rows.saturating_sub(self.scroll_rows) {
            cursor.set_from_point((cursor.row() + self.scroll_rows, cursor.col()));
            self.cursor_render.set_cursor(Some(cursor));
        } else {
            self.cursor_render.set_cursor(None);
        }
    }

    pub fn render(&mut self, view: &wgpu::TextureView, encoder: &mut wgpu::CommandEncoder) {
        self.grid_render.render_pass(view, encoder);
        self.cursor_render.render_pass(view, encoder);
    }

    /// Queue scroll to be applied in a future render pass
    pub fn queue_scroll(&mut self, rows: f64) -> f64 {
        self.scroll_queued += rows;
        self.scroll_queued
    }
}
