use std::{sync::mpsc::channel, thread, ascii::AsciiExt};

use core_foundation::runloop::{CFRunLoop, kCFRunLoopCommonModes};
use core_graphics::{event::{EventField, CGEventTap, CGEventTapLocation, CGEventTapPlacement, CGEventTapOptions, CGEventType, KeyCode, CGKeyCode, CGEventFlags, CGEvent}, event_source::{CGEventSource, self}};

use crate::keymap::get_printable_char;

mod keymap;

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

fn main() {
    let (tx, rx) = channel();

    thread::spawn(move || {
        let current = CFRunLoop::get_current();
        if let Ok(event_tap) = CGEventTap::new(
            CGEventTapLocation::HID,
            CGEventTapPlacement::HeadInsertEventTap,
            CGEventTapOptions::Default,
            vec![CGEventType::KeyDown],
            |_, _, event| {
                let source_state_id = event.get_integer_value_field(EventField::EVENT_SOURCE_STATE_ID);
                if source_state_id == 1 {
                    let key_code = event.get_integer_value_field(EventField::KEYBOARD_EVENT_KEYCODE) as CGKeyCode;
                    let has_shift = event.get_flags().contains(CGEventFlags::CGEventFlagShift);
                    _ = tx.send((key_code, has_shift));
                }
                None
            }) {
            unsafe {
                let loop_source = event_tap.mach_port.create_runloop_source(0).expect("Somethings is bad ");
                current.add_source(&loop_source, kCFRunLoopCommonModes);
                event_tap.enable();
                CFRunLoop::run_current();
            }
        }
    });

    let mut buf = vec![];
    let mut current_word = String::new();
    loop {
        if let Ok((key_code, has_shift)) = rx.recv() {
            match key_code {
                KeyCode::SPACE | KeyCode::RETURN => {
                    buf.clear();
                    current_word.clear();
                }
                KeyCode::DELETE => {
                    buf.pop();
                }
                c => {
                    if let Some(chr) = get_printable_char(c) {
                        buf.push(if has_shift { chr.to_ascii_uppercase() } else { chr });
                    } else {
                        buf.clear();
                    }
                }
            }
            println!("Buffer: {:?} - Last word: {} - {}", buf, current_word, current_word.chars().count());
            if buf.len() > 0 {
                let result = vi::telex::transform_buffer(&buf);
                println!("Transformed: {:?}", result);
                let del_count = if !current_word.is_empty() { current_word.chars().count() + 1 } else { buf.len() };
                _ = send_backspace(del_count);
                _ = send_string(&result);
                current_word = result.clone();
            }
        }
    }

}
