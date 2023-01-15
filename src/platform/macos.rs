use std::ptr;

use super::{CallbackFn, KEY_DELETE, KEY_ENTER, KEY_ESCAPE, KEY_SPACE, KEY_TAB};
use core_foundation::runloop::{kCFRunLoopCommonModes, CFRunLoop};
use core_graphics::{
    event::{
        CGEventFlags, CGEventTap, CGEventTapLocation, CGEventTapOptions,
        CGEventTapPlacement, CGEventType, CGKeyCode, EventField, KeyCode, CGEventTapProxy,
    }, sys,
};

pub type Handle = CGEventTapProxy;

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
        49 => Some(KEY_SPACE),      // SPACE
        48 => Some(KEY_TAB),        // TAB
        51 => Some(KEY_DELETE),     // DELETE
        53 => Some(KEY_ESCAPE),     // ESC
        _ => None,
    }
}


#[link(name = "CoreGraphics", kind = "framework")]
extern {
    fn CGEventTapPostEvent(proxy: CGEventTapProxy, event: sys::CGEventRef);
    fn CGEventCreateKeyboardEvent(source: sys::CGEventSourceRef, keycode: CGKeyCode,
        keydown: bool) -> sys::CGEventRef;
    fn CGEventKeyboardSetUnicodeString(event: sys::CGEventRef,
                                       length: libc::c_ulong,
                                       string: *const u16);
}

pub fn send_backspace(handle: Handle, count: usize) -> Result<(), ()> {
    for _ in 0..count {
        unsafe {
            let null_event_source = ptr::null_mut() as *mut sys::CGEventSource;
            let event_bs_down = CGEventCreateKeyboardEvent(null_event_source, KeyCode::DELETE, true);
            let event_bs_up = CGEventCreateKeyboardEvent(null_event_source, KeyCode::DELETE, false);

            CGEventTapPostEvent(handle, event_bs_down);
            CGEventTapPostEvent(handle, event_bs_up);
        }
    }
    Ok(())
}

pub fn send_string(handle: Handle, string: &str) -> Result<(), ()> {
    let utf_16_str: Vec<u16> = string.encode_utf16().collect();
    let null_event_source = ptr::null_mut() as *mut sys::CGEventSource;

    unsafe {
        let event_str = CGEventCreateKeyboardEvent(null_event_source, 0, true);
        let buflen = utf_16_str.len() as libc::c_ulong;
        let bufptr = utf_16_str.as_ptr();
        CGEventKeyboardSetUnicodeString(event_str, buflen, bufptr);
        CGEventTapPostEvent(handle, event_str);
    }
    Ok(())
}

pub fn run_event_listener(callback: &CallbackFn) {
    let current = CFRunLoop::get_current();
    if let Ok(event_tap) = CGEventTap::new(
        CGEventTapLocation::HID,
        CGEventTapPlacement::HeadInsertEventTap,
        CGEventTapOptions::Default,
        vec![CGEventType::KeyDown],
        |proxy, _, event| {
            let source_state_id = event.get_integer_value_field(EventField::EVENT_SOURCE_STATE_ID);
            if source_state_id == 1 {
                let key_code =
                    event.get_integer_value_field(EventField::KEYBOARD_EVENT_KEYCODE) as CGKeyCode;
                let has_shift = event.get_flags().contains(CGEventFlags::CGEventFlagShift);
                if let Some(key_char) = get_char(key_code) {
                    if callback(proxy, key_char, has_shift) {
                        // block the key if already processed
                        return None;
                    }
                }
            }
            Some(event.to_owned())
        },
    ) {
        unsafe {
            let loop_source = event_tap.mach_port.create_runloop_source(0).expect("Cannot start event tap. Make sure you have granted Accessibility Access for the application.");
            current.add_source(&loop_source, kCFRunLoopCommonModes);
            event_tap.enable();
            CFRunLoop::run_current();
        }
    }
}
