use std::{ffi::OsString, sync::Arc, time::Instant};

use smithay::{
    delegate_data_device, delegate_output, delegate_seat,
    desktop::{Space, Window, WindowSurfaceType},
    input::{keyboard::KeyboardHandle, pointer::PointerHandle, Seat, SeatHandler, SeatState},
    reexports::{
        calloop::LoopHandle,
        wayland_server::{
            backend::{ClientData, ClientId, DisconnectReason},
            protocol::wl_surface::WlSurface,
            Display,
        },
    },
    utils::{Logical, Point},
    wayland::{
        compositor::{CompositorClientState, CompositorState},
        output::{OutputHandler, OutputManagerState},
        selection::{
            data_device::{
                ClientDndGrabHandler, DataDeviceHandler, DataDeviceState, ServerDndGrabHandler,
            },
            SelectionHandler,
        },
        shell::xdg::XdgShellState,
        shm::ShmState,
        socket::ListeningSocketSource,
        xwayland_shell::XWaylandShellState,
    },
    xwayland::X11Wm,
};
use tracing::info;

use crate::CalloopData;

mod compositor;
mod elements;
mod input;
mod xdg_shell;
mod xwayland;

pub struct ThingState {
    pub loop_handle: LoopHandle<'static, CalloopData>,
    pub start_time: Instant,
    pub socket_name: OsString,
    pub space: Space<Window>,

    // Smithay
    pub compositor_state: CompositorState,
    pub xdg_shell_state: XdgShellState,
    pub shm_state: ShmState,
    pub output_manager_state: OutputManagerState,
    pub data_device_state: DataDeviceState,
    pub seat_state: SeatState<ThingState>,
    pub seat: Seat<ThingState>,
    // temporary, there is probably a better way to do this
    pub keyboard_handle: KeyboardHandle<ThingState>,
    pub pointer_handle: PointerHandle<ThingState>,

    // XWayland
    // pub xwayland: Option<XWayland>,
    pub xwm: Option<X11Wm>,
    pub xw_shell_state: XWaylandShellState,
}

impl ThingState {
    pub fn new(
        loop_handle: LoopHandle<'static, CalloopData>,
        display: &mut Display<ThingState>,
    ) -> Self {
        let start_time = Instant::now();

        let dh = &display.handle();

        let space = Space::default();
        let compositor_state = CompositorState::new::<ThingState>(dh);
        let xdg_shell_state = XdgShellState::new::<ThingState>(dh);
        let shm_state = ShmState::new::<ThingState>(dh, vec![]);
        let output_manager_state = OutputManagerState::new_with_xdg_output::<Self>(dh);
        let data_device_state = DataDeviceState::new::<Self>(dh);

        let mut seat_state = SeatState::new();
        let mut seat = seat_state.new_wl_seat(dh, "winit");
        let keyboard_handle = seat
            .add_keyboard(Default::default(), 200, 200)
            .expect("Unable to initialize default keyboard");
        let pointer_handle = seat.add_pointer();

        // Creating wayland socket
        let listening_socket = ListeningSocketSource::new_auto().unwrap();
        let socket_name = listening_socket.socket_name().to_os_string();

        // Insert new client when it connects to the socket
        loop_handle
            .insert_source(listening_socket, |stream, _, data| {
                let res = data
                    .display
                    .handle()
                    .insert_client(stream, Arc::new(ClientState::default()));

                if let Err(e) = res {
                    tracing::error!(err = ?e, "Error inserting new client from wayland socket.");
                }
            })
            .expect("Can't create event source for wayland socket");
        // loop_handle
        //     .insert_source(
        //         Generic::new(
        //             display.backend().poll_fd().as_raw_fd(),
        //             Interest::READ,
        //             Mode::Level,
        //         ),
        //         |_, _, data| {
        //             data.display.dispatch_clients(&mut data.state).unwrap();
        //             Ok(PostAction::Continue)
        //         },
        //     )
        //     .unwrap();

        // XWayland
        // let xwayland = xwayland::setup(&dh, &loop_handle);
        xwayland::setup(&dh, loop_handle.clone());
        let xw_shell_state = XWaylandShellState::new::<ThingState>(&dh);

        ThingState {
            loop_handle,
            start_time,
            space,
            socket_name,

            compositor_state,
            xdg_shell_state,
            shm_state,
            output_manager_state,
            data_device_state,
            seat_state,
            seat,
            keyboard_handle,
            pointer_handle,

            // xwayland,
            xwm: None,
            xw_shell_state,
        }
    }

    /// Finds the element's surface under the given location and return it's surface and location
    /// in the global space
    pub fn surface_under(
        &self,
        location: Point<f64, Logical>,
    ) -> Option<(WlSurface, Point<f64, Logical>)> {
        self.space
            .element_under(location)
            .map(|(window, window_pos)| {
                window
                    .surface_under(location - window_pos.to_f64(), WindowSurfaceType::ALL)
                    .map(|(s, surface_pos)| (s, window_pos.to_f64() + surface_pos.to_f64()))
            })
            .flatten()
    }
}

#[derive(Default)]
pub struct ClientState {
    pub compositor_state: CompositorClientState,
}

impl ClientData for ClientState {
    fn initialized(&self, client_id: ClientId) {
        info!("new client connected with id {:?}", client_id);
    }
    fn disconnected(&self, client_id: ClientId, reason: DisconnectReason) {
        info!(?reason, "Client with id {:?} disconnected.", client_id);
    }
}

impl SelectionHandler for ThingState {
    type SelectionUserData = ();
}

impl SeatHandler for ThingState {
    type KeyboardFocus = WlSurface;

    type PointerFocus = WlSurface;

    type TouchFocus = WlSurface;

    fn seat_state(&mut self) -> &mut SeatState<Self> {
        &mut self.seat_state
    }
}

delegate_seat!(ThingState);

delegate_output!(ThingState);

impl OutputHandler for ThingState {
    fn output_bound(
        &mut self,
        _output: smithay::output::Output,
        _wl_output: smithay::reexports::wayland_server::protocol::wl_output::WlOutput,
    ) {
    }
}

impl DataDeviceHandler for ThingState {
    fn data_device_state(&self) -> &DataDeviceState {
        &self.data_device_state
    }
}
impl ClientDndGrabHandler for ThingState {}
impl ServerDndGrabHandler for ThingState {}

delegate_data_device!(ThingState);
