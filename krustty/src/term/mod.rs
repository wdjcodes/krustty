use log::info;
use vte::Perform;
use winit::event_loop::EventLoopProxy;

pub mod cursor;
pub mod grid;

use crate::{
    color::{DEFAULT_COLORS, Rgb},
    term::grid::{CellFlags, GridCell},
    ui::Event,
};
use cursor::Cursor;
use grid::Grid;

pub struct Terminal {
    pub cursor: Cursor,
    pub event_loop: EventLoopProxy<Event>,
    response_buffer: Vec<u8>,
    /// Template to use when creating a new cell
    pub template_cell: GridCell,
    pub grid: Grid,
}

impl Terminal {
    pub fn new(event_loop: EventLoopProxy<Event>, width: usize, height: usize) -> Self {
        let grid = Grid::new(width, height, 1000);
        Self {
            event_loop,
            response_buffer: vec![],
            cursor: Cursor::new(height, width),
            template_cell: Default::default(),
            grid,
        }
    }

    pub fn take_response(&mut self) -> Vec<u8> {
        std::mem::take(&mut self.response_buffer)
    }

    pub fn line_feed(&mut self) {
        if self.cursor.is_row_max() {
            self.grid.push_row();
        } else {
            self.cursor.down(1);
        }
        self.cursor.will_wrap = false;
    }

    pub fn carriage_return(&mut self) {
        if self.cursor.will_wrap {
            self.cursor.up(1);
            self.cursor.will_wrap = false;
        }
        self.cursor.home_col();
    }

    pub fn clear_screen(&mut self) {
        for _ in 1..=self.cursor.max_row() {
            if self.cursor.is_row_max() {
                self.line_feed();
            } else {
                self.grid.clear_line(&self.cursor);
                self.cursor.down(1);
            }
        }
        self.cursor.home();
    }

    pub fn clear_screen_to_end(&mut self) {
        let point = self.cursor.as_point();
        self.grid.clear_line_to_end(&self.cursor);
        if self.cursor.is_row_max() {
            return;
        }
        while !self.cursor.is_row_max() {
            self.grid.clear_line(&self.cursor);
            self.cursor.down(1);
        }
        self.grid.clear_line(&self.cursor);
        self.cursor.set_from_point(point);
    }

    #[inline]
    pub fn set_fg(&mut self, fg: Rgb) {
        self.template_cell.fg = fg;
    }

    #[inline]
    pub fn set_bg(&mut self, bg: Rgb) {
        self.template_cell.bg = bg;
    }

    #[inline]
    pub fn set_inverse(&mut self, inverse: bool) {
        if inverse {
            self.template_cell.flags |= CellFlags::INVERSE;
        } else {
            self.template_cell.flags &= !CellFlags::INVERSE;
        }
    }

    pub fn resize(&mut self, rows: usize, cols: usize) {
        self.cursor.resize(rows, cols);
        self.grid.resize(rows, cols, &mut self.cursor);
    }
}

impl Perform for Terminal {
    fn print(&mut self, c: char) {
        self.grid.write_at_cursor(
            &mut self.cursor,
            GridCell {
                c,
                ..self.template_cell
            },
        );
        if self.cursor.is_col_max() {
            if self.cursor.is_row_max() {
                self.grid.push_row();
            }
            self.cursor.down(1);
            self.cursor.home_col();
            self.cursor.will_wrap = true;
        } else {
            self.cursor.right(1);
        }
    }

    fn execute(&mut self, byte: u8) {
        match byte {
            b'\n' | b'D' => {
                self.line_feed();
            }
            b'E' => {
                self.line_feed();
                self.carriage_return();
            }
            b'\x0B' | b'\x0C' => {
                self.line_feed();
            }
            b'\r' => {
                self.carriage_return();
            }
            b'\x08' => {
                // Backspace (BS)
                self.cursor.left(1);
            }
            b'\t' => {
                self.cursor.right(4);
            }
            //others Still need to be implemented
            byte => info!("Unsupported control character: 0x{:2x}", byte),
        }
    }

    fn hook(&mut self, _params: &vte::Params, _intermediates: &[u8], _ignore: bool, _action: char) {
    }

    fn put(&mut self, _byte: u8) {}

    fn unhook(&mut self) {}

    fn osc_dispatch(&mut self, _params: &[&[u8]], _bell_terminated: bool) {}

