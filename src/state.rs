use std::{ffi::OsString, os::fd::AsRawFd, sync::Arc, time::Instant};

use smithay::{
    delegate_output,
    desktop::{Space, Window},
    input::{Seat, SeatState, SeatHandler},
    reexports::{
        calloop::{generic::Generic, EventLoop, Interest, Mode, PostAction},
        wayland_server::{
            backend::{ClientData, ClientId, DisconnectReason},
            Display, protocol::wl_surface::WlSurface,
        },
    },
    wayland::{compositor::{CompositorClientState, CompositorState}, socket::ListeningSocketSource, shm::ShmState, shell::xdg::XdgShellState, output::OutputManagerState, data_device::{DataDeviceState, DataDeviceHandler, ClientDndGrabHandler, ServerDndGrabHandler}}, delegate_seat, delegate_data_device,
};
use tracing::info;

use crate::CalloopData;

mod input;
mod compositor;
mod xdg_shell;

pub struct ThingState {
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
}

impl ThingState {
    pub fn new(event_loop: &mut EventLoop<CalloopData>, display: &mut Display<ThingState>) -> Self {
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
        seat.add_keyboard(Default::default(), 200, 200).unwrap();
        seat.add_pointer();

        // Creating wayland socket
        let listening_socket = ListeningSocketSource::new_auto().unwrap();
        let socket_name = listening_socket.socket_name().to_os_string();

        // Insert new client when it connects to the socket
        event_loop
            .handle()
            .insert_source(listening_socket, |stream, _, data| {
                data.display
                    .handle()
                    .insert_client(stream, Arc::new(ClientState::default()))
                    .unwrap();
            })
            .unwrap();
        event_loop
            .handle()
            .insert_source(
                Generic::new(
                    display.backend().poll_fd().as_raw_fd(),
                    Interest::READ,
                    Mode::Level,
                ),
                |_, _, data| {
                    data.display.dispatch_clients(&mut data.state).unwrap();
                    Ok(PostAction::Continue)
                },
            )
            .unwrap();

        ThingState {
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
        }
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

impl SeatHandler for ThingState {
    type KeyboardFocus = WlSurface;

    type PointerFocus = WlSurface;

    fn seat_state(&mut self) -> &mut SeatState<Self> {
        &mut self.seat_state
    }
}

delegate_seat!(ThingState);

delegate_output!(ThingState);

impl DataDeviceHandler for ThingState {
    fn data_device_state(&self) -> &DataDeviceState {
        &self.data_device_state
    }
}
impl ClientDndGrabHandler for ThingState {}
impl ServerDndGrabHandler for ThingState {}

delegate_data_device!(ThingState);
