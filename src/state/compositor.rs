use smithay::{wayland::{compositor::{CompositorHandler, CompositorState, CompositorClientState, is_sync_subsurface, get_parent}, buffer::BufferHandler, shm::{ShmHandler, ShmState}}, reexports::wayland_server::{Client, protocol::{wl_surface::WlSurface, wl_buffer::WlBuffer}}, backend::renderer::utils::on_commit_buffer_handler, delegate_compositor, delegate_shm};

use super::{ThingState, ClientState, xdg_shell::{self, resize_grab}};

// COMPOSITOR

impl CompositorHandler for ThingState {
    fn compositor_state(&mut self) -> &mut CompositorState {
        &mut self.compositor_state
    }

    fn client_compositor_state<'a>(&self, client: &'a Client) -> &'a CompositorClientState {
        &client.get_data::<ClientState>().unwrap().compositor_state
    }

    fn commit(&mut self, surface: &WlSurface) {
        on_commit_buffer_handler::<Self>(surface);
        // No idea what this is supposed to do for now
        if !is_sync_subsurface(surface) {
            let mut root = surface.clone();
            while let Some(parent) = get_parent(&root) {
                root = parent;
            }
            if let Some(window) = self.space.elements().find(|w| w.toplevel().wl_surface() == &root) {
                window.on_commit();
            }
        };

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
