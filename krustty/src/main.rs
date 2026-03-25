use winit::event_loop::EventLoop;

mod ansi;
mod grid;
mod pty;
mod terminal;
mod ui;

use crate::ui::Application;

fn main() -> anyhow::Result<()> {
    env_logger::init();

    let event_loop = EventLoop::with_user_event().build()?;
    let mut app = Application::new(&event_loop);
    event_loop.run_app(&mut app)?;
    Ok(())
}
