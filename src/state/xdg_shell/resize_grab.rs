use std::cell::RefCell;

use smithay::{
    desktop::{Space, Window},
    input::{
        pointer::{
            ButtonEvent, GrabStartData, MotionEvent, PointerGrab, PointerInnerHandle,
            RelativeMotionEvent,
        },
        SeatHandler,
    },
    reexports::{
        wayland_protocols::xdg::shell::server::xdg_toplevel::{ResizeEdge, State},
        wayland_server::protocol::wl_surface::WlSurface,
    },
    utils::{Logical, Point, Rectangle, Size},
    wayland::{compositor, shell::xdg::SurfaceCachedState},
};

use crate::state::ThingState;

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

        ResizeSurfaceState::with(window.toplevel().wl_surface(), |state| {
            *state = ResizeSurfaceState::Resizing {
                edges,
                initial_rect,
            };
        });

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
            <ThingState as SeatHandler>::PointerFocus,
            Point<i32, Logical>,
        )>,
        event: &MotionEvent,
    ) {
        handle.motion(data, None, event);

        let mut delta = event.location - self.start_data.location;


        match self.edges {
            ResizeEdge::Top | ResizeEdge::Bottom => {
                delta.x = 0.0;
                if self.edges == ResizeEdge::Top {
                    delta.y = -delta.y;
                }
            }
            ResizeEdge::Left | ResizeEdge::Right => {
                delta.y = 0.0;
                if self.edges == ResizeEdge::Left {
                    delta.x = -delta.x;
                }
            }
            ResizeEdge::TopLeft => {
                delta.x = -delta.x;
                delta.y = -delta.y;
            }
            ResizeEdge::BottomLeft => delta.x = -delta.x,
            ResizeEdge::TopRight => delta.y = -delta.y,
            _ => (),
        };

        let (min_size, max_size) =
            compositor::with_states(self.window.toplevel().wl_surface(), |states| {
                let data = states.cached_state.current::<SurfaceCachedState>();
                (data.min_size, data.max_size)
            });

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

        let window = self.window.toplevel();
        window.with_pending_state(|state| {
            state.states.set(State::Resizing);
            state.size = Some(self.last_window_size);
        });

        window.send_pending_configure();
    }

    fn relative_motion(
        &mut self,
        data: &mut ThingState,
        handle: &mut PointerInnerHandle<'_, ThingState>,
        _focus: Option<(
            <ThingState as SeatHandler>::PointerFocus,
            Point<i32, Logical>,
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

        // The button is a button code as defined in the
        // Linux kernel's linux/input-event-codes.h header file, e.g. BTN_LEFT.
        const BTN_LEFT: u32 = 0x110;

        if !handle.current_pressed().contains(&BTN_LEFT) {
            // No more buttons are pressed, release the grab.
            handle.unset_grab(data, event.serial, event.time);

            ResizeSurfaceState::with(self.window.toplevel().wl_surface(), |state| {
                *state = ResizeSurfaceState::WaitingForLastCommit {
                    edges: self.edges,
                    initial_rect: self.initial_rect,
                };
            });

            let xdg = self.window.toplevel();
            xdg.with_pending_state(|state| {
                state.states.unset(State::Resizing);
                state.size = Some(self.last_window_size);
            });

            xdg.send_pending_configure();
        }
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

/// Should be called on `WlSurface::commit`
pub fn handle_commit(space: &mut Space<Window>, surface: &WlSurface) -> Option<()> {
    let window = space
        .elements()
        .find(|w| w.toplevel().wl_surface() == surface)
        .cloned()?;

    let mut window_loc = space.element_location(&window)?;
    let geometry = window.geometry();

    let new_loc: Point<Option<i32>, Logical> = ResizeSurfaceState::with(surface, |state| {
        state
            .commit()
            .and_then(|(edges, initial_rect)| {
                // If the window is being resized by top or left, its location must be adjusted
                // accordingly.
                let new_x = if edges == ResizeEdge::Left
                    || edges == ResizeEdge::BottomLeft
                    || edges == ResizeEdge::TopLeft
                {
                    Some(initial_rect.loc.x + (initial_rect.size.w - geometry.size.w))
                } else {
                    None
                };
                let new_y = if edges == ResizeEdge::Top
                    || edges == ResizeEdge::TopRight
                    || edges == ResizeEdge::TopLeft
                {
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
