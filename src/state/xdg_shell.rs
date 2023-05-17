use smithay::{
    delegate_xdg_shell,
    desktop::{Space, Window},
    input::{
        pointer::{
            Focus, GrabStartData,
        },
        Seat,
    },
    reexports::wayland_server::{
        protocol::{wl_seat::WlSeat, wl_surface::WlSurface},
        Resource,
    },
    utils::{Serial},
    wayland::{
        compositor::with_states,
        shell::xdg::{
            PopupSurface, PositionerState, ToplevelSurface, XdgShellHandler, XdgShellState,
            XdgToplevelSurfaceData,
        },
    },
};

use self::grabs::MovePointerGrab;

use super::ThingState;

pub mod grabs;

impl XdgShellHandler for ThingState {
    fn xdg_shell_state(&mut self) -> &mut XdgShellState {
        &mut self.xdg_shell_state
    }

    fn new_toplevel(&mut self, surface: ToplevelSurface) {
        let window = Window::new(surface);
        self.space.map_element(window, (0, 0), false);
    }

    fn new_popup(&mut self, _surface: PopupSurface, _positioner: PositionerState) {
        //TODO: Popup handling using PopupManager (see Smallvil)
    }

    fn grab(&mut self, _surface: PopupSurface, _seat: WlSeat, _serial: Serial) {
        //TODO: Popup grabs (see Smallvil)
    }

    fn move_request(&mut self, surface: ToplevelSurface, seat: WlSeat, serial: Serial) {
        let seat: Seat<ThingState> = Seat::from_resource(&seat).unwrap();

        let Some(start_data) = check_grab(&seat, surface.wl_surface(), serial) else {
            return;
        };

        let pointer = seat.get_pointer().unwrap();

        let window = self
            .space
            .elements()
            .find(|window| window.toplevel().wl_surface() == surface.wl_surface())
            .unwrap()
            .clone();
        let initial_window_location = self.space.element_location(&window).unwrap();

        let grab = MovePointerGrab {
            start_data,
            window,
            initial_window_location,
        };

        pointer.set_grab(self, grab, serial, Focus::Clear);
    }

    // TODO: implement `resize_request`
    //       Still need to understand the logic here tho
}

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

delegate_xdg_shell!(ThingState);

/// Sends the configure event to the given surface if it haven't been sent
/// Should be called on `WlSurface::commit`
pub fn handle_commit(space: &Space<Window>, surface: &WlSurface) -> Option<()> {
    let window = space
        .elements()
        .find(|w| w.toplevel().wl_surface() == surface)
        .cloned()?;

    let initial_configure_sent = with_states(surface, |states| {
        states
            .data_map
            .get::<XdgToplevelSurfaceData>()
            .unwrap()
            .lock()
            .unwrap()
            .initial_configure_sent
    });

    if !initial_configure_sent {
        window.toplevel().send_configure();
    }

    Some(())
}
