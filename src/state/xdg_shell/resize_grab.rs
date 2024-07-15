use std::cell::RefCell;

use smithay::{
    desktop::{Space, Window},
    input::{
        pointer::{
            ButtonEvent, Focus, GrabStartData, MotionEvent, PointerGrab, PointerInnerHandle,
            RelativeMotionEvent,
        },
        Seat, SeatHandler,
    },
    reexports::{
        wayland_protocols::xdg::shell::server::xdg_toplevel::{self, State},
        wayland_server::protocol::wl_surface::WlSurface,
    },
    utils::{Logical, Point, Rectangle, Serial, Size},
    wayland::{compositor, seat::WaylandFocus, shell::xdg::SurfaceCachedState},
    xwayland::xwm,
};
use tracing::error;

use crate::state::ThingState;

use super::check_grab;

// Copied from smallvil
bitflags::bitflags! {
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct ResizeEdge: u32 {
        const TOP          = 0b0001;
        const BOTTOM       = 0b0010;
        const LEFT         = 0b0100;
        const RIGHT        = 0b1000;

        const TOP_LEFT     = Self::TOP.bits() | Self::LEFT.bits();
        const BOTTOM_LEFT  = Self::BOTTOM.bits() | Self::LEFT.bits();

        const TOP_RIGHT    = Self::TOP.bits() | Self::RIGHT.bits();
        const BOTTOM_RIGHT = Self::BOTTOM.bits() | Self::RIGHT.bits();
    }
}

impl From<xdg_toplevel::ResizeEdge> for ResizeEdge {
    #[inline]
    fn from(x: xdg_toplevel::ResizeEdge) -> Self {
        Self::from_bits(x as u32).unwrap()
    }
}

impl From<xwm::ResizeEdge> for ResizeEdge {
    #[inline]
    fn from(x: xwm::ResizeEdge) -> Self {
        Self::from_bits(x as u32).unwrap()
    }
}

pub struct ResizePointerGrab {
    pub start_data: GrabStartData<ThingState>,
    pub window: Window,
    pub initial_rect: Rectangle<i32, Logical>,

    pub edges: ResizeEdge,

    pub last_window_size: Size<i32, Logical>,
}

impl ResizePointerGrab {
    pub fn start(
        start_data: GrabStartData<ThingState>,
        window: Window,
        initial_rect: Rectangle<i32, Logical>,
        edges: ResizeEdge,
    ) -> Self {
        let last_window_size = initial_rect.size;

        let surface = window.wl_surface().map(|s| s.into_owned());

        // ResizeSurfaceState::with(&surface, |state| {
        //     *state = ResizeSurfaceState::Resizing {
        //         edges,
        //         initial_rect,
        //     };
        // });

        Self {
            start_data,
            window,
            initial_rect,
            edges,
            last_window_size,
        }
    }
}

impl PointerGrab<ThingState> for ResizePointerGrab {
    fn motion(
        &mut self,
        data: &mut ThingState,
        handle: &mut PointerInnerHandle<'_, ThingState>,
        _focus: Option<(
            WlSurface,
            smithay::utils::Point<f64, smithay::utils::Logical>,
        )>,
        event: &MotionEvent,
    ) {
        handle.motion(data, None, event);

        let mut delta = event.location - self.start_data.location;

        if self.edges.intersects(ResizeEdge::TOP | ResizeEdge::BOTTOM) {
            delta.x = 0.0;
            if self.edges.intersects(ResizeEdge::TOP) {
                delta.y = -delta.y;
            }
        }

        if self.edges.intersects(ResizeEdge::LEFT | ResizeEdge::RIGHT) {
            delta.y = 0.0;
            if self.edges.intersects(ResizeEdge::LEFT) {
                delta.x = -delta.x;
            }
        }

        let (min_size, max_size) = {
            if let Some(surface) = self.window.wl_surface().map(|s| s.into_owned()) {
                compositor::with_states(&surface, |states| {
                    let mut guard = states.cached_state.get::<SurfaceCachedState>();
                    let data = guard.current();
                    (data.min_size, data.max_size)
                })
            } else {
                error!("Can't get surface for resize grab");
                return;
            }
        };

        let min_width = min_size.w.max(1);
        let min_height = min_size.h.max(1);

        let max_width = (max_size.w == 0).then(i32::max_value).unwrap_or(max_size.w);
        let max_height = (max_size.h == 0).then(i32::max_value).unwrap_or(max_size.h);

        self.last_window_size = Size::from((
            (self.initial_rect.size.w + delta.x as i32)
                .min(max_width)
                .max(min_width),
            (self.initial_rect.size.h + delta.y as i32)
                .min(max_height)
                .max(min_height),
        ));

        if let Some(toplevel) = self.window.toplevel() {
            toplevel.with_pending_state(|state| {
                state.states.set(State::Resizing);
                state.size = Some(self.last_window_size);
            });
            toplevel.send_pending_configure();
        }
    }

