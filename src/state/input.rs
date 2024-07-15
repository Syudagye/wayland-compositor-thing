use smithay::{
    backend::input::{
        AbsolutePositionEvent, Axis, AxisRelativeDirection, AxisSource, ButtonState, Event,
        InputBackend, InputEvent, KeyboardKeyEvent, PointerAxisEvent, PointerButtonEvent,
    },
    input::{
        keyboard::FilterResult,
        pointer::{AxisFrame, ButtonEvent, MotionEvent, PointerHandle},
    },
    utils::{Logical, Point, Serial, SERIAL_COUNTER},
    wayland::{relative_pointer, seat::WaylandFocus},
};
use tracing::{debug, trace};

use super::ThingState;

impl ThingState {
    pub fn process_input_event<I: InputBackend>(&mut self, event: InputEvent<I>) {
        match event {
            // InputEvent::DeviceAdded { device } => self.process_device_added(device),
            // InputEvent::DeviceRemoved { device } => self.process_device_removed(device),
            InputEvent::Keyboard { event } => self.process_keyboard::<I>(event),
            //TODO: Handle pointer events
            InputEvent::PointerMotion { event } => self.process_pointer_motion::<I>(event),
            InputEvent::PointerMotionAbsolute { event } => {
                self.process_pointer_motion_absolute::<I>(event)
            }
            InputEvent::PointerButton { event } => self.process_pointer_button::<I>(event),
            InputEvent::PointerAxis { event } => self.process_pointer_axis::<I>(event),
            _ => (),
        }
    }

    // fn process_device_added<I: InputBackend>(&mut self, device: <I as InputBackend>::Device) {
    //     // TODO: Handle device hotplug here
    // }
    // fn process_device_removed<I: InputBackend>(&mut self, device: <I as InputBackend>::Device) {
    //     // TODO: Handle device hot-removal here
    // }

    fn process_keyboard<I: InputBackend>(&mut self, event: <I as InputBackend>::KeyboardKeyEvent) {
        let serial = SERIAL_COUNTER.next_serial();
        let kbh = self.keyboard_handle.clone();
        kbh.input::<(), _>(
            self,
            event.key_code(),
            event.state(),
            serial,
            event.time_msec(),
            //TODO: Have magic keybinds to force quit the compositor
            |_, _, _| FilterResult::Forward,
        );
    }

    fn process_pointer_motion<I: InputBackend>(
        &mut self,
        event: <I as InputBackend>::PointerMotionEvent,
    ) {
        trace!("Pointer Motion");
    }

    fn process_pointer_motion_absolute<I: InputBackend>(
        &mut self,
        event: <I as InputBackend>::PointerMotionAbsoluteEvent,
    ) {
        trace!("Pointer Motion Abs");
        let output = self.space.outputs().next().unwrap();
        let output_geo = self.space.output_geometry(output).unwrap();
        let location = event.position_transformed(output_geo.size) + output_geo.loc.to_f64();

        let element_under = self.surface_under(location);

        let pointer = self.pointer_handle.clone();
        pointer.motion(
            self,
            element_under,
            &MotionEvent {
                location,
                serial: SERIAL_COUNTER.next_serial(),
                time: event.time_msec(),
            },
        );
        pointer.frame(self);
    }

    fn process_pointer_button<I: InputBackend>(
        &mut self,
        event: <I as InputBackend>::PointerButtonEvent,
    ) {
        let pointer = self.seat.get_pointer().unwrap();
        let button = event.button_code();
        let state = event.state();
        let serial = SERIAL_COUNTER.next_serial();

        // =====
        // TEMPORARY LOGIC
        //
        // This will be moved and made configurable in the future.
        // For now, it's just to have a minimal way to changing focus, moving windows, etc.
        // =====

        if state == ButtonState::Pressed {
            self.update_keyboard_focus_for_cursor_position(pointer.current_location(), serial);
        }

        pointer.button(
            self,
            &ButtonEvent {
                button,
                state,
                serial,
                time: event.time_msec(),
            },
        );
        pointer.frame(self);
    }

    fn process_pointer_axis<I: InputBackend>(
        &mut self,
        event: <I as InputBackend>::PointerAxisEvent,
    ) {
        let pointer = self.pointer_handle.clone();

        let source = event.source();
        let axis = (
            event.amount(Axis::Horizontal).unwrap_or(0.0),
            event.amount(Axis::Vertical).unwrap_or(0.0),
        );
        let relative_direction = (
            event.relative_direction(Axis::Horizontal),
            event.relative_direction(Axis::Vertical),
        );

        let mut v120: Option<(i32, i32)> = None;
        let mut stop = (false, false);

        match source {
            AxisSource::Finger => {
                stop.0 = event.amount(Axis::Horizontal) == Some(0.0);
                stop.1 = event.amount(Axis::Vertical) == Some(0.0);
            }
            AxisSource::Wheel | AxisSource::WheelTilt => {
                let v = event.amount_v120(Axis::Vertical);
                let h = event.amount_v120(Axis::Horizontal);

                v120 = match (h, v) {
                    (Some(h), Some(v)) => Some((h as i32, v as i32)),
                    _ => None,
                };
            }
            _ => (),
        }

        trace!("{:?}, {:?}", axis, v120);

        pointer.axis(
            self,
            AxisFrame {
                source: Some(source),
                relative_direction,
                time: event.time_msec(),
                axis,
                v120,
                stop,
            },
        );
        pointer.frame(self);
    }

    fn update_keyboard_focus_for_cursor_position(
        &mut self,
        location: Point<f64, Logical>,
        serial: Serial,
    ) {
        let keyboard = self.keyboard_handle.clone();

        if let Some((window, _)) = self
            .space
            .element_under(location)
            .map(|(w, p)| (w.clone(), p))
        {
            self.space.raise_element(&window, true);
            keyboard.set_focus(self, window.wl_surface().map(|s| s.into_owned()), serial);
        }
    }
}
