use std::time::Duration;

use smithay::{
    backend::{
        renderer::{
            damage::OutputDamageTracker, element::surface::WaylandSurfaceRenderElement,
            gles::GlesRenderer,
        },
        winit::{self, WinitEvent, WinitEventLoop, WinitGraphicsBackend},
    },
    desktop::space::render_output,
    output::{Mode, Output, PhysicalProperties, Subpixel},
    reexports::{
        calloop::{
            timer::{TimeoutAction, Timer},
            EventLoop,
        },
        wayland_server::{Display, DisplayHandle},
    },
    utils::{Rectangle, Transform},
};
use tracing::error;

use crate::{state::ThingState, CalloopData};

pub fn run(
    event_loop: &mut EventLoop<CalloopData>,
    data: &mut CalloopData,
) -> Result<(), Box<dyn std::error::Error>> {
    let state = &mut data.state;

    let (mut backend, mut winit) = winit::init::<GlesRenderer>()?;

    let mode = Mode {
        size: backend.window_size(),
        refresh: 60_000,
    };

    let output = Output::new(
        "winit".to_string(),
        PhysicalProperties {
            size: (0, 0).into(),
            subpixel: Subpixel::Unknown,
            make: "Thing".into(),
            model: "Winit".into(),
        },
    );
    let _global = output.create_global::<ThingState>(&data.dh);
    output.change_current_state(
        Some(mode),
        Some(Transform::Flipped180),
        None,
        Some((0, 0).into()),
    );
    output.set_preferred(mode);

    state.space.map_output(&output, (0, 0));

    let mut damage_tracker = OutputDamageTracker::from_output(&output);

    std::env::set_var("WAYLAND_DISPLAY", &state.socket_name);

    event_loop
        .handle()
        .insert_source(Timer::immediate(), move |_instant, _, data| {
            dispatch(
                &mut data.dh,
                &mut data.state,
                &mut backend,
                &mut winit,
                &output,
                &mut damage_tracker,
            );
            // Draw 60 time a second
            TimeoutAction::ToDuration(Duration::from_millis(16))
        })?;

    Ok(())
}

fn dispatch(
    // display: &mut Display<ThingState>,
    dh: &mut DisplayHandle,
    state: &mut ThingState,
    backend: &mut WinitGraphicsBackend<GlesRenderer>,
    winit: &mut WinitEventLoop,
    output: &Output,
    damage_tracker: &mut OutputDamageTracker,
) {
    // Dispatch winit events
    let _dispatch_status = winit.dispatch_new_events(|event| match event {
        WinitEvent::Resized { size, scale_factor } => output.change_current_state(
            Some(Mode {
                size,
                refresh: 60_000,
            }),
            None,
            None,
            None,
        ),
        WinitEvent::Input(input) => state.process_input_event(input),
        _ => (),
    });

    // TODO: handle `PumpStatus::Exit` here

    backend.bind().unwrap();

    let render_result = render_output::<_, WaylandSurfaceRenderElement<GlesRenderer>, _, _>(
        &output,
        backend.renderer(),
        1.0,
        0,
        [&state.space],
        &[],
        damage_tracker,
        [0.0, 0.0, 0.0, 1.0],
    );
    if let Err(render_err) = render_result {
        return tracing::error!(err = ?render_err, "Error when rendering output.");
    }

    let swap_result = backend.submit(Some(&[Rectangle::from_loc_and_size(
        (0, 0),
        backend.window_size(),
    )]));
    if let Err(swap_err) = swap_result {
        return tracing::error!(err = ?swap_err, "Error when swapping backbuffer to window.");
    }

    state.space.elements().for_each(|window| {
        window.send_frame(
            &output,
            state.start_time.elapsed(),
            Some(Duration::ZERO),
            |_, _| Some(output.clone()),
        )
    });

    state.space.refresh();
    if let Err(err) = dh.flush_clients() {
        error!(?err, "Error when flushing clients");
    }

    backend.window().request_redraw();
}
