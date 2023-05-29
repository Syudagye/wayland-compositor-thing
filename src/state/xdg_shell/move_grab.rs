use smithay::{
    desktop::Window,
    input::{
        pointer::{
            AxisFrame, ButtonEvent, GrabStartData, MotionEvent, PointerGrab, PointerInnerHandle,
            RelativeMotionEvent, Focus,
        },
        SeatHandler, Seat,
    },
    utils::{Logical, Point, Serial},
};

use crate::state::{elements::WindowElement, ThingState};

use super::check_grab;

pub struct MovePointerGrab {
    pub start_data: GrabStartData<ThingState>,
    pub window: WindowElement,
    pub initial_window_location: Point<i32, Logical>,
}

impl PointerGrab<ThingState> for MovePointerGrab {
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

        let delta = event.location - self.start_data.location;
        let new_location = self.initial_window_location.to_f64() + delta;
        data.space
            .map_element(self.window.clone(), new_location.to_i32_round(), true);
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
}

pub fn handle_move_request(
    state: &mut ThingState,
    window: WindowElement,
    seat: Seat<ThingState>,
    serial: Serial,
) {
    let surface = match window.clone() {
        WindowElement::Wayland(w) => w.toplevel().wl_surface().clone(),
        WindowElement::X11(w) => {
            if let Some(surface) = w.wl_surface() {
                surface
            } else {
                return;
            }
        }
    };
    let Some(start_data) = check_grab(&seat, &surface, serial) else {
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
