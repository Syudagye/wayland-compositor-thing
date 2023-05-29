use smithay::{
    backend::input::{
        AbsolutePositionEvent, ButtonState, Event, InputBackend, InputEvent, KeyboardKeyEvent,
        PointerButtonEvent,
    },
    desktop::WindowSurfaceType,
    input::{
        keyboard::{xkb, FilterResult, ModifiersState},
        pointer::{ButtonEvent, MotionEvent},
    },
    utils::SERIAL_COUNTER,
    wayland::{seat::WaylandFocus, shell::xdg::XdgShellHandler},
};
use tracing::debug;

use super::{elements::WindowElement, ThingState};

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
                let output = self.space.outputs().next().unwrap();
                let output_geo = self.space.output_geometry(output).unwrap();
                let location =
                    event.position_transformed(output_geo.size) + output_geo.loc.to_f64();

                let under = self.surface_under(location);

                let pointer = self.seat.get_pointer().unwrap();
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

                if state == ButtonState::Pressed && !pointer.is_grabbed() {
                    let keyboard = self.seat.get_keyboard().unwrap();

                    if let Some(window) = self
                        .space
                        .element_under(pointer.current_location())
                        .map(|(w, _)| w.clone())
                    {
                        let surface = match window.clone() {
                            WindowElement::Wayland(w) => w.toplevel().wl_surface().clone(),
                            // idk either here
                            WindowElement::X11(w) => w.wl_surface().unwrap().clone(),
                        };

                        self.space.raise_element(&window, true);
                        keyboard.set_focus(self, Some(surface), serial);
                        self.space.elements().for_each(|window| match window {
                            WindowElement::Wayland(w) => {
                                w.toplevel().send_pending_configure();
                            }
                            WindowElement::X11(_) => (),
                        });
                    } else {
                        self.space.elements().for_each(|window| match window {
                            WindowElement::Wayland(w) => {
                                w.set_activated(false);
                                w.toplevel().send_pending_configure();
                            },
                            WindowElement::X11(w) => {
                                w.set_activated(false).unwrap();
                            }
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
