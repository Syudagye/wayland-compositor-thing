use smithay::{
    utils::{Logical, Rectangle, SERIAL_COUNTER},
    xwayland::{
        xwm::{Reorder, ResizeEdge, XwmId},
        X11Surface, X11Wm, XwmHandler,
    },
};
use tracing::debug;

use crate::CalloopData;

use super::{
    elements::WindowElement,
    xdg_shell::{move_grab, resize_grab},
};

impl XwmHandler for CalloopData {
    fn xwm_state(&mut self, _xwm: XwmId) -> &mut X11Wm {
        // We can unwrap here because if there can't be a None.
        // If the X11Wm is not present we would have stopped the compositor already
        self.state.xwm.as_mut().unwrap()
    }

    fn new_window(&mut self, _xwm: XwmId, _window: X11Surface) {
        // self.state.space.map_element(WindowElement::X11(window), (0, 0), false);
        debug!("New XWayland Window");
    }

    fn new_override_redirect_window(&mut self, _xwm: XwmId, _window: X11Surface) {
        // self.state.space.map_element(WindowElement::X11(window), (0, 0), false);
        debug!("New XWayland Window (Override Redirect)");
    }

    fn map_window_request(&mut self, _xwm: XwmId, window: X11Surface) {
        debug!("New XWayland Window map request");
        window.set_mapped(true).unwrap();
        self.state
            .space
            .map_element(WindowElement::X11(window), (0, 0), true);
    }

    fn mapped_override_redirect_window(&mut self, _xwm: XwmId, window: X11Surface) {
        debug!("New XWayland Window map request (OR)");
        self.state
            .space
            .map_element(WindowElement::X11(window), (0, 0), true);
    }

    fn unmapped_window(&mut self, _xwm: XwmId, window: X11Surface) {
        self.state.space.unmap_elem(&WindowElement::X11(window));
    }

    fn destroyed_window(&mut self, _xwm: XwmId, window: X11Surface) {
        self.state.space.unmap_elem(&WindowElement::X11(window));
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

        window.configure(geometry).unwrap();
        // TODO: Restack
    }

    fn configure_notify(
        &mut self,
        _xwm: XwmId,
        window: X11Surface,
        geometry: Rectangle<i32, Logical>,
        _above: Option<u32>,
    ) {
        // window.configure(geometry).unwrap();
    }

    fn resize_request(
        &mut self,
        _xwm: XwmId,
        window: X11Surface,
        _button: u32,
        resize_edge: ResizeEdge,
    ) {
        let seat = self.state.seat.clone();
        resize_grab::handle_resize_request(
            &mut self.state,
            WindowElement::X11(window),
            seat,
            SERIAL_COUNTER.next_serial(),
            resize_edge.into(),
        );
    }

    fn move_request(&mut self, xwm: XwmId, window: X11Surface, button: u32) {
        let seat = self.state.seat.clone();
        move_grab::handle_move_request(
            &mut self.state,
            WindowElement::X11(window),
            seat,
            SERIAL_COUNTER.next_serial(),
        );
    }
}
