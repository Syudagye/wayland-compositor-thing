use smithay::{
    backend::input::{
        ButtonState, Event, InputBackend, InputEvent, KeyboardKeyEvent,
        PointerButtonEvent,
    },
    desktop::WindowSurfaceType,
    input::{
        keyboard::FilterResult,
        pointer::{ButtonEvent, MotionEvent},
    },
    utils::SERIAL_COUNTER,
};
use tracing::debug;

use super::ThingState;

impl ThingState {
    pub fn process_input_event<I: InputBackend>(&mut self, event: InputEvent<I>) {
        match event {
            InputEvent::Keyboard { event } => {
                let serial = SERIAL_COUNTER.next_serial();

                self.seat.get_keyboard().unwrap().input::<(), _>(
                    self,
                    event.key_code(),
                    event.state(),
                    serial,
                    event.time_msec(),
                    //TODO: Have magic keybinds to force quit the compositor
                    |_, _, _| FilterResult::Forward,
                );
            }
            //TODO: Handle pointer events
            InputEvent::PointerMotion { event } => debug!("PointerMotion"),
            InputEvent::PointerMotionAbsolute { event } => {
                let pointer = self.seat.get_pointer().unwrap();
                let location = pointer.current_location();

                let under =
                    self.space
                        .element_under(pointer.current_location())
                        .map(|(window, pos)| {
                            window
                                .surface_under(location - pos.to_f64(), WindowSurfaceType::ALL)
                                .unwrap()
                        });

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
            InputEvent::PointerButton { event } => {
                let pointer = self.seat.get_pointer().unwrap();
                let button = event.button_code();
                let state = event.state();
                let serial = SERIAL_COUNTER.next_serial();

                if let ButtonState::Pressed = state {
                    let keyboard = self.seat.get_keyboard().unwrap();

                    if let Some(window) = self
                        .space
                        .element_under(pointer.current_location())
                        .map(|(w, _)| w.clone())
                    {
                        self.space.raise_element(&window, true);
                        keyboard.set_focus(
                            self,
                            Some(window.toplevel().wl_surface().to_owned()),
                            serial,
                        );
                        self.space.elements().for_each(|window| {
                            window.toplevel().send_pending_configure();
                        });
                    } else {
                        self.space.elements().for_each(|window| {
                            window.set_activated(false);
                            window.toplevel().send_pending_configure();
                        });
                        keyboard.set_focus(self, None, serial);
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
            InputEvent::PointerAxis { event } => debug!("PointerAxis"),
            _ => (),
        }
    }
}
