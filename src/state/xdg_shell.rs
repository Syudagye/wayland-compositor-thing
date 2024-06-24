use super::ThingState;
use smithay::{
    delegate_xdg_shell,
    desktop::{Space, Window},
    input::{pointer::GrabStartData, Seat},
    reexports::{
        wayland_protocols::xdg::shell::server::xdg_toplevel::ResizeEdge,
        wayland_server::{
            protocol::{wl_seat::WlSeat, wl_surface::WlSurface},
            Resource,
        },
    },
    utils::Serial,
    wayland::{
        compositor::with_states,
        seat::WaylandFocus,
        shell::xdg::{
            PopupSurface, PositionerState, ToplevelSurface, XdgShellHandler, XdgShellState,
            XdgToplevelSurfaceData,
        },
    },
};
use tracing::trace;

pub mod move_grab;
pub mod resize_grab;

impl XdgShellHandler for ThingState {
    fn xdg_shell_state(&mut self) -> &mut XdgShellState {
        &mut self.xdg_shell_state
    }

    fn new_toplevel(&mut self, surface: ToplevelSurface) {
        trace!(?surface, "new top level");
        let window = Window::new_wayland_window(surface);
        self.space.map_element(window, (0, 0), true);
    }

    fn new_popup(&mut self, surface: PopupSurface, positioner: PositionerState) {
        trace!(?surface, ?positioner, "new popup surface");
        //TODO: Popup handling using PopupManager (see Smallvil)
    }

    fn grab(&mut self, surface: PopupSurface, _seat: WlSeat, _serial: Serial) {
        trace!(?surface, "new popup grab");
        //TODO: Popup grabs (see Smallvil)
    }

    fn move_request(&mut self, surface: ToplevelSurface, seat: WlSeat, serial: Serial) {
        let seat: Seat<ThingState> = Seat::from_resource(&seat).unwrap();

        let Some(window) = self.space.elements().find(|w| {
            w.wl_surface()
                .map(|s| s.as_ref() == surface.wl_surface())
                .unwrap_or(false)
        }) else {
            return;
        };

        move_grab::handle_move_request(self, window.clone(), seat, serial);
    }

    fn resize_request(
        &mut self,
        surface: ToplevelSurface,
        seat: WlSeat,
        serial: Serial,
        edges: ResizeEdge,
    ) {
        let seat: Seat<ThingState> = Seat::from_resource(&seat).unwrap();

        let Some(window) = self.space.elements().find(|w| {
            w.wl_surface()
                .map(|s| s.as_ref() == surface.wl_surface())
                .unwrap_or(false)
        }) else {
            return;
        };

        resize_grab::handle_resize_request(self, window.clone(), seat, serial, edges.into());
    }

    fn reposition_request(
        &mut self,
        surface: PopupSurface,
        positioner: PositionerState,
        token: u32,
    ) {
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
pub fn handle_commit(space: &Space<Window>, surface: &WlSurface) -> Option<()> {
    let window = space
        .elements()
        .find(|w| {
            w.wl_surface()
                .map(|s| s.as_ref() == surface)
                .unwrap_or(false)
        })
        .cloned()?;

    let initial_configure_sent = with_states(surface, |states| {
        if let Some(data) = states.data_map.get::<XdgToplevelSurfaceData>() {
            return Some(data.lock().ok().map(|l| l.initial_configure_sent));
        }
        None
    })
    .flatten()?;

    if !initial_configure_sent {
        window.toplevel()?.send_configure();
    }

    Some(())
}
