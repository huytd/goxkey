#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(improper_ctypes)]

use libc;
use std::sync::mpsc::Sender;

#[derive(Debug)]
#[repr(C)]
pub enum EventType {
    KeyDown,
    KeyUp,
    FlagsChanged,
}

#[derive(Debug)]
#[repr(C)]
pub struct KeyEvent {
    pub etype: EventType,
    pub code: u16,
}

// Opaque Pointer Types
pub type Pointer = *mut libc::c_void;
pub type CGEventRef = Pointer;
pub type CFMachPortRef = Pointer;
pub type CFRunLoopSourceRef = Pointer;

// Integer Types
pub type CGEventField = u32;
pub type CGEventMask = u64;
pub type CGEventTapLocation = u32;
pub type CGEventTapOptions = u32;
pub type CGEventTapPlacement = u32;
pub type CGEventType = u32;
pub type CGKeyCode = u16;

// Callback Type
pub type CGEventTapCallBack = extern "C" fn(Pointer, CGEventMask, CGEventRef, &Sender<KeyEvent>) -> CGEventRef;

// Constants
pub const kCGEventKeyDown: CGEventType = 10;
pub const kCGEventKeyUp: CGEventType = 11;
pub const kCGEventFlagsChanged: CGEventType = 12;
pub const kCGHIDEventTap: CGEventTapLocation = 0;
pub const kCGSessionEventTap: CGEventTapLocation = 1;
pub const kCGHeadInsertEventTap: CGEventTapPlacement = 0;
pub const kCGKeyboardEventKeycode: CGEventField = 9;
pub const kCGEventSourceStateID: CGEventField = 45;

pub const KEY_A: CGKeyCode = 0x00;
pub const KEY_B: CGKeyCode = 0x0B;
pub const KEY_C: CGKeyCode = 0x08;
pub const KEY_D: CGKeyCode = 0x02;
pub const KEY_E: CGKeyCode = 0x0E;
pub const KEY_F: CGKeyCode = 0x03;
pub const KEY_G: CGKeyCode = 0x05;
pub const KEY_H: CGKeyCode = 0x04;
pub const KEY_I: CGKeyCode = 0x22;
pub const KEY_J: CGKeyCode = 0x26;
pub const KEY_K: CGKeyCode = 0x28;
pub const KEY_L: CGKeyCode = 0x25;
pub const KEY_M: CGKeyCode = 0x2E;
pub const KEY_N: CGKeyCode = 0x2D;
pub const KEY_O: CGKeyCode = 0x1F;
pub const KEY_P: CGKeyCode = 0x23;
pub const KEY_Q: CGKeyCode = 0x0C;
pub const KEY_R: CGKeyCode = 0x0F;
pub const KEY_S: CGKeyCode = 0x01;
pub const KEY_T: CGKeyCode = 0x11;
pub const KEY_U: CGKeyCode = 0x20;
pub const KEY_V: CGKeyCode = 0x09;
pub const KEY_W: CGKeyCode = 0x0D;
pub const KEY_X: CGKeyCode = 0x07;
pub const KEY_Y: CGKeyCode = 0x10;
pub const KEY_Z: CGKeyCode = 0x06;

pub mod ext_quartz {
    extern crate libc;
    use std::sync::mpsc::Sender;

    // Import types from super
    use super::KeyEvent;
    use super::Pointer;
    use super::CGEventRef;
    use super::CFMachPortRef;
    use super::CGEventField;
    use super::CGEventMask;
    use super::CGEventTapCallBack;
    use super::CGEventTapLocation;
    use super::CGEventTapOptions;
    use super::CGEventTapPlacement;
    use super::CGKeyCode;

