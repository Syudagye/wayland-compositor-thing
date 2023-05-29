use std::{ffi::OsString, os::fd::AsRawFd, sync::Arc, time::Instant};

use smithay::{
    delegate_data_device, delegate_output, delegate_seat,
    desktop::{Space, WindowSurfaceType},
    input::{Seat, SeatHandler, SeatState},
    reexports::{
        calloop::{generic::Generic, Interest, LoopHandle, Mode, PostAction},
        wayland_server::{
            backend::{ClientData, ClientId, DisconnectReason},
            protocol::wl_surface::WlSurface,
            Display,
        },
    },
    utils::{Logical, Point},
    wayland::{
        compositor::{CompositorClientState, CompositorState},
        data_device::{
            ClientDndGrabHandler, DataDeviceHandler, DataDeviceState, ServerDndGrabHandler,
        },
        output::OutputManagerState,
        shell::xdg::XdgShellState,
        shm::ShmState,
        socket::ListeningSocketSource,
    },
    xwayland::{X11Wm, XWayland, XWaylandEvent},
};
use tracing::info;

use crate::CalloopData;

use self::elements::WindowElement;

mod compositor;
mod elements;
mod input;
mod xdg_shell;
mod xwayland;

pub struct ThingState {
    pub loop_handle: LoopHandle<'static, CalloopData>,
    pub start_time: Instant,
    pub socket_name: OsString,
    pub space: Space<WindowElement>,

    // Smithay
    pub compositor_state: CompositorState,
    pub xdg_shell_state: XdgShellState,
    pub shm_state: ShmState,
    pub output_manager_state: OutputManagerState,
    pub data_device_state: DataDeviceState,
    pub seat_state: SeatState<ThingState>,
    pub seat: Seat<ThingState>,

    // XWayland
    pub xwayland: XWayland,
    pub xwm: Option<X11Wm>,
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
        seat.add_keyboard(Default::default(), 200, 200).unwrap();
        seat.add_pointer();

        // Creating wayland socket
        let listening_socket = ListeningSocketSource::new_auto().unwrap();
        let socket_name = listening_socket.socket_name().to_os_string();

        // Insert new client when it connects to the socket
        loop_handle
            .insert_source(listening_socket, |stream, _, data| {
                data.display
                    .handle()
                    .insert_client(stream, Arc::new(ClientState::default()))
                    .unwrap();
            })
            .unwrap();
        loop_handle
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

        // XWayland
        let (xwayland, xw_source) = XWayland::new(&dh);

        loop_handle
            .insert_source(xw_source, |event, _, data| {
                match event {
                    XWaylandEvent::Ready {
                        connection,
                        client,
                        client_fd: _,
                        display,
                    } => {
                        std::env::set_var("DISPLAY", format!(":{}", display.to_string()));
                        info!("XWayland server ready");
                        // data.display.handle().insert_client(connection, Arc::new(ClientState::default())).unwrap();
                        let wm = X11Wm::start_wm(
                            data.state.loop_handle.clone(),
                            data.display.handle(),
                            connection,
                            client,
                        )
                        .unwrap();

                        data.state.xwm = Some(wm);
                    }
                    XWaylandEvent::Exited => info!("XWayland client exited"),
                }
            })
            .unwrap();

        xwayland
            .start::<_, String, String, _, _>(loop_handle.clone(), 1, [], false, |_| ())
            .unwrap();

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

            xwayland,
            xwm: None,
        }
    }

    pub fn surface_under(
        &self,
        location: Point<f64, Logical>,
    ) -> Option<(WlSurface, Point<i32, Logical>)> {
        self.space
            .element_under(location)
            .map(|(window, window_pos)| {
                window
                    .surface_under(location - window_pos.to_f64(), WindowSurfaceType::ALL)
                    .map(|(s, surface_pos)| (s, window_pos + surface_pos))
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
