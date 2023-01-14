mod platform;

use std::{cmp::Ordering, sync::Mutex};

use platform::{send_backspace, send_string, run_event_listener, KEY_ENTER, KEY_TAB, KEY_SPACE, KEY_ESCAPE, KEY_DELETE};

static mut TYPING_BUF: Mutex<Vec<char>> = Mutex::new(vec![]);

fn event_handler(keycode: char, shift: bool) -> bool {
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
            }
        }

        // TELEX for now, checking if the last key is where the vietnamese tone happen
        if ['a', 'e', 'o', 'd', 's', 't', 'j', 'f', 'x', 'r', 'w'].contains(&keycode) {
            let ret = vi::telex::transform_buffer(typing_buf.as_slice());
            if ret.chars().cmp(typing_buf.clone().into_iter()) != Ordering::Equal {
                // println!("BUF {:?} - RET {:?}", typing_buf, ret);
                let backspace_count = typing_buf.len();
                _ = send_backspace(backspace_count);
                _ = send_string(&ret);
                *typing_buf = ret.chars().collect();
                return true;
            }
        }
    }
    false
}

fn main() {
    run_event_listener(&event_handler);
}
