mod colors;
mod controllers;
mod data;
pub(crate) mod locale;
mod selectors;
mod views;
mod widgets;

use druid::Selector;

pub use colors::{get_theme, THEME};
pub use data::UIDataAdapter;
pub use views::{
    add_macro_dialog_ui_builder, center_window_position, edit_shortcut_dialog_ui_builder,
    main_ui_builder, permission_request_ui_builder, ADD_MACRO_DIALOG_HEIGHT,
    ADD_MACRO_DIALOG_WIDTH, EDIT_SHORTCUT_DIALOG_HEIGHT, EDIT_SHORTCUT_DIALOG_WIDTH,
};

pub const UPDATE_UI: Selector = Selector::new("gox-ui.update-ui");
pub const SHOW_UI: Selector = Selector::new("gox-ui.show-ui");
pub const WINDOW_WIDTH: f64 = 480.0;
pub const WINDOW_HEIGHT: f64 = 680.0;

pub fn format_letter_key(c: Option<char>) -> String {
    if let Some(c) = c {
        return if c.is_ascii_whitespace() {
            String::from("Space")
        } else {
            c.to_ascii_uppercase().to_string()
        };
    }
    String::new()
}

pub fn letter_key_to_char(input: &str) -> Option<char> {
    match input {
        "Space" => Some(' '),
        s => {
            if input.len() > 1 {
                None
            } else {
                s.chars().last()
            }
        }
    }
}