    fn relative_motion(
        &mut self,
        data: &mut ThingState,
        handle: &mut PointerInnerHandle<'_, ThingState>,
        _focus: Option<(
            WlSurface,
            smithay::utils::Point<f64, smithay::utils::Logical>,
        )>,
        event: &RelativeMotionEvent,
    ) {
        handle.relative_motion(data, None, event);
    }

    fn button(
        &mut self,
        data: &mut ThingState,
        handle: &mut PointerInnerHandle<'_, ThingState>,
        event: &ButtonEvent,
    ) {
        handle.button(data, event);

        // // The button is a button code as defined in the
        // // Linux kernel's linux/input-event-codes.h header file, e.g. BTN_LEFT.
        // const BTN_LEFT: u32 = 0x110;
        //
        // if !handle.current_pressed().contains(&BTN_LEFT) {
        //     // No more buttons are pressed, release the grab.
        //     handle.unset_grab(data, event.serial, event.time);
        //
        //     match &self.window {
        //         WindowElement::Wayland(surface) => {
        //             let xdg = surface.toplevel();
        //
        //             ResizeSurfaceState::with(xdg.wl_surface(), |state| {
        //                 *state = ResizeSurfaceState::WaitingForLastCommit {
        //                     edges: self.edges,
        //                     initial_rect: self.initial_rect,
        //                 };
        //             });
        //
        //             xdg.with_pending_state(|state| {
        //                 state.states.unset(State::Resizing);
        //                 state.size = Some(self.last_window_size);
        //             });
        //             xdg.send_pending_configure();
        //         }
        //         WindowElement::X11(surface) => {
        //             ResizeSurfaceState::with(&surface.wl_surface().unwrap(), |state| {
        //                 *state = ResizeSurfaceState::WaitingForLastCommit {
        //                     edges: self.edges,
        //                     initial_rect: self.initial_rect,
        //                 };
        //             });
        //
        //             let location = data
        //                 .space
        //                 .element_location(&WindowElement::X11(surface.clone()))
        //                 .unwrap();
        //             surface
        //                 .configure(Some(Rectangle::from_loc_and_size(
        //                     location,
        //                     self.last_window_size,
        //                 )))
        //                 .unwrap();
        //         }
        //     }
        // }
    }

    fn axis(
        &mut self,
        data: &mut ThingState,
        handle: &mut PointerInnerHandle<'_, ThingState>,
        details: smithay::input::pointer::AxisFrame,
    ) {
        handle.axis(data, details);
    }

    fn start_data(&self) -> &GrabStartData<ThingState> {
        &self.start_data
    }

    fn frame(&mut self, data: &mut ThingState, handle: &mut PointerInnerHandle<'_, ThingState>) {
        handle.frame(data);
    }

    fn gesture_swipe_begin(
        &mut self,
        data: &mut ThingState,
        handle: &mut PointerInnerHandle<'_, ThingState>,
        event: &smithay::input::pointer::GestureSwipeBeginEvent,
    ) {
        handle.gesture_swipe_begin(data, event);
    }

    fn gesture_swipe_update(
        &mut self,
        data: &mut ThingState,
        handle: &mut PointerInnerHandle<'_, ThingState>,
        event: &smithay::input::pointer::GestureSwipeUpdateEvent,
    ) {
        handle.gesture_swipe_update(data, event);
    }

    fn gesture_swipe_end(
        &mut self,
        data: &mut ThingState,
        handle: &mut PointerInnerHandle<'_, ThingState>,
        event: &smithay::input::pointer::GestureSwipeEndEvent,
    ) {
        handle.gesture_swipe_end(data, event);
    }

    fn gesture_pinch_begin(
        &mut self,
        data: &mut ThingState,
        handle: &mut PointerInnerHandle<'_, ThingState>,
        event: &smithay::input::pointer::GesturePinchBeginEvent,
    ) {
        handle.gesture_pinch_begin(data, event);
    }

    fn gesture_pinch_update(
        &mut self,
        data: &mut ThingState,
        handle: &mut PointerInnerHandle<'_, ThingState>,
        event: &smithay::input::pointer::GesturePinchUpdateEvent,
    ) {
        handle.gesture_pinch_update(data, event);
    }

