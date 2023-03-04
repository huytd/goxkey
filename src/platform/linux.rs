// TODO: Implement this

use druid::{commands::CLOSE_WINDOW, Selector};

use super::CallbackFn;

pub const SYMBOL_SHIFT: &str = "⇧";
pub const SYMBOL_CTRL: &str = "⌃";
pub const SYMBOL_SUPER: &str = "❖";
pub const SYMBOL_ALT: &str = "⌥";

// TODO: support HIDE_WINDOW on next druid future versions
pub const HIDE_COMMAND: Selector = CLOSE_WINDOW;

pub fn get_home_dir() -> Option<PathBuf> {
    env::var("HOME").ok().map(PathBuf::from)
}

pub fn send_backspace(count: usize) -> Result<(), ()> {
    todo!()
}

pub fn send_string(string: &str) -> Result<(), ()> {
    todo!()
}

pub fn run_event_listener(callback: &CallbackFn) {
    todo!()
}

pub fn ensure_accessibility_permission() -> bool {
    true
}
