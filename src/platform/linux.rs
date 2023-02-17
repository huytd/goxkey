// TODO: Implement this

use super::CallbackFn;

pub const SYMBOL_SHIFT: &str = "⇧";
pub const SYMBOL_CTRL: &str = "⌃";
pub const SYMBOL_SUPER: &str = "❖";
pub const SYMBOL_ALT: &str = "⌥";

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
