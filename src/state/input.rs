use smithay::{
    backend::input::{Event, InputBackend, InputEvent, KeyboardKeyEvent, Device, DeviceCapability},
    input::keyboard::FilterResult,
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
            InputEvent::PointerMotionAbsolute { event } => debug!("PointerMotionAbsolute"),
            InputEvent::PointerButton { event } => debug!("PointerButton"),
            InputEvent::PointerAxis { event } => debug!("PointerAxis"),
            _ => (),
        }
    }
}
