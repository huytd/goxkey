use std::{env, path::PathBuf, ptr};

use crate::input::KEYBOARD_LAYOUT_CHARACTER_MAP;

use super::{CallbackFn, KeyModifier, KEY_DELETE, KEY_ENTER, KEY_ESCAPE, KEY_SPACE, KEY_TAB};
use core_foundation::runloop::{kCFRunLoopCommonModes, CFRunLoop};
use core_graphics::{
    event::{
        CGEventFlags, CGEventTapLocation, CGEventTapOptions, CGEventTapPlacement, CGEventTapProxy,
        CGEventType, CGKeyCode, EventField, KeyCode,
    },
    sys,
};

pub type Handle = CGEventTapProxy;

pub const SYMBOL_SHIFT: &str = "⇧";
pub const SYMBOL_CTRL: &str = "⌃";
pub const SYMBOL_SUPER: &str = "⌘";
pub const SYMBOL_ALT: &str = "⌥";

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

#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
    fn CGEventTapPostEvent(proxy: CGEventTapProxy, event: sys::CGEventRef);
    fn CGEventCreateKeyboardEvent(
        source: sys::CGEventSourceRef,
        keycode: CGKeyCode,
        keydown: bool,
    ) -> sys::CGEventRef;
    fn CGEventKeyboardSetUnicodeString(
        event: sys::CGEventRef,
        length: libc::c_ulong,
        string: *const u16,
    );
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

mod new_tap {
    use std::{
        mem::{self, ManuallyDrop},
        ptr,
    };

    use core_foundation::{
        base::TCFType,
        mach_port::{CFMachPort, CFMachPortRef},
    };
    use core_graphics::{
        event::{
            CGEvent, CGEventMask, CGEventTapCallBackFn, CGEventTapLocation, CGEventTapOptions,
            CGEventTapPlacement, CGEventTapProxy, CGEventType,
        },
        sys,
    };
    use foreign_types::ForeignType;
    use libc::c_void;

    type CGEventTapCallBackInternal = unsafe extern "C" fn(
        proxy: CGEventTapProxy,
        etype: CGEventType,
        event: sys::CGEventRef,
        user_info: *const c_void,
    ) -> sys::CGEventRef;

    #[link(name = "CoreGraphics", kind = "framework")]
    extern "C" {
        fn CGEventTapCreate(
            tap: CGEventTapLocation,
            place: CGEventTapPlacement,
            options: CGEventTapOptions,
            eventsOfInterest: CGEventMask,
            callback: CGEventTapCallBackInternal,
            userInfo: *const c_void,
        ) -> CFMachPortRef;
        fn CGEventTapEnable(tap: CFMachPortRef, enable: bool);
    }

    #[no_mangle]
    unsafe extern "C" fn cg_new_tap_callback_internal(
        _proxy: CGEventTapProxy,
        _etype: CGEventType,
        _event: sys::CGEventRef,
        _user_info: *const c_void,
    ) -> sys::CGEventRef {
        let callback = _user_info as *mut CGEventTapCallBackFn;
        let event = CGEvent::from_ptr(_event);
        let new_event = (*callback)(_proxy, _etype, &event);
        match new_event {
            Some(new_event) => ManuallyDrop::new(new_event).as_ptr(),
            None => {
                mem::forget(event);
                ptr::null_mut() as sys::CGEventRef
            }
        }
    }

    /* Generate an event mask for a single type of event. */
    macro_rules! CGEventMaskBit {
        ($eventType:expr) => {
            1 << $eventType as CGEventMask
        };
    }

    pub struct CGEventTap<'tap_life> {
        pub mach_port: CFMachPort,
        pub callback_ref:
            Box<dyn Fn(CGEventTapProxy, CGEventType, &CGEvent) -> Option<CGEvent> + 'tap_life>,
    }

    impl<'tap_life> CGEventTap<'tap_life> {
        pub fn new<F: Fn(CGEventTapProxy, CGEventType, &CGEvent) -> Option<CGEvent> + 'tap_life>(
            tap: CGEventTapLocation,
            place: CGEventTapPlacement,
            options: CGEventTapOptions,
            events_of_interest: std::vec::Vec<CGEventType>,
            callback: F,
        ) -> Result<CGEventTap<'tap_life>, ()> {
            let event_mask: CGEventMask = events_of_interest
                .iter()
                .fold(CGEventType::Null as CGEventMask, |mask, &etype| {
                    mask | CGEventMaskBit!(etype)
                });
            let cb = Box::new(Box::new(callback) as CGEventTapCallBackFn);
            let cbr = Box::into_raw(cb);
            unsafe {
                let event_tap_ref = CGEventTapCreate(
                    tap,
                    place,
                    options,
                    event_mask,
                    cg_new_tap_callback_internal,
                    cbr as *const c_void,
                );

                if !event_tap_ref.is_null() {
                    Ok(Self {
                        mach_port: (CFMachPort::wrap_under_create_rule(event_tap_ref)),
                        callback_ref: Box::from_raw(cbr),
                    })
                } else {
                    _ = Box::from_raw(cbr);
                    Err(())
                }
            }
        }

        pub fn enable(&self) {
            unsafe { CGEventTapEnable(self.mach_port.as_concrete_TypeRef(), true) }
        }
    }
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
                        if flags.contains(CGEventFlags::CGEventFlagShift) {
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
