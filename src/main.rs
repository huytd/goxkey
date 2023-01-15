mod platform;

use log::debug;
use platform::{
    run_event_listener, send_backspace, send_string, KEY_DELETE, KEY_ENTER, KEY_ESCAPE, KEY_SPACE,
    KEY_TAB, Handle,
};
use std::sync::Mutex;

static mut TYPING_BUF: Mutex<String> = Mutex::new(String::new());

fn event_handler(handle: Handle, keycode: char, shift: bool) -> bool {
    unsafe {
        let mut typing_buf = TYPING_BUF.lock().unwrap();
        match keycode {
            KEY_ENTER | KEY_TAB | KEY_SPACE | KEY_ESCAPE => {
                typing_buf.clear();
            }
            KEY_DELETE => {
                typing_buf.pop();
            }
            c => {
                typing_buf.push(if shift { c.to_ascii_uppercase() } else { c });

                // TELEX for now, checking if the last key is where the vietnamese tone happen
                if ['a', 'e', 'o', 'd', 's', 't', 'j', 'f', 'x', 'r', 'w'].contains(&keycode) {
                    let mut output = String::new();
                    vi::telex::transform_buffer(typing_buf.chars(), &mut output);
                    if !typing_buf.eq(&output) {
                        debug!("BUF {:?} - RET {:?}", typing_buf, output);
                        let backspace_count = typing_buf.chars().count() - 1;
                        debug!("  DEL {} - SEND {}", backspace_count, output);
                        _ = send_backspace(handle, backspace_count);
                        _ = send_string(handle, &output);
                        *typing_buf = output;
                        return true;
                    }
                }
            }
        }
    }
    false
}

fn main() {
    env_logger::init();
    run_event_listener(&event_handler);
}
