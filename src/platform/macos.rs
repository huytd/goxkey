use std::env::current_exe;
use std::path::Path;
use std::{env, path::PathBuf, ptr};

mod macos_ext;
use auto_launch::{AutoLaunch, AutoLaunchBuilder};
use cocoa::base::id;
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
use objc::{class, msg_send, sel, sel_impl};

pub use macos_ext::SystemTray;
pub use macos_ext::SystemTrayMenuItemKey;
use once_cell::sync::Lazy;

use crate::input::KEYBOARD_LAYOUT_CHARACTER_MAP;
use accessibility::{AXAttribute, AXUIElement};
use accessibility_sys::{kAXFocusedUIElementAttribute, kAXSelectedTextAttribute};
use core_foundation::{
    runloop::{kCFRunLoopCommonModes, CFRunLoop},
    string::CFString,
};

pub use self::macos_ext::Handle;
use self::macos_ext::{
    kAXTrustedCheckOptionPrompt, new_tap, AXIsProcessTrustedWithOptions,
    CGEventCreateKeyboardEvent, CGEventKeyboardSetUnicodeString, CGEventTapPostEvent,
};

use super::{
    CallbackFn, EventTapType, KeyModifier, PressedKey, KEY_DELETE, KEY_ENTER, KEY_ESCAPE,
    KEY_SPACE, KEY_TAB,
};

pub const SYMBOL_SHIFT: &str = "⇧";
pub const SYMBOL_CTRL: &str = "⌃";
pub const SYMBOL_SUPER: &str = "⌘";
pub const SYMBOL_ALT: &str = "⌥";

impl From<CGEventType> for EventTapType {
    fn from(value: CGEventType) -> Self {
        match value {
            CGEventType::KeyDown => EventTapType::KeyDown,
            CGEventType::FlagsChanged => EventTapType::FlagsChanged,
            _ => EventTapType::Other,
        }
    }
}

static AUTO_LAUNCH: Lazy<AutoLaunch> = Lazy::new(|| {
    let app_path = get_current_app_path();
    let app_name = Path::new(&app_path)
        .file_stem()
        .and_then(|f| f.to_str())
        .unwrap();
    AutoLaunchBuilder::new()
        .set_app_name(app_name)
        .set_app_path(&app_path)
        .build()
        .unwrap()
});

/// On macOS, current_exe gives path to /Applications/Example.app/MacOS/Example but this results in seeing a Unix Executable in macOS login items. It must be: /Applications/Example.app
/// If it didn't find exactly a single occurrence of .app, it will default to exe path to not break it.
fn get_current_app_path() -> String {
    let current_exe = current_exe().unwrap();
    let exe_path = current_exe.canonicalize().unwrap().display().to_string();
    let parts: Vec<&str> = exe_path.split(".app/").collect();
    return if parts.len() == 2 {
        format!("{}.app", parts.get(0).unwrap().to_string())
    } else {
        exe_path
    };
}

#[macro_export]
macro_rules! nsstring_to_string {
    ($ns_string:expr) => {{
        use objc::{sel, sel_impl};
        let utf8: id = objc::msg_send![$ns_string, UTF8String];
        let string = if !utf8.is_null() {
            Some({
                std::ffi::CStr::from_ptr(utf8 as *const std::ffi::c_char)
                    .to_string_lossy()
                    .into_owned()
            })
        } else {
            None
        };
        string
    }};
}

pub fn get_home_dir() -> Option<PathBuf> {
    env::var("HOME").ok().map(PathBuf::from)
}

