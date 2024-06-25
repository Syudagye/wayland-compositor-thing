use smithay::reexports::{
    calloop::EventLoop,
    wayland_server::{Display, DisplayHandle},
};
use state::ThingState;
use tracing::info;

mod backend;
mod state;

pub struct CalloopData {
    state: ThingState,
    dh: DisplayHandle,
}

fn main() {
    if let Ok(env_filter) = tracing_subscriber::EnvFilter::try_from_default_env() {
        tracing_subscriber::fmt()
            .compact()
            .with_env_filter(env_filter)
            .init();
    } else {
        tracing_subscriber::fmt().compact().init();
    }

    let mut event_loop: EventLoop<CalloopData> =
        EventLoop::try_new().expect("unable to initialize event loop");
    let display: Display<ThingState> = Display::new().expect("unable to initialize display");
    let dh = display.handle();

    let state = ThingState::new(event_loop.handle(), display);

    let mut data = CalloopData { state, dh };

    //TODO: Auto-detect backend
    backend::winit::run(&mut event_loop, &mut data).unwrap();

    event_loop
        .run(None, &mut data, move |_| {
            // Smallvil is running
        })
    .expect("Unable to start event loop");

    info!("compositor closed");
}
