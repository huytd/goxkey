mod input;
mod platform;

use input::InputState;
use lazy_static::lazy_static;
use log::debug;
use platform::{
    run_event_listener, send_backspace, send_string, Handle, KeyModifier, KEY_DELETE, KEY_ENTER,
    KEY_ESCAPE, KEY_SPACE, KEY_TAB,
};
use std::sync::Mutex;

lazy_static! {
    static ref INPUT_STATE: Mutex<InputState> = Mutex::new(InputState::new());
}

fn event_handler(handle: Handle, keycode: Option<char>, modifiers: KeyModifier) -> bool {
    let mut input_state = INPUT_STATE.lock().unwrap();

    match keycode {
        Some(keycode) => {
            // Toggle Vietnamese input mod with Ctrl + Cmd + Space key
            if modifiers.is_control() && modifiers.is_super() && keycode == KEY_SPACE {
                input_state.toggle_vietnamese();
                return true;
            }

            if input_state.enabled {
                match keycode {
                    KEY_ENTER | KEY_TAB | KEY_SPACE | KEY_ESCAPE => {
                        input_state.clear();
                    }
                    KEY_DELETE => {
                        input_state.pop();
                    }
                    c => {
                        if modifiers.is_super() || modifiers.is_control() || modifiers.is_alt() {
                            input_state.clear();
                        } else {
                            input_state.push(if modifiers.is_shift() {
                                c.to_ascii_uppercase()
                            } else {
                                c
                            });

                            if input_state.should_process(&keycode) {
                                let output = input_state.process_key();
                                if !input_state.buffer.eq(&output) {
                                    debug!("BUF {:?} - RET {:?}", input_state.buffer, output);
                                    let backspace_count = input_state.buffer.chars().count() - 1;
                                    debug!("  DEL {} - SEND {}", backspace_count, output);
                                    _ = send_backspace(handle, backspace_count);
                                    _ = send_string(handle, &output);
                                    input_state.replace(output);
                                    return true;
                                }
                            }
                        }
                    }
                }
            }
        },
        None => {
            input_state.clear();
        }
    }
    false
}

fn main() {
    env_logger::init();
    run_event_listener(&event_handler);
}
