use std::ptr;

use super::{
    CallbackFn, KEY_DELETE, KEY_ENTER, KEY_ESCAPE, KEY_SPACE, KEY_TAB, KeyModifier,
};
use core_foundation::runloop::{kCFRunLoopCommonModes, CFRunLoop};
use core_graphics::{
    event::{
        CGEventFlags, CGEventTapLocation, CGEventTapOptions, CGEventTapPlacement, CGEventTapProxy,
        CGEventType, CGKeyCode, EventField, KeyCode,
    },
    sys,
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
        vec![CGEventType::KeyDown],
        |proxy, _, event| {
            let source_state_id = event.get_integer_value_field(EventField::EVENT_SOURCE_STATE_ID);
            if source_state_id == 1 {
                let key_code =
                    event.get_integer_value_field(EventField::KEYBOARD_EVENT_KEYCODE) as CGKeyCode;
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
