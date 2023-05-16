use smithay::{
    delegate_xdg_shell,
    desktop::{Space, Window},
    reexports::wayland_server::protocol::{wl_seat::WlSeat, wl_surface::WlSurface},
    utils::Serial,
    wayland::{
        compositor::with_states,
        shell::xdg::{
            PopupSurface, PositionerState, ToplevelSurface, XdgShellHandler, XdgShellState,
            XdgToplevelSurfaceData,
        },
    },
};

use super::ThingState;

impl XdgShellHandler for ThingState {
    fn xdg_shell_state(&mut self) -> &mut XdgShellState {
        &mut self.xdg_shell_state
    }

    fn new_toplevel(&mut self, surface: ToplevelSurface) {
        let window = Window::new(surface);
        self.space.map_element(window, (0, 0), true);
    }

    fn new_popup(&mut self, _surface: PopupSurface, _positioner: PositionerState) {
        //TODO: Popup handling using PopupManager (see Smallvil)
    }

    fn grab(&mut self, _surface: PopupSurface, _seat: WlSeat, _serial: Serial) {
        //TODO: Popup grabs (see Smallvil)
    }

    // TODO: implement `move_request` ans `resize_request`
    //       Still need to understand the logic here tho
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
