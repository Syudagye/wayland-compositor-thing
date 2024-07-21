use smithay::reexports::wayland_server::DisplayHandle;

use crate::state::ThingState;

pub mod winit;

pub struct CalloopData {
    pub state: ThingState,
    pub dh: DisplayHandle,
}
