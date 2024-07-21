use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::Duration,
};

use smithay::{
    backend::{
        input::ButtonState,
        renderer::{
            damage::OutputDamageTracker, element::surface::WaylandSurfaceRenderElement,
            test::DummyRenderer,
        },
    },
    desktop::space::render_output,
    input::{
        pointer::{ButtonEvent, MotionEvent},
        touch::DownEvent,
    },
    output::{Mode, Output, PhysicalProperties, Subpixel},
    reexports::{
        calloop::{
            channel::{Channel, Event},
            timer::{TimeoutAction, Timer},
            EventLoop, LoopSignal,
        },
        wayland_server::{Client, Display, DisplayHandle, Resource},
    },
    utils::{Clock, Transform, SERIAL_COUNTER},
    wayland::seat::WaylandFocus,
};
use wayland_compositor_thing::{
    backend::CalloopData,
    state::{ClientState, ThingState},
};

use crate::WlcsEvent;

pub fn run(channel: Channel<WlcsEvent>) {
    let display = Display::new().expect("Unable to create display");
    let dh = display.handle();
    let mut event_loop = EventLoop::try_new().expect("Unable to create event loop");
    let mut state = ThingState::new(event_loop.handle(), display);

    let clients: Arc<Mutex<HashMap<i32, Client>>> = Arc::new(Mutex::new(HashMap::new()));

    let mut renderer = DummyRenderer::new();

    let mode = Mode {
        size: (800, 600).into(),
        refresh: 60_000,
    };

    let output = Output::new(
        "wlcs".to_string(),
        PhysicalProperties {
            size: (0, 0).into(),
            subpixel: Subpixel::Unknown,
            make: "Thing".into(),
            model: "Winit".into(),
        },
    );
    let _ = output.create_global::<ThingState>(&dh);
    output.change_current_state(
        Some(mode),
        Some(Transform::Flipped180),
        None,
        Some((0, 0).into()),
    );
    output.set_preferred(mode);

    // Could setup EGL here, but not sure

    state.space.map_output(&output, (0, 0));

    let mut damage_tracker = OutputDamageTracker::from_output(&output);

    std::env::set_var("WAYLAND_DISPLAY", &state.socket_name);

    let loop_signal = event_loop.get_signal();
    let _ = event_loop
        .handle()
        .insert_source(Timer::immediate(), move |_instant, _, data| {
            render(
                &mut data.dh,
                &mut data.state,
                &mut renderer,
                &output,
                &mut damage_tracker,
            );
            // Draw 60 time a second
            TimeoutAction::ToDuration(Duration::from_millis(16))
        });

    event_loop
        .handle()
        .insert_source(channel, move |event, _, data| match event {
            Event::Msg(ev) => handle_event(ev, data, loop_signal.clone(), clients.clone()),
            Event::Closed => loop_signal.stop(),
        })
        .expect("Unable to insert event source");

    let mut data = CalloopData { state, dh };
    event_loop
        .run(None, &mut data, move |_| {})
        .expect("Unable to start event loop");
}

fn handle_event(
    event: WlcsEvent,
    data: &mut CalloopData,
    loop_signal: LoopSignal,
    clients: Arc<Mutex<HashMap<i32, Client>>>,
) {
    let Ok(mut clients) = clients.lock() else {
        return;
    };
    let state = &mut data.state;

    match event {
        WlcsEvent::Exit => loop_signal.stop(),
        WlcsEvent::NewClient { stream, client_id } => {
            let client = data
                .dh
                .insert_client(stream, Arc::new(ClientState::default()));
            if let Ok(c) = client {
                clients.insert(client_id, c);
            }
        }
        WlcsEvent::PositionWindow {
            client_id,
            surface_id,
            location,
        } => {
            let client = clients.get(&client_id);
            let toplevel = data.state.space.elements().find(|w| {
                if let Some(surface) = w.wl_surface().map(|s| s.into_owned()) {
                    data.dh.get_client(surface.id()).ok().as_ref() == client
                        && surface.id().protocol_id() == surface_id
                } else {
                    false
                }
            });
            if let Some(toplevel) = toplevel.cloned() {
                // set its location
                data.state.space.map_element(toplevel, location, false);
            }
        }

        // Pointer
        WlcsEvent::PointerMoveRelative { delta, .. } => {
            let ptr = state.pointer_handle.clone();
            let serial = SERIAL_COUNTER.next_serial();
            let time = Duration::from(state.clock.now()).as_millis() as u32;

            let location = ptr.current_location() + delta;
            let focus = state.surface_under(location);

            ptr.motion(
                state,
                focus,
                &MotionEvent {
                    location,
                    serial,
                    time,
                },
            );
            ptr.frame(state);
        }
        WlcsEvent::PointerMoveAbsolute { location, .. } => {
            let ptr = state.pointer_handle.clone();
            let serial = SERIAL_COUNTER.next_serial();
            let time = Duration::from(state.clock.now()).as_millis() as u32;

            let focus = state.surface_under(location);

            ptr.motion(
                state,
                focus,
                &MotionEvent {
                    location,
                    serial,
                    time,
                },
            );
            ptr.frame(state);
        }
        WlcsEvent::PointerButtonUp { button_id, .. } => {
            let ptr = state.pointer_handle.clone();
            let serial = SERIAL_COUNTER.next_serial();
            let time = Duration::from(state.clock.now()).as_millis() as u32;

            ptr.button(
                state,
                &ButtonEvent {
                    serial,
                    time,
                    button: button_id as u32,
                    state: ButtonState::Pressed,
                },
            );
            ptr.frame(state);
        }
        WlcsEvent::PointerButtonDown { button_id, .. } => {
            let ptr = state.pointer_handle.clone();
            let serial = SERIAL_COUNTER.next_serial();
            let time = Duration::from(state.clock.now()).as_millis() as u32;

            ptr.button(
                state,
                &ButtonEvent {
                    serial,
                    time,
                    button: button_id as u32,
                    state: ButtonState::Pressed,
                },
            );
            ptr.frame(state);
        }

        _ => (),
    }
}

fn render(
    dh: &mut DisplayHandle,
    state: &mut ThingState,
    renderer: &mut DummyRenderer,
    output: &Output,
    damage_tracker: &mut OutputDamageTracker,
) {
    let _ = render_output::<_, WaylandSurfaceRenderElement<DummyRenderer>, _, _>(
        output,
        renderer,
        1.0,
        0,
        [&state.space],
        &[],
        damage_tracker,
        [0.0, 0.0, 0.0, 1.0],
    );

    state.space.elements().for_each(|window| {
        window.send_frame(
            &output,
            state.start_time.elapsed(),
            Some(Duration::ZERO),
            |_, _| Some(output.clone()),
        )
    });

    state.space.refresh();
    state.popup_manager.cleanup();
    let _ = dh.flush_clients();
}
