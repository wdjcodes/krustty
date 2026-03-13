use rtrb::CopyToUninit;
use winit::event_loop::EventLoop;

mod ui;

use crate::{terminal::Terminal, ui::Application};

const MAX_LINE_LENGTH: usize = 4096;

mod terminal;
fn main() -> anyhow::Result<()> {
    env_logger::init();
    let mut term = Terminal::spawn("zsh");

    let mut buffer = String::with_capacity(MAX_LINE_LENGTH);
    let stdin = std::io::stdin();

    println!("You can now type commands for Bash (type 'exit' to quit):");
    let event_loop = EventLoop::with_user_event().build()?;
    let proxy = event_loop.create_proxy();
    // Main thread sends user input to the writer thread.
    std::thread::spawn(move || -> anyhow::Result<()> {
        loop {
            buffer.clear();
            let num = stdin.read_line(&mut buffer)?;
            if buffer.trim() == "exit" {
                break;
            }
            if let Ok(mut chunk) = term.input.write_chunk_uninit(num) {
                let (slice1, slice2) = chunk.as_mut_slices();
                let wrap = slice1.len();
                buffer.as_bytes()[..wrap].copy_to_uninit(slice1);
                buffer.as_bytes()[wrap..].copy_to_uninit(slice2);
                unsafe { chunk.commit(num) };
            };
        }
        term.close();
        let _ = proxy.send_event(ui::Event::CloseRequested);
        Ok(())
    });
    let mut app = Application::new(&event_loop);
    event_loop.run_app(&mut app)?;
    Ok(())
}
