use std::{cell::Cell, collections::HashSet, rc::Rc, time::Duration};

use smithay::{
    backend::{
        allocator::{
            dmabuf::DmabufAllocator,
            gbm::{GbmAllocator, GbmDevice},
        },
        egl::{EGLContext, EGLDisplay},
        renderer::{
            damage::OutputDamageTracker, element::surface::WaylandSurfaceRenderElement,
            glow::GlowRenderer, Bind,
        },
        x11::{WindowBuilder, X11Backend, X11Event},
    },
    desktop::space::render_output,
    output::{Mode, Output, PhysicalProperties, Subpixel},
    reexports::calloop::{
        timer::{TimeoutAction, Timer},
        EventLoop,
    },
    utils::{DeviceFd, Rectangle, Size, Transform},
};

use crate::{state::ThingState, CalloopData};

pub fn run(
    event_loop: &mut EventLoop<CalloopData>,
    data: &mut CalloopData,
) -> Result<(), Box<dyn std::error::Error>> {
    let display = &mut data.display;
    let state = &mut data.state;

    // Creating the x11 backend

    let backend = X11Backend::new()?;
    let x_handle = backend.handle();

    let window = WindowBuilder::new()
        .title("X11")
        .build(&x_handle)
        .expect("Error building the x11 window");

    let (drm_node, fd) = x_handle.drm_node()?;

    let gbm = GbmDevice::new(DeviceFd::from(fd))?;

    let egl = EGLDisplay::new(gbm.clone()).expect("Failed to create EGLDisplay");
    let context = EGLContext::new(&egl).expect("Failed to create EGL Context");
    let modifiers: HashSet<_> = context
        .dmabuf_render_formats()
        .iter()
        .map(|fmt| fmt.modifier)
        .collect();

    let mut surface = x_handle.create_surface(
        &window,
        DmabufAllocator(GbmAllocator::new(
            gbm,
            smithay::backend::allocator::gbm::GbmBufferFlags::RENDERING,
        )),
        modifiers.into_iter(),
    )?;

    // Renderer

    let mut renderer =
        unsafe { GlowRenderer::new(context).expect("Failed to create glow renderer") };

    // Output creation

    let mode = Mode {
        size: {
            let size = window.size();
            (size.w as i32, size.h as i32).into()
        },
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

    // Event Loops

    let render = Rc::new(Cell::new(false));
    let render_clone = render.clone();

    let output_clone = output.clone();
    event_loop
        .handle()
        .insert_source(backend, move |event, _, data| match event {
            X11Event::Input(event) => data.state.process_input_event(event),
            X11Event::Resized {
                new_size,
                window_id: _,
            } => {
                let mode = Mode {
                    size: {
                        let size = new_size.to_physical(1);
                        (size.w as i32, size.h as i32).into()
                    },
                    refresh: 60_000,
                };

                output_clone.delete_mode(output_clone.current_mode().unwrap());
                output_clone.change_current_state(
                    Some(mode),
                    Some(Transform::Flipped180),
                    None,
                    Some((0, 0).into()),
                );
                output_clone.set_preferred(mode);

                state.space.map_output(&output_clone, (0, 0));
            }
            X11Event::PresentCompleted { .. } | X11Event::Refresh { .. } => render_clone.set(true),
            _ => (),
        })
        .unwrap();

    event_loop
        .handle()
        .insert_source(Timer::immediate(), move |event, _, data| {
            // TODO: Maybe move this to an external function

            let display = &mut data.display;
            let state = &mut data.state;

            if render.get() {
                let (buffer, age) = surface.buffer().unwrap();
                renderer.bind(buffer).unwrap();

                render_output::<_, WaylandSurfaceRenderElement<GlowRenderer>, _, _>(
                    &output,
                    &mut renderer,
                    1.0,
                    0,
                    [&state.space],
                    &[],
                    &mut damage_tracker,
                    [0.1, 0.1, 0.1, 1.0],
                )
                .unwrap();

                surface.submit().unwrap();

                state.space.elements().for_each(|window| {
                    window.send_frame(
                        &output,
                        state.start_time.elapsed(),
                        Some(Duration::ZERO),
                        |_, _| Some(output.clone()),
                    )
                });
            }

            state.space.refresh();
            display.flush_clients().unwrap();
            TimeoutAction::ToDuration(Duration::ZERO)
        })
        .unwrap();

    Ok(())
}