    fn gesture_pinch_end(
        &mut self,
        data: &mut ThingState,
        handle: &mut PointerInnerHandle<'_, ThingState>,
        event: &smithay::input::pointer::GesturePinchEndEvent,
    ) {
        handle.gesture_pinch_end(data, event);
    }

    fn gesture_hold_begin(
        &mut self,
        data: &mut ThingState,
        handle: &mut PointerInnerHandle<'_, ThingState>,
        event: &smithay::input::pointer::GestureHoldBeginEvent,
    ) {
        handle.gesture_hold_begin(data, event);
    }

    fn gesture_hold_end(
        &mut self,
        data: &mut ThingState,
        handle: &mut PointerInnerHandle<'_, ThingState>,
        event: &smithay::input::pointer::GestureHoldEndEvent,
    ) {
        handle.gesture_hold_end(data, event);
    }

    fn unset(&mut self, data: &mut ThingState) {
    }
}

/// State of the resize operation.
///
/// It is stored inside of WlSurface,
/// and can be accessed using [`ResizeSurfaceState::with`]
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum ResizeSurfaceState {
    Idle,
    Resizing {
        edges: ResizeEdge,
        /// The initial window size and location.
        initial_rect: Rectangle<i32, Logical>,
    },
    /// Resize is done, we are now waiting for last commit, to do the final move
    WaitingForLastCommit {
        edges: ResizeEdge,
        /// The initial window size and location.
        initial_rect: Rectangle<i32, Logical>,
    },
}

impl Default for ResizeSurfaceState {
    fn default() -> Self {
        ResizeSurfaceState::Idle
    }
}

impl ResizeSurfaceState {
    fn with<F, T>(surface: &WlSurface, cb: F) -> T
    where
        F: FnOnce(&mut Self) -> T,
    {
        compositor::with_states(surface, |states| {
            states.data_map.insert_if_missing(RefCell::<Self>::default);
            let state = states.data_map.get::<RefCell<Self>>().unwrap();

            cb(&mut state.borrow_mut())
        })
    }

    fn commit(&mut self) -> Option<(ResizeEdge, Rectangle<i32, Logical>)> {
        match *self {
            Self::Resizing {
                edges,
                initial_rect,
            } => Some((edges, initial_rect)),
            Self::WaitingForLastCommit {
                edges,
                initial_rect,
            } => {
                // The resize is done, let's go back to idle
                *self = Self::Idle;

                Some((edges, initial_rect))
            }
            Self::Idle => None,
        }
    }
}

pub fn handle_resize_request(
    state: &mut ThingState,
    window: Window,
    seat: Seat<ThingState>,
    serial: Serial,
    edges: ResizeEdge,
) {
    let Some(surface) = window.wl_surface().map(|s| s.into_owned()) else {
        return;
    };
    let Some(start_data) = check_grab(&seat, &surface, serial) else {
        return;
    };

    let pointer = seat.get_pointer().unwrap();

    let initial_location = state.space.element_location(&window).unwrap();
    let initial_size = window.geometry().size;

    let grab = ResizePointerGrab::start(
        start_data,
        window,
        Rectangle::from_loc_and_size(initial_location, initial_size),
        edges.into(),
    );
    pointer.set_grab(state, grab, serial, Focus::Clear);
}

/// Should be called on `WlSurface::commit`
pub fn handle_commit(space: &mut Space<Window>, surface: &WlSurface) -> Option<()> {
    let window = space
        .elements()
        .find(|w| w.wl_surface().map(|s| s.as_ref() == surface).unwrap_or(false))
        .cloned()?;

    let mut window_loc = space.element_location(&window)?;
    let geometry = window.geometry();

    let new_loc: Point<Option<i32>, Logical> = ResizeSurfaceState::with(surface, |state| {
        state
            .commit()
            .and_then(|(edges, initial_rect)| {
                // If the window is being resized by top or left, its location must be adjusted
                // accordingly.
                let new_x = if edges.intersects(ResizeEdge::LEFT) {
                    Some(initial_rect.loc.x + (initial_rect.size.w - geometry.size.w))
                } else {
                    None
                };
                let new_y = if edges.intersects(ResizeEdge::TOP) {
                    Some(initial_rect.loc.y + (initial_rect.size.h - geometry.size.h))
                } else {
                    None
                };
                (new_x, new_y).into()
            })
            .map(Into::into)
            .unwrap_or_default()
    });

    if let Some(new_x) = new_loc.x {
        window_loc.x = new_x;
    }
    if let Some(new_y) = new_loc.y {
        window_loc.y = new_y;
    }

    if new_loc.x.is_some() || new_loc.y.is_some() {
        // If TOP or LEFT side of the window got resized, we have to move it
        space.map_element(window, window_loc, false);
    }

    Some(())
}
