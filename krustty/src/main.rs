use winit::event_loop::EventLoop;

mod grid;
mod pty;
mod ui;

use crate::ui::Application;

const MAX_LINE_LENGTH: usize = 4096;

fn main() -> anyhow::Result<()> {
    env_logger::init();

    let mut buffer = String::with_capacity(MAX_LINE_LENGTH);
    let stdin = std::io::stdin();

    let event_loop = EventLoop::with_user_event().build()?;
    let proxy = event_loop.create_proxy();
    let mut app = Application::new(&event_loop);
    println!("You can now type commands for Bash (type 'exit' to quit):");
    // Main thread sends user input to the writer thread.
    std::thread::spawn(move || -> anyhow::Result<()> {
        loop {
            buffer.clear();
            stdin.read_line(&mut buffer)?;
            if buffer.trim() == "exit" {
                break;
            }
            let _ = proxy.send_event(ui::Event::Input(buffer.clone()));
        }
        let _ = proxy.send_event(ui::Event::CloseRequested);
        Ok(())
    });
    event_loop.run_app(&mut app)?;
    Ok(())
}
