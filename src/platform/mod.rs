#[cfg_attr(target_os = "macos", path = "macos.rs")]
#[cfg_attr(target_os = "linux", path = "linux.rs")]
#[cfg_attr(target_os = "window", path = "window.rs")]
mod os;

use bitflags::bitflags;

pub const KEY_ENTER: char = '\x13';
pub const KEY_SPACE: char = '\x32';
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
    }
}

impl KeyModifier {
    pub fn new() -> Self {
        Self { bits: 0 }
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
}

pub type CallbackFn = dyn Fn(os::Handle, char, KeyModifier) -> bool;

pub use os::{run_event_listener, send_backspace, send_string, Handle};
