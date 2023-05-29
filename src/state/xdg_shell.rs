use smithay::{
    delegate_xdg_shell,
    desktop::{Space, Window},
    input::{
        pointer::{Focus, GrabStartData},
        Seat,
    },
    reexports::{
        wayland_protocols::xdg::shell::server::xdg_toplevel::ResizeEdge,
        wayland_server::{
            protocol::{wl_seat::WlSeat, wl_surface::WlSurface},
            Resource,
        },
    },
    utils::{Rectangle, Serial},
    wayland::{
        compositor::with_states,
        shell::xdg::{
            PopupSurface, PositionerState, ToplevelSurface, XdgShellHandler, XdgShellState,
            XdgToplevelSurfaceData,
        },
    },
};
use tracing::debug;

use self::{move_grab::MovePointerGrab, resize_grab::ResizePointerGrab};

use super::{elements::WindowElement, ThingState};

pub mod move_grab;
pub mod resize_grab;

impl XdgShellHandler for ThingState {
    fn xdg_shell_state(&mut self) -> &mut XdgShellState {
        &mut self.xdg_shell_state
    }

    fn new_toplevel(&mut self, surface: ToplevelSurface) {
        let window = Window::new(surface);
        self.space
            .map_element(WindowElement::Wayland(window), (0, 0), false);
    }

    fn new_popup(&mut self, _surface: PopupSurface, _positioner: PositionerState) {
        //TODO: Popup handling using PopupManager (see Smallvil)
    }

    fn grab(&mut self, _surface: PopupSurface, _seat: WlSeat, _serial: Serial) {
        //TODO: Popup grabs (see Smallvil)
    }

    fn move_request(&mut self, surface: ToplevelSurface, seat: WlSeat, serial: Serial) {
        let seat: Seat<ThingState> = Seat::from_resource(&seat).unwrap();

        let window = self
            .space
            .elements()
            .find(|w| match w {
                WindowElement::Wayland(s) => s.toplevel().wl_surface() == surface.wl_surface(),
                WindowElement::X11(s) => s.wl_surface() == Some(surface.wl_surface().clone()),
            })
            .unwrap()
            .clone();

        move_grab::handle_move_request(self, window, seat, serial);
    }

    fn resize_request(
        &mut self,
        surface: ToplevelSurface,
        seat: WlSeat,
        serial: Serial,
        edges: ResizeEdge,
    ) {
        let seat: Seat<ThingState> = Seat::from_resource(&seat).unwrap();

        let window = self
            .space
            .elements()
            .find(|w| match w {
                WindowElement::Wayland(s) => s.toplevel().wl_surface() == surface.wl_surface(),
                WindowElement::X11(s) => s.wl_surface() == Some(surface.wl_surface().clone()),
            })
            .unwrap()
            .clone();

        resize_grab::handle_resize_request(self, window, seat, serial, edges.into());
    }
}

delegate_xdg_shell!(ThingState);

/// Verify if the given surface has the cursor grab
fn check_grab(
    seat: &Seat<ThingState>,
    surface: &WlSurface,
    serial: Serial,
) -> Option<GrabStartData<ThingState>> {
    let pointer = seat.get_pointer()?;

    // Check that this surface has a click grab.
    if !pointer.has_grab(serial) {
        return None;
    }

    let start_data = pointer.grab_start_data()?;

    let (focus, _) = start_data.focus.as_ref()?;
    // If the focus was for a different surface, ignore the request.
    if !focus.id().same_client_as(&surface.id()) {
        return None;
    }

    Some(start_data)
}

/// Sends the configure event to the given surface if it haven't been sent
/// Should be called on `WlSurface::commit`
pub fn handle_commit(space: &Space<WindowElement>, surface: &WlSurface) -> Option<()> {
    let window = space
        .elements()
        .find(|w| match w {
            WindowElement::Wayland(s) => s.toplevel().wl_surface() == surface,
            WindowElement::X11(s) => s.wl_surface() == Some(surface.clone()),
        })
        .cloned()?;

    let initial_configure_sent = with_states(surface, |states| {
        if let Some(data) = states.data_map.get::<XdgToplevelSurfaceData>() {
            return Some(data.lock().unwrap().initial_configure_sent);
        }
        None
    });

    if let WindowElement::Wayland(window) = window {
        if let Some(initial_configure_sent) = initial_configure_sent {
            if !initial_configure_sent {
                window.toplevel().send_configure();
            }
        }
    }

    Some(())
}