    fn csi_dispatch(
        &mut self,
        params: &vte::Params,
        intermediates: &[u8],
        _ignore: bool,
        action: char,
    ) {
        let grid = &mut self.grid;
        match action {
            'c' => {
                let code = params.iter().next().and_then(|p| p.first()).unwrap_or(&0);
                match code {
                    0 => {
                        self.response_buffer.extend_from_slice(b"\x1b[?62;22c");
                        let _ = self.event_loop.send_event(Event::SendPtyResponse);
                    }
                    code => info!(
                        "Unsupported CSI: Intermediates: {:?} Params: {:?} Action: {}",
                        intermediates, code, action
                    ),
                }
            }
            'm' => {
                for param in params {
                    let code = param.first().unwrap_or(&255);
                    match code {
                        0 => {
                            self.set_fg(DEFAULT_COLORS.fg.into_format());
                            self.set_bg(DEFAULT_COLORS.bg.into_format());
                        }
                        7 => self.set_inverse(true),
                        27 => self.set_inverse(false),
                        // Foreground
                        30 => self.set_fg(DEFAULT_COLORS.black.into_format()),
                        31 => self.set_fg(DEFAULT_COLORS.red.into_format()),
                        32 => self.set_fg(DEFAULT_COLORS.green.into_format()),
                        33 => self.set_fg(DEFAULT_COLORS.yellow.into_format()),
                        34 => self.set_fg(DEFAULT_COLORS.blue.into_format()),
                        35 => self.set_fg(DEFAULT_COLORS.purple.into_format()),
                        36 => self.set_fg(DEFAULT_COLORS.cyan.into_format()),
                        37 => self.set_fg(DEFAULT_COLORS.white.into_format()),
                        39 => self.set_fg(DEFAULT_COLORS.white.into_format()),
                        // Background
                        40 => self.set_bg(DEFAULT_COLORS.black.into_format()),
                        41 => self.set_bg(DEFAULT_COLORS.red.into_format()),
                        42 => self.set_bg(DEFAULT_COLORS.green.into_format()),
                        43 => self.set_bg(DEFAULT_COLORS.yellow.into_format()),
                        44 => self.set_bg(DEFAULT_COLORS.blue.into_format()),
                        45 => self.set_bg(DEFAULT_COLORS.purple.into_format()),
                        46 => self.set_bg(DEFAULT_COLORS.cyan.into_format()),
                        47 => self.set_bg(DEFAULT_COLORS.white.into_format()),
                        49 => self.set_bg(DEFAULT_COLORS.black.into_format()),
                        // Bright Foreground
                        90 => self.set_fg(DEFAULT_COLORS.bright_black.into_format()),
                        91 => self.set_fg(DEFAULT_COLORS.bright_red.into_format()),
                        92 => self.set_fg(DEFAULT_COLORS.bright_green.into_format()),
                        93 => self.set_fg(DEFAULT_COLORS.bright_yellow.into_format()),
                        94 => self.set_fg(DEFAULT_COLORS.bright_blue.into_format()),
                        95 => self.set_fg(DEFAULT_COLORS.bright_purple.into_format()),
                        96 => self.set_fg(DEFAULT_COLORS.bright_cyan.into_format()),
                        97 => self.set_fg(DEFAULT_COLORS.bright_white.into_format()),
                        // Bright Background
                        100 => self.set_bg(DEFAULT_COLORS.bright_black.into_format()),
                        101 => self.set_bg(DEFAULT_COLORS.bright_red.into_format()),
                        102 => self.set_bg(DEFAULT_COLORS.bright_green.into_format()),
                        103 => self.set_bg(DEFAULT_COLORS.bright_yellow.into_format()),
                        104 => self.set_bg(DEFAULT_COLORS.bright_blue.into_format()),
                        105 => self.set_bg(DEFAULT_COLORS.bright_purple.into_format()),
                        106 => self.set_bg(DEFAULT_COLORS.bright_cyan.into_format()),
                        107 => self.set_bg(DEFAULT_COLORS.bright_white.into_format()),

                        code => {
                            info!(
                                "Unsupported SGR: Code: {} Intermediates: {:?} Params: {:?} Action: {}",
                                code, intermediates, params, action
                            );
                        }
                    }
                }
            }
            'A' => {
                let mut count = params.iter().next().and_then(|p| p.first()).unwrap_or(&1);
                count = if *count == 0 { &1 } else { count };
                self.cursor.up(*count as usize);
            }
            'B' => {
                let mut count = params.iter().next().and_then(|p| p.first()).unwrap_or(&0);
                count = if *count == 0 { &1 } else { count };
                self.cursor.down(*count as usize);
            }
            'C' => {
                let mut count = params.iter().next().and_then(|p| p.first()).unwrap_or(&0);
                count = if *count == 0 { &1 } else { count };
                self.cursor.right(*count as usize)
            }
            'D' => {
                let mut count = params.iter().next().and_then(|p| p.first()).unwrap_or(&0);
                count = if *count == 0 { &1 } else { count };
                self.cursor.left(*count as usize);
            }
            // Cursor Horizontal Absolute (CHA)
            'G' | '`' => {
                let target_col = params.iter().next().and_then(|p| p.first()).unwrap_or(&1);
                self.cursor.set_col(*target_col as usize);
            }
            // Position cursor [row;column]
            'H' | 'f' => {
                let point = match params.iter().collect::<Vec<&[u16]>>().as_slice() {
                    [row, col, ..] => {
                        if let Some(row) = row.first()
                            && let Some(col) = col.first()
                        {
                            (*row as usize, *col as usize)
                        } else {
                            log::debug!("Missing row or column");
                            return;
                        }
                    }
                    _ => (1, 1),
                };
                self.cursor.set_from_point(point);
            }
            'J' => {
                let mode = params.iter().next().and_then(|p| p.first()).unwrap_or(&0);
                match mode {
                    0 => self.clear_screen_to_end(),
                    2 => self.clear_screen(),
                    _ => {
                        info!(
                            "Unsupported CSI: Intermediates: {:?} Params: {:?} Action: {}",
                            intermediates, params, action
                        );
                    }
                }
            }
            // Erase in Line (EL)
            'K' => {
                // If no parameter is provided, it defaults to 0
                let mode = params.iter().next().and_then(|p| p.first()).unwrap_or(&0);

                match mode {
                    0 => {
                        grid.clear_line_to_end(&self.cursor);
                    }
                    1 => {
                        grid.clear_line_to_start(&self.cursor);
                    }
                    2 => {
                        grid.clear_line(&self.cursor);
                    }
                    _ => {
                        info!(
                            "Unsupported CSI: Intermediates: {:?} Params: {:?} Action: {}",
                            intermediates, params, action
                        );
                    }
                }
            }
            _ => {
                info!(
                    "Unsupported CSI: Intermediates: {:?} Params: {:?} Action: {}",
                    intermediates, params, action
                );
            }
        }
    }

    fn esc_dispatch(&mut self, _intermediates: &[u8], _ignore: bool, _byte: u8) {}

    fn terminated(&self) -> bool {
        false
    }
}
