// TODO: Implement this

use druid::{Selector, commands::CLOSE_WINDOW};

use super::CallbackFn;

pub const SYMBOL_SHIFT: &str = "⇧";
pub const SYMBOL_CTRL: &str = "⌃";
pub const SYMBOL_SUPER: &str = "⊞";
pub const SYMBOL_ALT: &str = "⌥";

pub fn get_home_dir() -> Option<PathBuf> {
    env::var("USERPROFILE").ok().map(PathBuf::from)
        .or_else(|| env::var("HOMEDRIVE").ok().and_then(|home_drive| {
            env::var("HOMEPATH").ok().map(|home_path| {
                PathBuf::from(format!("{}{}", home_drive, home_path))
            })
        }))
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

pub fn is_in_text_selection() -> bool {
    todo!()
}

pub fn update_launch_on_login(is_enable: bool) {
    todo!()
}

pub fn is_launch_on_login() {
    todo!()
}
