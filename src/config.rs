use crate::hotkey::Hotkey;
use once_cell::sync::Lazy;

pub static HOTKEY_CONFIG: Lazy<Hotkey> = Lazy::new(|| Hotkey::from("super+ctrl+space"));
