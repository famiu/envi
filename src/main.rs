use envi::app::{Application, UserEvent};
use envi::prelude::*;
use tracing_subscriber::filter::{EnvFilter, LevelFilter};
use winit::event_loop::EventLoop;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Setup tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::builder()
                .with_default_directive(LevelFilter::INFO.into())
                .from_env_lossy(),
        )
        .init();

    // Create async runtime
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("Failed to create runtime");

    info!("Creating application instance");
    let mut app = Application::new("Envi", &rt)?.with_icon("assets/icon.png");

    info!("Creating event loop");
    let event_loop = EventLoop::<UserEvent>::with_user_event().build()?;
    event_loop.set_control_flow(winit::event_loop::ControlFlow::Wait);

    info!("Running application");
    let _ = event_loop.run_app(&mut app);

    Ok(())
}
