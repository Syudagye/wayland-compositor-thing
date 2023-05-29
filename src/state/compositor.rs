use smithay::{
    backend::renderer::utils::on_commit_buffer_handler,
    delegate_compositor, delegate_shm,
    reexports::wayland_server::{
        protocol::{wl_buffer::WlBuffer, wl_surface::WlSurface},
        Client, backend::ClientData,
    },
    wayland::{
        buffer::BufferHandler,
        compositor::{
            get_parent, is_sync_subsurface, CompositorClientState, CompositorHandler,
            CompositorState,
        },
        shm::{ShmHandler, ShmState},
    }, xwayland::{X11Wm, XWaylandClientData},
};

use crate::CalloopData;

use super::{
    elements::WindowElement,
    xdg_shell::{self, resize_grab},
    ClientState, ThingState,
};

// COMPOSITOR

impl CompositorHandler for ThingState {
    fn compositor_state(&mut self) -> &mut CompositorState {
        &mut self.compositor_state
    }

    fn client_compositor_state<'a>(&self, client: &'a Client) -> &'a CompositorClientState {
        if let Some(client_data) = client.get_data::<XWaylandClientData>() {
            return &client_data.compositor_state;
        }
        if let Some(client_data) = client.get_data::<ClientState>() {
            return &client_data.compositor_state;
        }
        panic!("Can't get the client's compositor state");
    }

    fn commit(&mut self, surface: &WlSurface) {
        on_commit_buffer_handler::<Self>(surface);
        // No idea what this is supposed to do for now
        if !is_sync_subsurface(surface) {
            let mut root = surface.clone();
            while let Some(parent) = get_parent(&root) {
                root = parent;
            }
            if let Some(window) = self
                .space
                .elements()
                .filter_map(|w| {
                    if let WindowElement::Wayland(w) = w {
                        Some(w)
                    } else {
                        None
                    }
                })
                .find(|w| w.toplevel().wl_surface() == &root)
            {
                window.on_commit();
            }
        };

        // Idk where to put this tho
        X11Wm::commit_hook::<CalloopData>(surface);

        xdg_shell::handle_commit(&self.space, surface);
        resize_grab::handle_commit(&mut self.space, surface);
    }
}

delegate_compositor!(ThingState);

// SHM

impl BufferHandler for ThingState {
    fn buffer_destroyed(&mut self, _buffer: &WlBuffer) {}
}

impl ShmHandler for ThingState {
    fn shm_state(&self) -> &ShmState {
        &self.shm_state
    }
}

delegate_shm!(ThingState);
