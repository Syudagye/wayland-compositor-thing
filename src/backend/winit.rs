use std::time::Duration;

use smithay::{
    backend::{
        renderer::{
            damage::OutputDamageTracker, element::surface::WaylandSurfaceRenderElement,
            gles::GlesRenderer,
        },
        winit::{self, WinitEvent},
    },
    desktop::space::render_output,
    output::{Mode, Output, PhysicalProperties, Subpixel},
    reexports::calloop::{
        timer::{TimeoutAction, Timer},
        EventLoop,
    },
    utils::{Rectangle, Transform},
};

use crate::{state::ThingState, CalloopData};

pub fn run(
    event_loop: &mut EventLoop<CalloopData>,
    data: &mut CalloopData,
) -> Result<(), Box<dyn std::error::Error>> {
    let display = &mut data.display;
    let state = &mut data.state;

    let (mut backend, mut winit) = winit::init::<GlesRenderer>()?;

    let mode = Mode {
        size: backend.window_size().physical_size,
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
    let _global = output.create_global::<ThingState>(&display.handle());
    output.change_current_state(Some(mode), Some(Transform::Flipped180), None, Some((0, 0).into()));
    output.set_preferred(mode);

    state.space.map_output(&output, (0, 0));

    let mut damage_tracker = OutputDamageTracker::from_output(&output);

    std::env::set_var("WAYLAND_DISPLAY", &state.socket_name);

    event_loop
        .handle()
        .insert_source(Timer::immediate(), move |_instant, _, data| {
            // TODO: Maybe move this to an external function

            let display = &mut data.display;
            let state = &mut data.state;

            // Dispatch winit events
            winit
                .dispatch_new_events(|event| match event {
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
                })
                .unwrap();

            backend.bind().unwrap();

            render_output::<_, WaylandSurfaceRenderElement<GlesRenderer>, _, _>(
                &output,
                backend.renderer(),
                1.0,
                0,
                [&state.space],
                &[],
                &mut damage_tracker,
                [0.1, 0.1, 0.1, 1.0],
            )
            .unwrap();
            backend
                .submit(Some(&[Rectangle::from_loc_and_size(
                    (0, 0),
                    backend.window_size().physical_size,
                )]))
                .unwrap();

            state.space.elements().for_each(|window| {
                window.send_frame(
                    &output,
                    state.start_time.elapsed(),
                    Some(Duration::ZERO),
                    |_, _| Some(output.clone()),
                )
            });

            state.space.refresh();
            display.flush_clients().unwrap();

            // Reschedule 60 times per seconds
            TimeoutAction::ToDuration(Duration::from_millis(16))
        })?;

    Ok(())
}
