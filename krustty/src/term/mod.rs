use log::info;
use vte::Perform;
use winit::event_loop::EventLoopProxy;

pub mod cursor;
pub mod grid;

use crate::{
    color::{
        Color,
        Component::{Bg, Fg},
        NamedColor,
    },
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
        if self.cursor.is_at_bottom() {
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
        let mut cursor = Cursor::new(self.cursor.max_row(), self.cursor.max_col());
        for _ in 1..cursor.max_row() {
            self.grid.clear_line(&cursor);
            cursor.down(1);
        }
    }

    pub fn clear_screen_to_end(&mut self) {
        let point = self.cursor.as_point();
        self.grid.clear_line_to_end(&self.cursor);
        if self.cursor.is_at_bottom() {
            return;
        }
        while !self.cursor.is_at_bottom() {
            self.grid.clear_line(&self.cursor);
            self.cursor.down(1);
        }
        self.grid.clear_line(&self.cursor);
        self.cursor.set_from_point(point);
    }

    #[inline]
    pub fn set_fg<T: Into<Color>>(&mut self, fg: T) {
        self.template_cell.fg = fg.into();
    }

    #[inline]
    pub fn set_bg<T: Into<Color>>(&mut self, bg: T) {
        self.template_cell.bg = bg.into();
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
            if self.cursor.is_at_bottom() {
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
            b'\n' => {
                self.line_feed();
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
                            self.set_fg(Color::Default(Fg));
                            self.set_bg(Color::Default(Bg));
                            self.set_inverse(false);
                        }
                        7 => self.set_inverse(true),
                        27 => self.set_inverse(false),
                        // Foreground
                        30 => self.set_fg(NamedColor::Black),
                        31 => self.set_fg(NamedColor::Red),
                        32 => self.set_fg(NamedColor::Green),
                        33 => self.set_fg(NamedColor::Yellow),
                        34 => self.set_fg(NamedColor::Blue),
                        35 => self.set_fg(NamedColor::Magenta),
                        36 => self.set_fg(NamedColor::Cyan),
                        37 => self.set_fg(NamedColor::White),
                        39 => self.set_fg(Color::Default(Fg)),
                        // Background
                        40 => self.set_bg(NamedColor::Black),
                        41 => self.set_bg(NamedColor::Red),
                        42 => self.set_bg(NamedColor::Green),
                        43 => self.set_bg(NamedColor::Yellow),
                        44 => self.set_bg(NamedColor::Blue),
                        45 => self.set_bg(NamedColor::Magenta),
                        46 => self.set_bg(NamedColor::Cyan),
                        47 => self.set_bg(NamedColor::White),
                        49 => self.set_bg(Color::Default(Bg)),
                        // Bright Foreground
                        90 => self.set_fg(NamedColor::BrightBlack),
                        91 => self.set_fg(NamedColor::BrightRed),
                        92 => self.set_fg(NamedColor::BrightGreen),
                        93 => self.set_fg(NamedColor::BrightYellow),
                        94 => self.set_fg(NamedColor::BrightBlue),
                        95 => self.set_fg(NamedColor::BrightMagenta),
                        96 => self.set_fg(NamedColor::BrightCyan),
                        97 => self.set_fg(NamedColor::BrightWhite),
                        // Bright Background
                        100 => self.set_bg(NamedColor::BrightBlack),
                        101 => self.set_bg(NamedColor::BrightRed),
                        102 => self.set_bg(NamedColor::BrightGreen),
                        103 => self.set_bg(NamedColor::BrightYellow),
                        104 => self.set_bg(NamedColor::BrightBlue),
                        105 => self.set_bg(NamedColor::BrightMagenta),
                        106 => self.set_bg(NamedColor::BrightCyan),
                        107 => self.set_bg(NamedColor::BrightWhite),

                        code => {
                            info!(
                                "Unsupported SGR: Code: {} Intermediates: {:?} Params: {:?} Action: {}",
                                code, intermediates, params, action
                            );
                        }
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

    fn esc_dispatch(&mut self, intermediates: &[u8], ignore: bool, byte: u8) {
        match byte {
            b'D' => {
                self.line_feed();
            }
            b'E' => {
                self.line_feed();
                self.carriage_return();
            }
            b'M' => {
                if self.cursor.is_at_top() {
                    self.grid.pop_bottom();
                } else {
                    self.cursor.up(1);
                }
            }
            _ => {
                info!(
                    "Unsupported ESC: Intermediates: {:?} Ignore: {:?} Byte: {}",
                    intermediates, ignore, byte as char
                );
            }
        }
    }

    fn terminated(&self) -> bool {
        false
    }
}
