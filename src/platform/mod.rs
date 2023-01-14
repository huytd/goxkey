#[cfg_attr(target_os = "macos", path = "macos.rs")]
#[cfg_attr(target_os = "linux", path = "linux.rs")]
#[cfg_attr(target_os = "window", path = "window.rs")]
mod os;

pub const KEY_ENTER: char = '\x13';
pub const KEY_SPACE: char = '\x32';
pub const KEY_TAB: char = '\x09';
pub const KEY_DELETE: char = '\x08';
pub const KEY_ESCAPE: char = '\x26';

pub type CallbackFn = dyn Fn(char, bool) -> bool;

pub use os::{run_event_listener, send_backspace, send_string};