// List of keycode: https://eastmanreference.com/complete-list-of-applescript-key-codes
fn get_char(keycode: CGKeyCode) -> Option<PressedKey> {
    if let Some(key_map) = unsafe { KEYBOARD_LAYOUT_CHARACTER_MAP.get() } {
        return match keycode {
            0 => Some(PressedKey::Char(key_map[&'a'])),
            1 => Some(PressedKey::Char(key_map[&'s'])),
            2 => Some(PressedKey::Char(key_map[&'d'])),
            3 => Some(PressedKey::Char(key_map[&'f'])),
            4 => Some(PressedKey::Char(key_map[&'h'])),
            5 => Some(PressedKey::Char(key_map[&'g'])),
            6 => Some(PressedKey::Char(key_map[&'z'])),
            7 => Some(PressedKey::Char(key_map[&'x'])),
            8 => Some(PressedKey::Char(key_map[&'c'])),
            9 => Some(PressedKey::Char(key_map[&'v'])),
            11 => Some(PressedKey::Char(key_map[&'b'])),
            12 => Some(PressedKey::Char(key_map[&'q'])),
            13 => Some(PressedKey::Char(key_map[&'w'])),
            14 => Some(PressedKey::Char(key_map[&'e'])),
            15 => Some(PressedKey::Char(key_map[&'r'])),
            16 => Some(PressedKey::Char(key_map[&'y'])),
            17 => Some(PressedKey::Char(key_map[&'t'])),
            31 => Some(PressedKey::Char(key_map[&'o'])),
            32 => Some(PressedKey::Char(key_map[&'u'])),
            34 => Some(PressedKey::Char(key_map[&'i'])),
            35 => Some(PressedKey::Char(key_map[&'p'])),
            37 => Some(PressedKey::Char(key_map[&'l'])),
            38 => Some(PressedKey::Char(key_map[&'j'])),
            40 => Some(PressedKey::Char(key_map[&'k'])),
            45 => Some(PressedKey::Char(key_map[&'n'])),
            46 => Some(PressedKey::Char(key_map[&'m'])),
            18 => Some(PressedKey::Char(key_map[&'1'])),
            19 => Some(PressedKey::Char(key_map[&'2'])),
            20 => Some(PressedKey::Char(key_map[&'3'])),
            21 => Some(PressedKey::Char(key_map[&'4'])),
            22 => Some(PressedKey::Char(key_map[&'6'])),
            23 => Some(PressedKey::Char(key_map[&'5'])),
            25 => Some(PressedKey::Char(key_map[&'9'])),
            26 => Some(PressedKey::Char(key_map[&'7'])),
            28 => Some(PressedKey::Char(key_map[&'8'])),
            29 => Some(PressedKey::Char(key_map[&'0'])),
            27 => Some(PressedKey::Char(key_map[&'-'])),
            33 => Some(PressedKey::Char(key_map[&'['])),
            30 => Some(PressedKey::Char(key_map[&']'])),
            41 => Some(PressedKey::Char(key_map[&';'])),
            43 => Some(PressedKey::Char(key_map[&','])),
            24 => Some(PressedKey::Char(key_map[&'='])),
            42 => Some(PressedKey::Char(key_map[&'\\'])),
            44 => Some(PressedKey::Char(key_map[&'/'])),
            39 => Some(PressedKey::Char(key_map[&'\''])),
            47 => Some(PressedKey::Char(key_map[&'.'])),
            36 | 52 => Some(PressedKey::Char(KEY_ENTER)), // ENTER
            49 => Some(PressedKey::Char(KEY_SPACE)),      // SPACE
            48 => Some(PressedKey::Char(KEY_TAB)),        // TAB
            51 => Some(PressedKey::Char(KEY_DELETE)),     // DELETE
            53 => Some(PressedKey::Char(KEY_ESCAPE)),     // ESC
            _ => Some(PressedKey::Raw(keycode)),
        };
    }
    None
}

pub fn is_in_text_selection() -> bool {
    let system_element = AXUIElement::system_wide();
    let Some(selected_element) = system_element
        .attribute(&AXAttribute::new(&CFString::from_static_string(
            kAXFocusedUIElementAttribute,
        )))
        .map(|elemenet| elemenet.downcast_into::<AXUIElement>())
        .ok()
        .flatten()
    else {
        return false;
    };
    let Some(selected_text) = selected_element
        .attribute(&AXAttribute::new(&CFString::from_static_string(
            kAXSelectedTextAttribute,
        )))
        .map(|text| text.downcast_into::<CFString>())
        .ok()
        .flatten()
    else {
        return false;
    };
    !selected_text.to_string().is_empty()
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

pub fn add_app_change_callback<F>(cb: F)
where
    F: Fn() + Send + 'static,
{
    macos_ext::add_app_change_callback(cb);
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
            CGEventType::FlagsChanged,
        ],
        |proxy, _, event| {
            if !is_process_trusted() {
                eprintln!("Accessibility access removed!");
                std::process::exit(1);
            }

            let mut modifiers = KeyModifier::new();
            let flags = event.get_flags();
            if flags.contains(CGEventFlags::CGEventFlagShift) {
                modifiers.add_shift();
            }
            if flags.contains(CGEventFlags::CGEventFlagAlphaShift) {
                modifiers.add_capslock();
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
            if flags.eq(&CGEventFlags::CGEventFlagNonCoalesced)
                || flags.eq(&CGEventFlags::CGEventFlagNull)
            {
                modifiers = KeyModifier::MODIFIER_NONE;
            }

            let event_tap_type: EventTapType = EventTapType::from(event.get_type());
            match event_tap_type {
                EventTapType::KeyDown => {
                    let source_state_id =
                        event.get_integer_value_field(EventField::EVENT_SOURCE_STATE_ID);
                    if source_state_id == 1 {
                        let key_code = event
                            .get_integer_value_field(EventField::KEYBOARD_EVENT_KEYCODE)
                            as CGKeyCode;

                        if callback(proxy, event_tap_type, get_char(key_code), modifiers) {
                            // block the key if already processed
                            return None;
                        }
                    }
                }
                EventTapType::FlagsChanged => {
                    callback(proxy, event_tap_type, None, modifiers);
                }
                _ => {
                    callback(proxy, event_tap_type, None, KeyModifier::new());
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

pub fn is_process_trusted() -> bool {
    unsafe { accessibility_sys::AXIsProcessTrusted() }
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

pub fn get_active_app_name() -> String {
    unsafe {
        let shared_workspace: id = msg_send![class!(NSWorkspace), sharedWorkspace];
        let front_most_app: id = msg_send![shared_workspace, frontmostApplication];
        let bundle_url: id = msg_send![front_most_app, bundleURL];
        let path: id = msg_send![bundle_url, path];
        nsstring_to_string!(path).unwrap_or("/Unknown.app".to_string())
    }
}

pub fn update_launch_on_login(is_enable: bool) -> Result<(), auto_launch::Error> {
    match is_enable {
        true => AUTO_LAUNCH.enable(),
        false => AUTO_LAUNCH.disable(),
    }
}

pub fn is_launch_on_login() -> bool {
    AUTO_LAUNCH.is_enabled().unwrap()
}
