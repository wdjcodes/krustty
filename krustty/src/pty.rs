use std::{
    io::{Read, Write},
    thread::{self, JoinHandle},
};

use crate::ansi::AnsiParser;
use portable_pty::{Child, CommandBuilder, NativePtySystem, PtySize, PtySystem};
use rtrb::{Consumer, Producer, RingBuffer};

pub struct Pty {
    _pty_reader: JoinHandle<()>,
    _pty_writer: JoinHandle<anyhow::Result<()>>,
    child: Box<dyn Child + Send + Sync>,
    pub input: Producer<u8>,
}

impl Pty {
    pub fn spawn(cmd: &str, performer: AnsiParser) -> anyhow::Result<Self> {
        let pty = NativePtySystem::default().openpty(PtySize {
            rows: 24,
            cols: 80,
            pixel_width: 0,
            pixel_height: 0,
        })?;
        let cmd = CommandBuilder::new(cmd);
        let child = pty.slave.spawn_command(cmd)?;
        drop(pty.slave);

        let std_in = pty.master.take_writer()?;
        let std_out = pty.master.try_clone_reader()?;
        let (writer, reader) = RingBuffer::<u8>::new(4096);
        Ok(Self {
            _pty_reader: thread::spawn(move || read_pty(std_out, performer)),
            _pty_writer: thread::spawn(move || write_pty(std_in, reader)),
            child,
            input: writer,
        })
    }

    pub fn close(&mut self) {
        self.child.kill().unwrap();
        let _status = self.child.wait().unwrap();
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

pub fn read_pty(mut std_out: Box<dyn Read + Send>, mut performer: AnsiParser) {
    let mut parser = vte::Parser::new();
    let mut buffer = [0u8; 1024];
    loop {
        match std_out.read(&mut buffer) {
            Ok(0) => break, // EOF
            Ok(n) => {
                #[cfg(debug_assertions)]
                println!("{:?}", String::from_utf8_lossy(&buffer[0..n]).chars());
                parser.advance(&mut performer, &buffer[..n]);
            }
            Err(e) => {
                eprintln!("Error reading from PTY: {}", e);
                break;
            }
        }
    }
    // TODO: Add event_loop.send(Event::CloseRequested)
}
