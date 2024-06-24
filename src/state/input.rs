use smithay::{
    backend::input::{
        AbsolutePositionEvent, ButtonState, Event, InputBackend, InputEvent, KeyboardKeyEvent,
        PointerButtonEvent,
    },
    input::{
        keyboard::FilterResult,
        pointer::{ButtonEvent, MotionEvent},
    },
    utils::SERIAL_COUNTER,
    wayland::seat::WaylandFocus,
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
        let output = self.space.outputs().next().unwrap();
        let output_geo = self.space.output_geometry(output).unwrap();
        let location = event.position_transformed(output_geo.size) + output_geo.loc.to_f64();

        let under = self.surface_under(location);

        let pointer = self.pointer_handle.clone();
        pointer.motion(
            self,
            under,
            &MotionEvent {
                location,
                serial: SERIAL_COUNTER.next_serial(),
                time: event.time_msec(),
            },
        );
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

        if state == ButtonState::Pressed && !pointer.is_grabbed() {
            // let keyboard = self.seat.get_keyboard().unwrap();
            let keyboard = self.keyboard_handle.clone();

            if let Some(window) = self
                .space
                .element_under(pointer.current_location())
                .map(|(w, _)| w.clone())
            {
                let surface = window.wl_surface().map(|h| h.into_owned());

                self.space.raise_element(&window, true);
                keyboard.set_focus(self, surface, serial);

                self.space.elements().for_each(|w| {
                    if let Some(toplevel) = w.toplevel() {
                        toplevel.send_pending_configure();
                    }
                });
            } else {
                keyboard.set_focus(self, None, serial);
                self.space.elements().for_each(|w| {
                    w.set_activated(false);
                    if let Some(toplevel) = w.toplevel() {
                        toplevel.send_pending_configure();
                    }
                });
            }
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
    }

    fn process_pointer_axis<I: InputBackend>(
        &mut self,
        event: <I as InputBackend>::PointerAxisEvent,
    ) {
    }
}
