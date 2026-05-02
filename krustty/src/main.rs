use winit::event_loop::EventLoop;

mod color;
mod grid;
mod pty;
mod terminal;
mod ui;

use crate::ui::Application;

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt().with_line_number(true).init();

    let event_loop = EventLoop::with_user_event().build()?;
    let mut app = Application::new(&event_loop);
    event_loop.run_app(&mut app)?;
    Ok(())
}
