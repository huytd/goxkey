#[cfg_attr(target_os = "macos", path = "macos.rs")]
#[cfg_attr(target_os = "linux", path = "linux.rs")]
#[cfg_attr(target_os = "windows", path = "window.rs")]
mod os;

pub use os::{
    add_app_change_callback, add_appearance_change_callback, defer_open_app_file_picker,
    defer_open_text_file_picker,
    defer_save_text_file_picker, dispatch_set_systray_title, ensure_accessibility_permission,
    get_active_app_name, get_app_icon_rgba, get_home_dir, get_preferred_language, is_dark_mode,
    is_in_text_selection, is_launch_on_login, rebuild_keyboard_layout_map, run_event_listener,
    send_arrow_left, send_arrow_right, send_backspace, send_string, update_launch_on_login, Handle,
};

#[cfg(target_os = "macos")]
pub use os::SystemTray;
pub use os::SystemTrayMenuItemKey;

pub use goxkey_core::{KeyModifier, SYMBOL_ALT, SYMBOL_CTRL, SYMBOL_SHIFT, SYMBOL_SUPER};
pub use goxkey_core::{KEY_DELETE, KEY_ENTER, KEY_ESCAPE, KEY_SPACE, KEY_TAB};

pub const RAW_KEY_GLOBE: u16 = 0xb3;
pub const RAW_ARROW_DOWN: u16 = 0x7d;
pub const RAW_ARROW_UP: u16 = 0x7e;
pub const RAW_ARROW_LEFT: u16 = 0x7b;
pub const RAW_ARROW_RIGHT: u16 = 0x7c;

#[derive(Debug, Copy, Clone)]
pub enum PressedKey {
    Char(char),
    Raw(u16),
}

#[derive(Debug, PartialEq, Eq)]
pub enum EventTapType {
    KeyDown,
    FlagsChanged,
    Other,
}

pub type CallbackFn = dyn Fn(os::Handle, EventTapType, Option<PressedKey>, KeyModifier) -> bool;
