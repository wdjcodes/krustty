use tracing_subscriber::EnvFilter;
use winit::event_loop::EventLoop;

mod color;
mod pty;
mod term;
mod ui;

use crate::ui::Application;

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("off,krustty=info")),
        )
        .with_line_number(true)
        .init();

    let event_loop = EventLoop::with_user_event().build()?;
    let mut app = Application::new(&event_loop);
    event_loop.run_app(&mut app)?;
    Ok(())
}
