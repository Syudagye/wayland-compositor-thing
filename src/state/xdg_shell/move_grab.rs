use smithay::{
    desktop::Window,
    input::{
        pointer::{
            AxisFrame, ButtonEvent, Focus, GrabStartData, MotionEvent, PointerGrab,
            PointerInnerHandle, RelativeMotionEvent,
        },
        Seat,
    },
    reexports::wayland_server::protocol::wl_surface::WlSurface,
    utils::{Logical, Point, Serial},
    wayland::seat::WaylandFocus,
};

use crate::state::ThingState;

pub struct MovePointerGrab {
    pub start_data: GrabStartData<ThingState>,
    pub window: Window,
    pub initial_window_location: Point<i32, Logical>,
}

impl PointerGrab<ThingState> for MovePointerGrab {
    fn motion(
        &mut self,
        data: &mut ThingState,
        handle: &mut PointerInnerHandle<'_, ThingState>,
        _focus: Option<(WlSurface, Point<f64, Logical>)>,
        event: &MotionEvent,
    ) {
        handle.motion(data, None, event);

        let delta = event.location - self.start_data.location;
        let new_location = self.initial_window_location.to_f64() + delta;
        data.space
            .map_element(self.window.clone(), new_location.to_i32_round(), true);
    }

    fn relative_motion(
        &mut self,
        data: &mut ThingState,
        handle: &mut PointerInnerHandle<'_, ThingState>,
        _focus: Option<(WlSurface, Point<f64, Logical>)>,
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

        // =====
        // TEMPORARY LOGIC
        //
        // This will be moved and made configurable in the future.
        // For now, it's just to have a minimal way to changing focus, moving windows, etc.
        // =====

        const BTN_LEFT: u32 = 0x110;

        if !handle.current_pressed().contains(&BTN_LEFT) {
            // No more buttons are pressed, release the grab.
            handle.unset_grab(self, data, event.serial, event.time, true);
        }
    }

    fn axis(
        &mut self,
        data: &mut ThingState,
        handle: &mut PointerInnerHandle<'_, ThingState>,
        details: AxisFrame,
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

    fn unset(&mut self, _data: &mut ThingState) {}
}

pub fn handle_move_request(
    state: &mut ThingState,
    window: Window,
    seat: Seat<ThingState>,
    serial: Serial,
) {
    let Some(surface) = window.wl_surface().map(|s| s.into_owned()) else {
        return;
    };
    let Some(start_data) = super::check_grab(&seat, &surface, serial) else {
        return;
    };

    let pointer = seat.get_pointer().unwrap();

    let initial_window_location = state.space.element_location(&window).unwrap();

    let grab = MovePointerGrab {
        start_data,
        window,
        initial_window_location,
    };

    pointer.set_grab(state, grab, serial, Focus::Clear);
}
