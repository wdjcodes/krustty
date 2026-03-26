use std::sync::{Arc, Mutex};

use vte::Perform;

use crate::terminal::Terminal;

pub struct AnsiParser {
    term: Arc<Mutex<Terminal>>,
}

impl AnsiParser {
    pub fn new(term: Arc<Mutex<Terminal>>) -> Self {
        Self { term: term.clone() }
    }
}

impl Perform for AnsiParser {
    fn print(&mut self, c: char) {
        let mut term = self
            .term
            .lock()
            .expect("Failed to lock the terminal while printing ANSI");
        term.print(c);
    }

    fn execute(&mut self, byte: u8) {
        println!("execute: 0x{:02x}", byte);
        let mut term = self
            .term
            .lock()
            .expect("Failed to lock the terminal while executing ANSI");
        match byte {
            b'\n' => {
                term.grid.carriage_return();
                term.grid.line_feed();
            }
            b'\x0B' | b'\x0C' => {
                term.grid.line_feed();
            }
            b'\r' => {
                term.grid.carriage_return();
            }
            b'\x08' => {
                // Backspace (BS)
                term.grid.left();
            }
            b'\t' => {
                term.grid.advance_cursor(4);
            }
            //others Still need to be implemented
            _ => {}
        }
    }

    fn hook(&mut self, _params: &vte::Params, _intermediates: &[u8], _ignore: bool, _action: char) {
        println!("hook called");
    }

    fn put(&mut self, _byte: u8) {
        println!("put called");
    }

    fn unhook(&mut self) {
        println!("unhook called");
    }

    fn osc_dispatch(&mut self, _params: &[&[u8]], _bell_terminated: bool) {
        println!("osc called");
    }

    fn csi_dispatch(
        &mut self,
        params: &vte::Params,
        _intermediates: &[u8],
        _ignore: bool,
        action: char,
    ) {
        let mut term = self
            .term
            .lock()
            .expect("Failed to lock the terminal during CSI dispatch");
        match action {
            'D' => {
                let count = params.iter().next().and_then(|p| p.first()).unwrap_or(&0);
                term.grid.cursor.col = term.grid.cursor.col.saturating_sub(*count as usize);
            }
            // Erase in Line (EL)
            'K' => {
                // If no parameter is provided, it defaults to 0
                let mode = params.iter().next().and_then(|p| p.first()).unwrap_or(&0);

                match mode {
                    0 => {
                        term.clear_to_end();
                    }
                    1 => {
                        term.clear_to_start();
                    }
                    2 => {
                        term.clear_line();
                    }
                    _ => {} // Ignore unsupported modes
                }
            }

            // Cursor Horizontal Absolute (CHA)
            'G' | '`' => {
                let target_col = params.iter().next().and_then(|p| p.first()).unwrap_or(&1);

                // VT coordinates are 1-indexed. Rust arrays are 0-indexed.
                let zero_indexed_col = (*target_col as usize).saturating_sub(1);
                term.grid.cursor.col = zero_indexed_col.min(term.grid.width.saturating_sub(1));
            }

            _ => {
                println!("Csi Dispatch: {:?}{:?}{}", _intermediates, params, action);
            }
        }
    }

    fn esc_dispatch(&mut self, _intermediates: &[u8], _ignore: bool, _byte: u8) {
        println!("esc called");
    }

    fn terminated(&self) -> bool {
        false
    }
}
