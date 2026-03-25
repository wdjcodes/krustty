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
                // Line Feed (LF), VT, FF
                term.grid.line_feed();
            }
            b'\r' => {
                // Carriage Return (CR)
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
    }

    fn put(&mut self, _byte: u8) {}

    fn unhook(&mut self) {}

    fn osc_dispatch(&mut self, _params: &[&[u8]], _bell_terminated: bool) {}

    fn csi_dispatch(
        &mut self,
        _params: &vte::Params,
        _intermediates: &[u8],
        _ignore: bool,
        _action: char,
    ) {
        println!("Csi Dispatch: {:?} action: {}", _params, _action);
    }

    fn esc_dispatch(&mut self, _intermediates: &[u8], _ignore: bool, _byte: u8) {}

    fn terminated(&self) -> bool {
        false
    }
}
