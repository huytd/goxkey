use core_foundation::runloop::{CFRunLoop, kCFRunLoopCommonModes};
use core_graphics::{event::{EventField, CGEventTap, CGEventTapLocation, CGEventTapPlacement, CGEventTapOptions, CGEventType, KeyCode, CGKeyCode, CGEventFlags, CGEvent}, event_source::{CGEventSource, self}};
use lazy_static::lazy_static;
use super::{CallbackFn, KEY_ENTER, KEY_SPACE, KEY_TAB, KEY_DELETE, KEY_ESCAPE};

struct SharedBox<T>(T);
unsafe impl<T> Send for SharedBox<T> {}
unsafe impl<T> Sync for SharedBox<T> {}
impl<T> SharedBox<T> {
    unsafe fn new(v: T) -> Self {
        Self(v)
    }
}

lazy_static! {
    static ref EVENT_SOURCE: SharedBox<CGEventSource> = unsafe { SharedBox::new(CGEventSource::new(event_source::CGEventSourceStateID::Private).unwrap()) };
    static ref BACKSPACE_DOWN: SharedBox<CGEvent> = unsafe { SharedBox::new(CGEvent::new_keyboard_event(EVENT_SOURCE.0.clone(), KeyCode::DELETE, true).unwrap()) };
    static ref BACKSPACE_UP: SharedBox<CGEvent> = unsafe { SharedBox::new(CGEvent::new_keyboard_event(EVENT_SOURCE.0.clone(), KeyCode::DELETE, false).unwrap()) };
    static ref SENDKEY: SharedBox<CGEvent> = unsafe { SharedBox::new(CGEvent::new_keyboard_event(EVENT_SOURCE.0.clone(), 0, true).unwrap()) };
}

// Modified from http://ritter.ist.psu.edu/projects/RUI/macosx/rui.c
fn get_char(keycode: CGKeyCode) -> Option<char> {
    match keycode {
        0 => Some('a'),
        1 => Some('s'),
        2 => Some('d'),
        3 => Some('f'),
        4 => Some('h'),
        5 => Some('g'),
        6 => Some('z'),
        7 => Some('x'),
        8 => Some('c'),
        9 => Some('v'),
        11 => Some('b'),
        12 => Some('q'),
        13 => Some('w'),
        14 => Some('e'),
        15 => Some('r'),
        16 => Some('y'),
        17 => Some('t'),
        31 => Some('o'),
        32 => Some('u'),
        34 => Some('i'),
        35 => Some('p'),
        37 => Some('l'),
        38 => Some('j'),
        40 => Some('k'),
        45 => Some('n'),
        46 => Some('m'),
        18 => Some('1'),
		19 => Some('2'),
		20 => Some('3'),
		21 => Some('4'),
		22 => Some('6'),
		23 => Some('5'),
		25 => Some('9'),
		26 => Some('7'),
		28 => Some('8'),
		29 => Some('0'),
		36 | 52 => Some(KEY_ENTER), // ENTER
		49 => Some(KEY_SPACE), // SPACE
		48 => Some(KEY_TAB), // TAB
		51 => Some(KEY_DELETE), // DELETE
		53 => Some(KEY_ESCAPE), // ESC
        _ => None
    }
}

pub fn send_backspace(count: usize) -> Result<(), ()> {
    for _ in 0..count {
        BACKSPACE_DOWN.0.post(CGEventTapLocation::HID);
        BACKSPACE_UP.0.post(CGEventTapLocation::HID);
    }
    Ok(())
}

pub fn send_string(string: &str) -> Result<(), ()> {
    SENDKEY.0.set_string(string);
    SENDKEY.0.post(CGEventTapLocation::HID);
    Ok(())
}

pub fn run_event_listener(callback: &CallbackFn) {
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
                if let Some(key_char) = get_char(key_code) {
                    if callback(key_char, has_shift) {
                        // block the key if already processed
                        return None;
                    }
                }
            }
            Some(event.to_owned())
        }) {
        unsafe {
            let loop_source = event_tap.mach_port.create_runloop_source(0).expect("Cannot start event tap. Make sure you have granted Accessibility Access for the application.");
            current.add_source(&loop_source, kCFRunLoopCommonModes);
            event_tap.enable();
            CFRunLoop::run_current();
        }
    }
}
