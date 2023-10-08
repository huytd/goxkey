#[cfg_attr(target_os = "macos", path = "macos.rs")]
#[cfg_attr(target_os = "linux", path = "linux.rs")]
#[cfg_attr(target_os = "window", path = "window.rs")]
mod os;

use std::fmt::Display;

use bitflags::bitflags;
pub use os::{
    add_app_change_callback, ensure_accessibility_permission, get_active_app_name, get_home_dir,
    is_in_text_selection, is_launch_on_login, run_event_listener, send_backspace, send_string,
    update_launch_on_login, Handle, SYMBOL_ALT, SYMBOL_CTRL, SYMBOL_SHIFT, SYMBOL_SUPER,
};

#[cfg(target_os = "macos")]
pub use os::SystemTray;
pub use os::SystemTrayMenuItemKey;

pub const RAW_KEY_GLOBE: u16 = 0xb3;
pub const KEY_ENTER: char = '\x13';
pub const KEY_SPACE: char = '\u{0020}';
pub const KEY_TAB: char = '\x09';
pub const KEY_DELETE: char = '\x08';
pub const KEY_ESCAPE: char = '\x26';

bitflags! {
    pub struct KeyModifier: u32 {
        const MODIFIER_NONE     = 0b00000000;
        const MODIFIER_SHIFT    = 0b00000001;
        const MODIFIER_SUPER    = 0b00000010;
        const MODIFIER_CONTROL  = 0b00000100;
        const MODIFIER_ALT      = 0b00001000;
        const MODIFIER_CAPSLOCK = 0b00010000;
    }
}

impl Display for KeyModifier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.is_super() {
            write!(f, "super+")?;
        }
        if self.is_control() {
            write!(f, "ctrl+")?;
        }
        if self.is_alt() {
            write!(f, "alt+")?;
        }
        if self.is_shift() {
            write!(f, "shift+")?;
        }
        if self.is_capslock() {
            write!(f, "capslock+")?;
        }
        write!(f, "")
    }
}

impl KeyModifier {
    pub fn new() -> Self {
        Self { bits: 0 }
    }

    pub fn apply(
        &mut self,
        is_super: bool,
        is_ctrl: bool,
        is_alt: bool,
        is_shift: bool,
        is_capslock: bool,
    ) {
        self.set(Self::MODIFIER_SUPER, is_super);
        self.set(Self::MODIFIER_CONTROL, is_ctrl);
        self.set(Self::MODIFIER_ALT, is_alt);
        self.set(Self::MODIFIER_SHIFT, is_shift);
        self.set(Self::MODIFIER_CAPSLOCK, is_capslock);
    }

    pub fn add_shift(&mut self) {
        self.set(Self::MODIFIER_SHIFT, true);
    }

    pub fn add_super(&mut self) {
        self.set(Self::MODIFIER_SUPER, true);
    }

    pub fn add_control(&mut self) {
        self.set(Self::MODIFIER_CONTROL, true);
    }

    pub fn add_alt(&mut self) {
        self.set(Self::MODIFIER_ALT, true);
    }

    pub fn add_capslock(&mut self) {
        self.set(Self::MODIFIER_CAPSLOCK, true);
    }

    pub fn is_shift(&self) -> bool {
        self.contains(Self::MODIFIER_SHIFT)
    }

    pub fn is_super(&self) -> bool {
        self.contains(Self::MODIFIER_SUPER)
    }

    pub fn is_control(&self) -> bool {
        self.contains(Self::MODIFIER_CONTROL)
    }

    pub fn is_alt(&self) -> bool {
        self.contains(Self::MODIFIER_ALT)
    }

    pub fn is_capslock(&self) -> bool {
        self.contains(Self::MODIFIER_CAPSLOCK)
    }
}

#[derive(Debug, Copy, Clone)]
pub enum PressedKey {
    Char(char),
    Raw(u16),
}
pub type CallbackFn = dyn Fn(os::Handle, Option<PressedKey>, KeyModifier) -> bool;
