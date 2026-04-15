use std::{
    io::{Read, Write},
    sync::{Arc, Mutex},
    thread::{self, JoinHandle},
};

use crate::{terminal::Terminal, ui::Event};
use portable_pty::{Child, CommandBuilder, MasterPty, NativePtySystem, PtySize, PtySystem};
use rtrb::{Consumer, CopyToUninit, Producer, RingBuffer};

pub struct Pty {
    _pty_reader: JoinHandle<()>,
    _pty_writer: JoinHandle<anyhow::Result<()>>,
    child: Box<dyn Child + Send + Sync>,
    pub master: Box<dyn MasterPty + Send>,
    pub input: Producer<u8>,
}

impl Pty {
    pub fn spawn(
        cmd: &str,
        term: Arc<Mutex<Terminal>>,
        cols: u16,
        rows: u16,
    ) -> anyhow::Result<Self> {
        let pty = NativePtySystem::default().openpty(PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        })?;
        let mut cmd = CommandBuilder::new(cmd);
        cmd.env("TERM", "xterm-256color");
        let child = pty.slave.spawn_command(cmd)?;
        drop(pty.slave);

        let master = pty.master;
        let std_in = master.take_writer()?;
        let std_out = master.try_clone_reader()?;
        let (writer, reader) = RingBuffer::<u8>::new(4096);
        Ok(Self {
            _pty_reader: thread::spawn(move || read_pty(std_out, term)),
            _pty_writer: thread::spawn(move || write_pty(std_in, reader)),
            child,
            input: writer,
            master,
        })
    }

    pub fn close(&mut self) {
        self.child.kill().unwrap();
        let _status = self.child.wait().unwrap();
    }

    pub fn resize(&self, cols: u16, rows: u16) -> anyhow::Result<()> {
        let size = self.master.get_size()?;
        if size.cols != cols || size.rows != rows {
            self.master.resize(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
        } else {
            Ok(())
        }
    }

    pub fn send_input(&mut self, input: &str) {
        let bytes = input.as_bytes();
        if let Ok(mut chunk) = self.input.write_chunk_uninit(bytes.len()) {
            let (slice1, slice2) = chunk.as_mut_slices();
            let wrap = slice1.len();
            let _ = &bytes[..wrap].copy_to_uninit(slice1);
            let _ = &bytes[wrap..].copy_to_uninit(slice2);
            // SAFETY: bytes is initialized, and bytes.len() uninitialized bytes originally requested
            unsafe { chunk.commit(bytes.len()) };
        }
    }
}

pub fn write_pty(
    mut std_in: Box<dyn Write + Send>,
    mut reader: Consumer<u8>,
) -> anyhow::Result<()> {
    loop {
        let chunk = reader.read_chunk(reader.slots())?;
        let (slice1, slice2) = chunk.as_slices();
        if !slice1.is_empty() {
            std_in.write_all(slice1)?;
            if !slice2.is_empty() {
                std_in.write_all(slice2)?;
            }
        }
        chunk.commit_all();
    }
}

pub fn read_pty(mut std_out: Box<dyn Read + Send>, term: Arc<Mutex<Terminal>>) {
    let mut parser = vte::Parser::new();
    let mut buffer = [0u8; 1024];
    loop {
        match std_out.read(&mut buffer) {
            Ok(0) => break, // EOF
            Ok(n) => {
                let mut terminal = term
                    .lock()
                    .expect("Could not lock terminal while reading pty");
                #[cfg(debug_assertions)]
                println!("{:?}", String::from_utf8_lossy(&buffer[0..n]).chars());
                parser.advance(&mut *terminal, &buffer[..n]);
                let _ = terminal.event_loop.send_event(Event::WakeUp);
            }
            Err(e) => {
                eprintln!("Error reading from PTY: {}", e);
                break;
            }
        }
    }
}
