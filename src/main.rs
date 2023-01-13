use std::{cmp::Ordering, sync::Mutex};

use core_foundation::runloop::{CFRunLoop, kCFRunLoopCommonModes};
use core_graphics::{event::{EventField, CGEventTap, CGEventTapLocation, CGEventTapPlacement, CGEventTapOptions, CGEventType, KeyCode, CGKeyCode, CGEventFlags, CGEvent, CGEventTapProxy}, event_source::{CGEventSource, self}};

use crate::keymap::get_printable_char;

mod keymap;

static mut TYPING_BUF: Mutex<Vec<char>> = Mutex::new(vec![]);

fn send_backspace(count: usize) -> Result<(), ()> {
    let source = CGEventSource::new(event_source::CGEventSourceStateID::Private)?;
    let backspace_down = CGEvent::new_keyboard_event(source.clone(), KeyCode::DELETE, true)?;
    let backspace_up = CGEvent::new_keyboard_event(source.clone(), KeyCode::DELETE, false)?;

    for _ in 0..count {
        backspace_down.post(CGEventTapLocation::HID);
        backspace_up.post(CGEventTapLocation::HID);
    }

    Ok(())
}

fn send_string(string: &str) -> Result<(), ()> {
    let source = CGEventSource::new(event_source::CGEventSourceStateID::Private)?;
    let event = CGEvent::new_keyboard_event(source, 0, true)?;
    event.set_string(string);
    event.post(CGEventTapLocation::HID);
    Ok(())
}

fn callback(_proxy: CGEventTapProxy, _event_type: CGEventType, event: &CGEvent) -> Option<CGEvent> {
    unsafe {
        let mut typing_buf = TYPING_BUF.lock().unwrap();
        let source_state_id = event.get_integer_value_field(EventField::EVENT_SOURCE_STATE_ID);
        if source_state_id == 1 {
            let key_code = event.get_integer_value_field(EventField::KEYBOARD_EVENT_KEYCODE) as CGKeyCode;
            let has_shift = event.get_flags().contains(CGEventFlags::CGEventFlagShift);
            match key_code {
                KeyCode::SPACE | KeyCode::RETURN => {
                    typing_buf.clear();
                }
                KeyCode::DELETE => {
                    typing_buf.pop();
                }
                c => {
                    if let Some(chr) = get_printable_char(c) {
                        typing_buf.push(if has_shift { chr.to_ascii_uppercase() } else { chr });
                    } else {
                        typing_buf.clear();
                    }
                }
            }

            if !typing_buf.is_empty() {
                let ret = vi::telex::transform_buffer(typing_buf.as_slice());
                if ret.chars().cmp(typing_buf.clone().into_iter()) != Ordering::Equal {
                    // println!("BUF {:?} - RET {:?}", typing_buf, ret);
                    let backspace_count = typing_buf.len();
                    _ = send_backspace(backspace_count);
                    _ = send_string(&ret);
                    *typing_buf = ret.chars().collect();
                    return None;
                }
            }
        }
        Some(event.to_owned())
    }
}

fn main() {
    let current = CFRunLoop::get_current();
    if let Ok(event_tap) = CGEventTap::new(
        CGEventTapLocation::HID,
        CGEventTapPlacement::HeadInsertEventTap,
        CGEventTapOptions::Default,
        vec![CGEventType::KeyDown],
        callback) {
        unsafe {
            let loop_source = event_tap.mach_port.create_runloop_source(0).expect("Cannot start event tap. Make sure you have granted Accessibility Access for the application.");
            current.add_source(&loop_source, kCFRunLoopCommonModes);
            event_tap.enable();
            CFRunLoop::run_current();
        }
    }
}
