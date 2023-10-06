mod config;
mod hotkey;
mod input;
mod platform;
mod ui;

use std::thread;

use druid::{AppLauncher, ExtEventSink, Target, WindowDesc};
use input::{rebuild_keyboard_layout_map, INPUT_STATE};
use log::debug;
use once_cell::sync::OnceCell;
use platform::{
    ensure_accessibility_permission, run_event_listener, send_backspace, send_string, Handle,
    KeyModifier, PressedKey, KEY_DELETE, KEY_ENTER, KEY_ESCAPE, KEY_SPACE, KEY_TAB, RAW_KEY_GLOBE,
};

use ui::{UIDataAdapter, UPDATE_UI};

static UI_EVENT_SINK: OnceCell<ExtEventSink> = OnceCell::new();

fn do_transform_keys(handle: Handle, is_delete: bool) -> bool {
    unsafe {
        if let Ok(output) = INPUT_STATE.transform_keys() {
            debug!("Transformed: {:?}", output);
            if INPUT_STATE.should_send_keyboard_event(&output) || is_delete {
                // This is a workaround for Firefox, where macOS's Accessibility API cannot work.
                // We cannot get the selected text in the address bar, so we will go with another
                // hacky way: Always send a space and delete it immediately. This will dismiss the
                // current pre-selected URL and fix the double character issue.
                if INPUT_STATE.should_dismiss_selection_if_needed() {
                    _ = send_string(handle, " ");
                    _ = send_backspace(handle, 1);
                }

                let backspace_count = INPUT_STATE.get_backspace_count(is_delete);
                debug!("Backspace count: {}", backspace_count);
                _ = send_backspace(handle, backspace_count);
                _ = send_string(handle, &output);
                debug!("Sent: {:?}", output);
                INPUT_STATE.replace(output);
                return true;
            }
        }
    }
    false
}

fn do_restore_word(handle: Handle) {
    unsafe {
        let backspace_count = INPUT_STATE.get_backspace_count(true);
        debug!("Backspace count: {}", backspace_count);
        _ = send_backspace(handle, backspace_count);
        let typing_buffer = INPUT_STATE.get_typing_buffer();
        _ = send_string(handle, typing_buffer);
        debug!("Sent: {:?}", typing_buffer);
        INPUT_STATE.replace(typing_buffer.to_owned());
    }
}

unsafe fn toggle_vietnamese() {
    INPUT_STATE.toggle_vietnamese();
    if let Some(event_sink) = UI_EVENT_SINK.get() {
        _ = event_sink.submit_command(UPDATE_UI, (), Target::Auto);
    }
}

unsafe fn auto_toggle_vietnamese() {
    INPUT_STATE.update_active_app();
    if let Some(event_sink) = UI_EVENT_SINK.get() {
        _ = event_sink.submit_command(UPDATE_UI, (), Target::Auto);
    }
}

fn event_handler(handle: Handle, pressed_key: Option<PressedKey>, modifiers: KeyModifier) -> bool {
    unsafe {
        auto_toggle_vietnamese();
        let pressed_key_code = pressed_key.and_then(|p| match p {
            PressedKey::Char(c) => Some(c),
            _ => None
        });
        let is_hotkey_pressed =
            INPUT_STATE.get_hotkey().is_match(modifiers, pressed_key_code);
        if is_hotkey_pressed {
            toggle_vietnamese();
            return true;
        }
        match pressed_key {
            Some(pressed_key) => {
                match pressed_key {
                    PressedKey::Raw(raw_keycode) => {
                        if raw_keycode == RAW_KEY_GLOBE {
                            toggle_vietnamese();
                            return true;
                        }
                    }
                    PressedKey::Char(keycode) => {
                        if INPUT_STATE.is_enabled() {
                            match keycode {
                                KEY_ENTER | KEY_TAB | KEY_SPACE | KEY_ESCAPE => {
                                    let is_valid_word = vi::validation::is_valid_word(
                                        INPUT_STATE.get_displaying_word(),
                                    );
                                    let is_transformed_word = !INPUT_STATE
                                        .get_typing_buffer()
                                        .eq(INPUT_STATE.get_displaying_word());
                                    if is_transformed_word && !is_valid_word {
                                        do_restore_word(handle);
                                    }

                                    if INPUT_STATE.previous_word_is_stop_tracking_words() {
                                        INPUT_STATE.clear_previous_word();
                                    }
                                    INPUT_STATE.new_word();
                                }
                                KEY_DELETE => {
                                    INPUT_STATE.pop();
                                }
                                c => {
                                    if "()[]{}<>/\\!@#$%^&*-_=+|~`,.;'\"".contains(c)
                                        || (c.is_numeric() && modifiers.is_shift())
                                    {
                                        // If special characters detected, dismiss the current tracking word
                                        INPUT_STATE.push(c);
                                        INPUT_STATE.new_word();
                                    } else {
                                        // Otherwise, process the character
                                        if modifiers.is_super()
                                            || modifiers.is_control()
                                            || modifiers.is_alt()
                                        {
                                            INPUT_STATE.new_word();
                                        } else if INPUT_STATE.is_tracking() {
                                            INPUT_STATE.push(
                                                if modifiers.is_shift() || modifiers.is_capslock() {
                                                    c.to_ascii_uppercase()
                                                } else {
                                                    c
                                                },
                                            );
                                            let ret = do_transform_keys(handle, false);
                                            INPUT_STATE.stop_tracking_if_needed();
                                            return ret;
                                        }
                                    }
                                }
                            }
                        }
                    }
                };
            }
            None => {
                if !modifiers.is_shift() {
                    INPUT_STATE.new_word();
                }
            }
        }
    }
    false
}

fn main() {
    env_logger::init();
    if !ensure_accessibility_permission() {
        // Show the Accessibility Permission Request screen
        let win = WindowDesc::new(ui::permission_request_ui_builder())
            .title("gõkey")
            .window_size((500.0, 360.0))
            .resizable(false);
        let app = AppLauncher::with_window(win);
        _ = app.launch(());
    } else {
        // Start the GõKey application
        rebuild_keyboard_layout_map();
        let win = WindowDesc::new(ui::main_ui_builder())
            .title("gõkey")
            .window_size((ui::WINDOW_WIDTH, ui::WINDOW_HEIGHT))
            .set_position(ui::center_window_position())
            .set_always_on_top(true)
            .resizable(false);
        let app = AppLauncher::with_window(win);
        let event_sink = app.get_external_handle();
        _ = UI_EVENT_SINK.set(event_sink);
        thread::spawn(|| {
            run_event_listener(&event_handler);
        });
        _ = app.launch(UIDataAdapter::new());
    }
}
