use std::{env, path::PathBuf, ptr};

mod macos_ext;
use cocoa::{
    base::{nil, YES},
    foundation::NSDictionary,
};
use core_graphics::{
    event::{
        CGEventFlags, CGEventTapLocation, CGEventTapOptions, CGEventTapPlacement, CGEventType,
        CGKeyCode, EventField, KeyCode,
    },
    sys,
};
use druid::{commands::HIDE_APPLICATION, Selector};
use objc::{class, msg_send, sel, sel_impl};

pub use macos_ext::SystemTray;
pub use macos_ext::SystemTrayMenuItemKey;

use crate::input::KEYBOARD_LAYOUT_CHARACTER_MAP;
use core_foundation::runloop::{kCFRunLoopCommonModes, CFRunLoop};

pub use self::macos_ext::Handle;
use self::macos_ext::{
    kAXTrustedCheckOptionPrompt, new_tap, AXIsProcessTrustedWithOptions,
    CGEventCreateKeyboardEvent, CGEventKeyboardSetUnicodeString, CGEventTapPostEvent,
};

use super::{CallbackFn, KeyModifier, KEY_DELETE, KEY_ENTER, KEY_ESCAPE, KEY_SPACE, KEY_TAB};

pub const SYMBOL_SHIFT: &str = "⇧";
pub const SYMBOL_CTRL: &str = "⌃";
pub const SYMBOL_SUPER: &str = "⌘";
pub const SYMBOL_ALT: &str = "⌥";

pub const HIDE_COMMAND: Selector = HIDE_APPLICATION;

pub fn get_home_dir() -> Option<PathBuf> {
    env::var("HOME").ok().map(PathBuf::from)
}

// List of keycode: https://eastmanreference.com/complete-list-of-applescript-key-codes
fn get_char(keycode: CGKeyCode) -> Option<char> {
    if let Some(key_map) = unsafe { KEYBOARD_LAYOUT_CHARACTER_MAP.get() } {
        return match keycode {
            0 => Some(key_map[&'a']),
            1 => Some(key_map[&'s']),
            2 => Some(key_map[&'d']),
            3 => Some(key_map[&'f']),
            4 => Some(key_map[&'h']),
            5 => Some(key_map[&'g']),
            6 => Some(key_map[&'z']),
            7 => Some(key_map[&'x']),
            8 => Some(key_map[&'c']),
            9 => Some(key_map[&'v']),
            11 => Some(key_map[&'b']),
            12 => Some(key_map[&'q']),
            13 => Some(key_map[&'w']),
            14 => Some(key_map[&'e']),
            15 => Some(key_map[&'r']),
            16 => Some(key_map[&'y']),
            17 => Some(key_map[&'t']),
            31 => Some(key_map[&'o']),
            32 => Some(key_map[&'u']),
            34 => Some(key_map[&'i']),
            35 => Some(key_map[&'p']),
            37 => Some(key_map[&'l']),
            38 => Some(key_map[&'j']),
            40 => Some(key_map[&'k']),
            45 => Some(key_map[&'n']),
            46 => Some(key_map[&'m']),
            18 => Some(key_map[&'1']),
            19 => Some(key_map[&'2']),
            20 => Some(key_map[&'3']),
            21 => Some(key_map[&'4']),
            22 => Some(key_map[&'6']),
            23 => Some(key_map[&'5']),
            25 => Some(key_map[&'9']),
            26 => Some(key_map[&'7']),
            28 => Some(key_map[&'8']),
            29 => Some(key_map[&'0']),
            27 => Some(key_map[&'-']),
            33 => Some(key_map[&'[']),
            30 => Some(key_map[&']']),
            41 => Some(key_map[&';']),
            43 => Some(key_map[&',']),
            36 | 52 => Some(KEY_ENTER), // ENTER
            49 => Some(KEY_SPACE),      // SPACE
            48 => Some(KEY_TAB),        // TAB
            51 => Some(KEY_DELETE),     // DELETE
            53 => Some(KEY_ESCAPE),     // ESC
            _ => None,
        };
    }
    None
}

pub fn send_backspace(handle: Handle, count: usize) -> Result<(), ()> {
    let null_event_source = ptr::null_mut() as *mut sys::CGEventSource;
    let (event_bs_down, event_bs_up) = unsafe {
        (
            CGEventCreateKeyboardEvent(null_event_source, KeyCode::DELETE, true),
            CGEventCreateKeyboardEvent(null_event_source, KeyCode::DELETE, false),
        )
    };
    for _ in 0..count {
        unsafe {
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
    if let Ok(event_tap) = new_tap::CGEventTap::new(
        CGEventTapLocation::HID,
        CGEventTapPlacement::HeadInsertEventTap,
        CGEventTapOptions::Default,
        vec![
            CGEventType::KeyDown,
            CGEventType::RightMouseDown,
            CGEventType::LeftMouseDown,
            CGEventType::OtherMouseDown,
        ],
        |proxy, _, event| {
            match event.get_type() {
                CGEventType::KeyDown => {
                    let source_state_id =
                        event.get_integer_value_field(EventField::EVENT_SOURCE_STATE_ID);
                    if source_state_id == 1 {
                        let key_code = event
                            .get_integer_value_field(EventField::KEYBOARD_EVENT_KEYCODE)
                            as CGKeyCode;
                        let mut modifiers = KeyModifier::new();
                        let flags = event.get_flags();
                        if flags.contains(CGEventFlags::CGEventFlagShift)
                            || flags.contains(CGEventFlags::CGEventFlagAlphaShift)
                        {
                            modifiers.add_shift();
                        }
                        if flags.contains(CGEventFlags::CGEventFlagControl) {
                            modifiers.add_control();
                        }
                        if flags.contains(CGEventFlags::CGEventFlagCommand) {
                            modifiers.add_super();
                        }
                        if flags.contains(CGEventFlags::CGEventFlagAlternate) {
                            modifiers.add_alt();
                        }

                        if callback(proxy, get_char(key_code), modifiers) {
                            // block the key if already processed
                            return None;
                        }
                    }
                }
                _ => {
                    // A callback with None char for dismissing the tracking buffer
                    // but it's up to the implementor on the behavior
                    callback(proxy, None, KeyModifier::new());
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

pub fn ensure_accessibility_permission() -> bool {
    unsafe {
        let options = NSDictionary::dictionaryWithObject_forKey_(
            nil,
            msg_send![class!(NSNumber), numberWithBool: YES],
            kAXTrustedCheckOptionPrompt as _,
        );
        return AXIsProcessTrustedWithOptions(options as _);
    }
}
