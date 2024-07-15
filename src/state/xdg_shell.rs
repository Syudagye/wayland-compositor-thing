use super::ThingState;
use smithay::{
    delegate_xdg_shell,
    desktop::{
        find_popup_root_surface, get_popup_toplevel_coords, PopupKeyboardGrab, PopupKind,
        PopupManager, PopupPointerGrab, PopupUngrabStrategy, Space, Window,
    },
    input::{
        keyboard::KeyboardGrab,
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
        seat::WaylandFocus,
        shell::xdg::{
            Configure, PopupSurface, PositionerState, ToplevelSurface, XdgShellHandler, XdgShellState, XdgToplevelSurfaceData
        },
    },
};
use tracing::{error, trace};

pub mod move_grab;
pub mod resize_grab;

impl ThingState {
    /// Adjust popup position for it to fit in the visible area of the compositor
    fn uncontrain_popup(&self, popup: PopupSurface) {
        let Some(root) = find_popup_root_surface(&PopupKind::Xdg(popup.clone())).ok() else {
            return;
        };
        let Some(window) = self.window_for_surface(root) else {
            return;
        };

        let outputs = self.space.outputs_for_element(window);
        if outputs.is_empty() {
            return;
        }

        let output_geo = outputs
            .iter()
            .filter_map(|o| self.space.output_geometry(o))
            .fold(Rectangle::default(), |acc, a| acc.merge(a));
        // let Some(window_geo) = self.space.element_geometry(window) else {
        //     return;
        // };

        // get_popup_toplevel_coords(&PopupKind::Xdg(popup));

        popup.with_pending_state(|state| {
            state.geometry = state.positioner.get_unconstrained_geometry(output_geo);
        });
    }
}

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

        self.uncontrain_popup(surface.clone());

        let kind = PopupKind::Xdg(surface);
        let res = self.popup_manager.track_popup(kind);
        if let Err(err) = res {
            error!(?err, "Unable to track popup");
        }
    }

    fn grab(&mut self, surface: PopupSurface, seat: WlSeat, serial: Serial) {
        trace!(?surface, "new popup grab");
        let Some(seat) = Seat::<ThingState>::from_resource(&seat) else {
            error!("Cannot initialise seat for popup grab");
            return;
        };
        let kind = PopupKind::Xdg(surface);

        let Some(root) = find_popup_root_surface(&kind).ok().and_then(|surface| {
            self.space
                .elements()
                .filter_map(|e| e.wl_surface().map(|s| s.into_owned()))
                .find(|s| s == &surface)
        }) else {
            return;
        };

        let res = self.popup_manager.grab_popup(root, kind, &seat, serial);

        if let Ok(mut grab) = res {
            if let Some(kb) = seat.get_keyboard() {
                trace!("grabing keyboard");
                if kb.is_grabbed()
                    && !(kb.has_grab(serial)
                        || kb.has_grab(grab.previous_serial().unwrap_or(serial)))
                {
                    grab.ungrab(PopupUngrabStrategy::All);
                    return;
                }
                kb.set_focus(self, grab.current_grab(), serial);
                kb.set_grab(self, PopupKeyboardGrab::new(&grab), serial);
            }
            if let Some(ptr) = seat.get_pointer() {
                trace!("pointer grab");
                if ptr.is_grabbed()
                    && !(ptr.has_grab(serial)
                        || ptr.has_grab(grab.previous_serial().unwrap_or(serial)))
                {
                    grab.ungrab(PopupUngrabStrategy::All);
                    return;
                }
                ptr.set_grab(self, PopupPointerGrab::new(&grab), serial, Focus::Clear);
            }
        }
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
        surface.send_repositioned(token);
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