    // Link to ApplicationServices/ApplicationServices.h and Carbon/Carbon.h
    #[link(name = "ApplicationServices", kind = "framework")]
    #[link(name = "Carbon", kind = "framework")]
    extern {

        /// Pass through to the default loop modes
        pub static kCFRunLoopCommonModes: Pointer;

        /// Pass through to the default allocator
        pub static kCFAllocatorDefault: Pointer;

        /// Run the current threads loop in default mode
        pub fn CFRunLoopRun();

        /// Obtain the current threads loop
        pub fn CFRunLoopGetCurrent() -> Pointer;

        /// Get the code of the event back, e.g. the key code
        pub fn CGEventGetIntegerValueField(
            event: CGEventRef,
            field: CGEventField,
        ) -> CGKeyCode;

        /// Create an event tap
        ///
        /// # Arguments
        ///
        /// * `place` - The location of the new event tap. Pass one of
        ///          the constants listed in Event Tap Locations. Only
        ///          processes running as the root user may locate an
        ///          event tap at the point where HID events enter the
        ///          window server; for other users, this function
        ///          returns NULL.
        ///
        /// * `options` - The placement of the new event tap in the
        ///          list of active event taps. Pass one of the
        ///          constants listed in Event Tap Placement.
        ///
        /// * `eventsOfInterest` - A constant that specifies whether
        ///          the new event tap is a passive listener or an
        ///          active filter.
        ///
        /// * `callback` - A bit mask that specifies the set of events
        ///          to be observed. For a list of possible events,
        ///          see Event Types. For information on how to
        ///          specify the mask, see CGEventMask. If the event
        ///          tap is not permitted to monitor one or more of
        ///          the events specified in the eventsOfInterest
        ///          parameter, then the appropriate bits in the mask
        ///          are cleared. If that action results in an empty
        ///          mask, this function returns NULL.  callback
        ///
        /// * `refcon` - An event tap callback function that you
        ///          provide. Your callback function is invoked from
        ///          the run loop to which the event tap is added as a
        ///          source. The thread safety of the callback is
        ///          defined by the run loop’s environment. To learn
        ///          more about event tap callbacks, see
        ///          CGEventTapCallBack.  refcon
        ///
        /// * `channel` - A pointer to user-defined data. This pointer
        ///          is passed into the callback function specified in
        ///          the callback parameter.  Here we use it as a mpsc
        ///          channel.
        pub fn CGEventTapCreate(
            tap: CGEventTapLocation,
            place: CGEventTapPlacement,
            options: CGEventTapOptions,
            eventsOfInterest: CGEventMask,
            callback: CGEventTapCallBack,
            channel: &Sender<KeyEvent>,
        ) -> CFMachPortRef;

        /// Creates a CFRunLoopSource object for a CFMachPort
        /// object.
        ///
        /// The run loop source is not automatically added to
        /// a run loop. To add the source to a run loop, use
        /// CFRunLoopAddSource
        pub fn CFMachPortCreateRunLoopSource(
            allocator: Pointer,
            port: CFMachPortRef,
            order: libc::c_int,
        ) -> Pointer;

        /// Adds a CFRunLoopSource object to a run loop mode.
        pub fn CFRunLoopAddSource(
            run_loop: Pointer,
            run_loop_source: Pointer,
            mode: Pointer,
        );

        pub fn CGEventTapEnable(port: CFMachPortRef, enable: bool);
    }
}

///  This callback will be registered to be invoked from the run loop
///  to which the event tap is added as a source.
#[no_mangle]
#[allow(unused_variables)]
pub extern fn callback(proxy: Pointer, etype: CGEventMask, event: CGEventRef, channel: &Sender<KeyEvent>) -> CGEventRef {
    unsafe {
        let source = ext_quartz::CGEventGetIntegerValueField(event, kCGEventSourceStateID);
        if source != 0 {
            let keyCode = ext_quartz::CGEventGetIntegerValueField(event, kCGKeyboardEventKeycode);
            let _ = channel.send(KeyEvent {
                etype: match etype as u32 {
                    kCGEventKeyDown => EventType::KeyDown,
                    kCGEventKeyUp => EventType::KeyUp,
                    kCGEventFlagsChanged => EventType::FlagsChanged,
                    _ => unreachable!(),
                },
                code: keyCode,
            });
        }
    }
    event
}

