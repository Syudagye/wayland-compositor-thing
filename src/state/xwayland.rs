use std::process::Stdio;

use smithay::{
    delegate_xwayland_shell,
    desktop::{space::SpaceElement, Window},
    reexports::{calloop::LoopHandle, wayland_server::DisplayHandle},
    utils::{Logical, Rectangle, SERIAL_COUNTER},
    wayland::xwayland_shell::{XWaylandShellHandler, XWaylandShellState},
    xwayland::{
        xwm::{Reorder, ResizeEdge, XwmId},
        X11Surface, X11Wm, XWayland, XWaylandEvent, XwmHandler,
    },
};
use tracing::{debug, error, info};

use crate::CalloopData;

use super::{
    xdg_shell::{move_grab, resize_grab},
    ThingState,
};

pub fn setup(dh: &DisplayHandle, loop_handle: LoopHandle<'static, CalloopData>) {
    let Ok((xwayland, xw_client)) = XWayland::spawn::<&str, &str, _, _>(
        &dh,
        None,
        [],
        true,
        Stdio::null(),
        std::io::stderr(),
        |_user_data| {},
    ) else {
        return error!("Unable to spawn XWayland");
    };

    let res = loop_handle.insert_source(xwayland, move |event, _, data| match event {
        XWaylandEvent::Ready {
            x11_socket,
            display_number,
        } => {
            info!(
                "XWayland server started successfully with display number {}",
                display_number
            );
            std::env::set_var("DISPLAY", format!(":{}", display_number.to_string()));
            match X11Wm::start_wm(
                data.state.loop_handle.clone(),
                x11_socket,
                xw_client.clone(),
            ) {
                Ok(xwm) => data.state.xwm = Some(xwm),
                Err(err) => error!(?err, "Unable to start X11 Window Manager"),
            }
        }
        XWaylandEvent::Error => {
            error!("XWayland exited unexpectedly on startup");
        }
    });

    if let Err(err) = res {
        error!(
            ?err,
            "Error when inserting xwayland event source to the loop"
        );
    }
}

impl XwmHandler for CalloopData {
    fn xwm_state(&mut self, xwm: XwmId) -> &mut X11Wm {
        self.state.xwm_state(xwm)
    }

    fn new_window(&mut self, xwm: XwmId, window: X11Surface) {
        self.state.new_window(xwm, window)
    }

    fn new_override_redirect_window(&mut self, xwm: XwmId, window: X11Surface) {
        self.state.new_override_redirect_window(xwm, window)
    }

    fn map_window_request(&mut self, xwm: XwmId, surface: X11Surface) {
        self.state.map_window_request(xwm, surface)
    }

    fn mapped_override_redirect_window(&mut self, xwm: XwmId, surface: X11Surface) {
        self.state.mapped_override_redirect_window(xwm, surface)
    }

    fn unmapped_window(&mut self, xwm: XwmId, window: X11Surface) {
        self.state.unmapped_window(xwm, window)
    }

    fn destroyed_window(&mut self, xwm: XwmId, window: X11Surface) {
        self.state.destroyed_window(xwm, window)
    }

    fn configure_request(
        &mut self,
        xwm: XwmId,
        window: X11Surface,
        x: Option<i32>,
        y: Option<i32>,
        w: Option<u32>,
        h: Option<u32>,
        reorder: Option<Reorder>,
    ) {
        self.state
            .configure_request(xwm, window, x, y, w, h, reorder)
    }

    fn configure_notify(
        &mut self,
        xwm: XwmId,
        window: X11Surface,
        geometry: Rectangle<i32, Logical>,
        above: Option<u32>,
    ) {
        self.state.configure_notify(xwm, window, geometry, above)
    }

    fn resize_request(
        &mut self,
        xwm: XwmId,
        surface: X11Surface,
        button: u32,
        resize_edge: ResizeEdge,
    ) {
        self.state.resize_request(xwm, surface, button, resize_edge)
    }

    fn move_request(&mut self, xwm: XwmId, surface: X11Surface, button: u32) {
        self.state.move_request(xwm, surface, button)
    }
}

impl XWaylandShellHandler for CalloopData {
    fn xwayland_shell_state(&mut self) -> &mut XWaylandShellState {
        &mut self.state.xw_shell_state
    }
}

impl XwmHandler for ThingState {
    fn xwm_state(&mut self, _xwm: XwmId) -> &mut X11Wm {
        self.xwm.as_mut().unwrap()
    }

    fn new_window(&mut self, xwm: XwmId, surface: X11Surface) {
        let window = Window::new_x11_window(surface);
        self.space.map_element(window, (0, 0), true);
    }

    fn new_override_redirect_window(&mut self, xwm: XwmId, window: X11Surface) {
        <Self as XwmHandler>::new_window(self, xwm, window);
    }

    fn map_window_request(&mut self, xwm: XwmId, window: X11Surface) {
        let window = self
            .space
            .elements()
            .filter_map(|w| w.x11_surface())
            .find(|&s| s == &window);

        if let Some(window) = window {
            window.set_mapped(true);
            window.set_activate(true);
        }
    }

    fn mapped_override_redirect_window(&mut self, xwm: XwmId, window: X11Surface) {}

    fn unmapped_window(&mut self, xwm: XwmId, window: X11Surface) {
        let window = self
            .space
            .elements()
            .filter_map(|w| w.x11_surface())
            .find(|&s| s == &window);

        if let Some(window) = window {
            window.set_mapped(false);
            window.set_activate(false);
        }
    }

    fn destroyed_window(&mut self, xwm: XwmId, window: X11Surface) {
        <Self as XwmHandler>::unmapped_window(self, xwm, window);
    }

    fn configure_request(
        &mut self,
        _xwm: XwmId,
        window: X11Surface,
        x: Option<i32>,
        y: Option<i32>,
        w: Option<u32>,
        h: Option<u32>,
        _reorder: Option<Reorder>,
    ) {
        let mut geometry = window.geometry();
        geometry.loc = (x.unwrap_or(geometry.loc.x), y.unwrap_or(geometry.loc.y)).into();
        geometry.size = (
            w.map(|v| v.try_into().ok())
                .flatten()
                .unwrap_or(geometry.size.w),
            h.map(|v| v.try_into().ok())
                .flatten()
                .unwrap_or(geometry.size.h),
        )
            .into();

        if let Err(err) = window.configure(geometry) {
            error!(?err, "Unable to configure window");
        }

        // TODO: Restack
    }

    fn configure_notify(
        &mut self,
        _xwm: XwmId,
        _window: X11Surface,
        _geometry: Rectangle<i32, Logical>,
        _above: Option<smithay::reexports::x11rb::protocol::xproto::Window>,
    ) {
        debug!("configure_notify");
    }

    fn resize_request(
        &mut self,
        _xwm: XwmId,
        surface: X11Surface,
        _button: u32,
        resize_edge: ResizeEdge,
    ) {
        let seat = self.seat.clone();
        resize_grab::handle_resize_request(
            self,
            Window::new_x11_window(surface),
            seat,
            SERIAL_COUNTER.next_serial(),
            resize_edge.into(),
        );
    }

    fn move_request(&mut self, _xwm: XwmId, surface: X11Surface, _button: u32) {
        let seat = self.seat.clone();
        move_grab::handle_move_request(
            self,
            Window::new_x11_window(surface),
            seat,
            SERIAL_COUNTER.next_serial(),
        );
    }
}

impl XWaylandShellHandler for ThingState {
    fn xwayland_shell_state(&mut self) -> &mut XWaylandShellState {
        &mut self.xw_shell_state
    }
}

delegate_xwayland_shell!(ThingState);