/// Redefine macro for bitshifting from header as function here
pub fn CGEventMaskBit(eventType: u32) -> CGEventMask {
    1 << (eventType)
}

/// Safe wrapper around CFRunLoopRun
pub fn CFRunLoopRun() {
    unsafe {
        ext_quartz::CFRunLoopRun();
    }
}

/// Registeres an event tap
pub fn register_event_tap(tx: &Sender<KeyEvent>) {
    let mask = CGEventMaskBit(kCGEventKeyDown)
        | CGEventMaskBit(kCGEventKeyUp)
        | CGEventMaskBit(kCGEventFlagsChanged);

    unsafe {
        let options = 0;

        // Create the event tap
        let event_tap = ext_quartz::CGEventTapCreate(
            kCGSessionEventTap,
            kCGHeadInsertEventTap,
            options,
            mask,
            callback,
            tx,
        );
        assert!(!event_tap.is_null(),
                "Unable to create event tap. Please make sure you have the correct permissions");
        println!("Created event tap...");

        let allocator = ext_quartz::kCFAllocatorDefault;
        let current_event_loop = ext_quartz::CFRunLoopGetCurrent();
        let mode = ext_quartz::kCFRunLoopCommonModes;

        // Create Run Loop Source
        let run_loop_source = ext_quartz::CFMachPortCreateRunLoopSource(allocator, event_tap, 0);

        // Add Run Loop Source to the current event loop
        ext_quartz::CFRunLoopAddSource(current_event_loop, run_loop_source, mode);

        // Enable the tap
        ext_quartz::CGEventTapEnable(event_tap, true);

    }
}

// Event Source
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub enum CGEventSourceStateID {
    Private = -1,
    CombinedSessionState = 0,
    HIDSystemState = 1,
}

pub enum CGEventSource {}
pub type CGEventSourceRef = *mut CGEventSource;

#[link(name = "CoreGraphics", kind = "framework")]
extern {
    pub fn CGEventSourceCreate(stateID: CGEventSourceStateID) -> CGEventSourceRef;
    pub fn CGEventCreate(source: CGEventSourceRef) -> CGEventRef;
    pub fn CGEventCreateKeyboardEvent(source: *const Pointer, keycode: CGKeyCode, keydown: bool) -> CGEventRef;
    pub fn CGEventPost(tapLocation: CGEventTapLocation, event: CGEventRef);
    pub fn CGEventKeyboardSetUnicodeString(event: CGEventRef, length: libc::c_ulong, string: *const u16);
}

pub fn create_event_source(state_id: CGEventSourceStateID) -> Result<CGEventSourceRef, ()> {
    unsafe {
        let event_source_ref = CGEventSourceCreate(state_id);
        if !event_source_ref.is_null() {
            Ok(event_source_ref)
        } else {
            Err(())
        }
    }
}

pub fn send_keyboard_event(keycode: CGKeyCode, keydown: bool) {
    unsafe {
        let event_ref = CGEventCreateKeyboardEvent(std::ptr::null(), keycode, keydown);
        if !event_ref.is_null() {
            CGEventPost(kCGHIDEventTap, event_ref);
        }
    }
}

pub fn send_backspace() {
    _ = send_keyboard_event(0x33, true);
    _ = send_keyboard_event(0x33, false);
}

pub fn send_string(content: &str) {
    unsafe {
        let event_ref = CGEventCreateKeyboardEvent(std::ptr::null(), 0, true);
        if !event_ref.is_null() {
            let buf: Vec<u16> = content.encode_utf16().collect();
            let buflen = buf.len() as libc::c_ulong;
            CGEventKeyboardSetUnicodeString(event_ref, buflen, buf.as_ptr());
            CGEventPost(kCGHIDEventTap, event_ref);
        }
    }
}
