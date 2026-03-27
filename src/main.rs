mod config;
mod hotkey;
mod input;
mod platform;
mod scripting;
mod ui;

use std::thread;

use druid::{AppLauncher, ExtEventSink, Target, WindowDesc};
use input::{
    get_diff_parts, rebuild_keyboard_layout_map, TypingMethod, HOTKEY_MATCHING_CIRCUIT_BREAK,
    INPUT_STATE,
};
use log::debug;
use once_cell::sync::OnceCell;
use platform::{
    add_app_change_callback, dispatch_set_systray_title, ensure_accessibility_permission,
    run_event_listener, send_arrow_left, send_arrow_right, send_backspace, send_string,
    EventTapType, Handle, KeyModifier, PressedKey, KEY_DELETE, KEY_ENTER, KEY_ESCAPE, KEY_SPACE,
    KEY_TAB, RAW_KEY_GLOBE,
};

use crate::{
    input::{HOTKEY_MATCHING, HOTKEY_MODIFIERS},
    platform::{RAW_ARROW_DOWN, RAW_ARROW_LEFT, RAW_ARROW_RIGHT, RAW_ARROW_UP},
};
use ui::{UIDataAdapter, UPDATE_UI};

static UI_EVENT_SINK: OnceCell<ExtEventSink> = OnceCell::new();
const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

fn apply_capslock_to_output(output: String, is_capslock: bool) -> String {
    if is_capslock {
        output.to_uppercase()
    } else {
        output
    }
}

fn normalize_input_char(c: char, is_shift: bool) -> char {
    if is_shift {
        c.to_ascii_uppercase()
    } else {
        c
    }
}

fn do_transform_keys(handle: Handle, is_delete: bool, is_capslock: bool) -> bool {
    unsafe {
        if let Ok((raw_output, transform_result)) = INPUT_STATE.transform_keys() {
            let should_send_event = INPUT_STATE.should_send_keyboard_event(&raw_output);
            let output = apply_capslock_to_output(raw_output, is_capslock);
            debug!("Transformed: {:?}", output);
            if should_send_event || is_delete {
                // This is a workaround for Firefox-based browsers, where macOS's Accessibility API cannot work.
                // We cannot get the selected text in the address bar, so we will go with another
                // hacky way: Always send a space and delete it immediately. This will dismiss the
                // current pre-selected URL and fix the double character issue.
                if INPUT_STATE.should_dismiss_selection_if_needed() {
                    _ = send_string(handle, " ");
                    _ = send_backspace(handle, 1);
                }

                // Compute the minimal diff between what is currently displayed
                // and the new output.  Only delete and retype the diverging
                // suffix — the common prefix stays on screen untouched, which
                // eliminates flicker in Chromium/Electron apps (e.g. Messenger)
                // caused by a VSync frame landing between the backspace burst
                // and the reinsertion of the full word.
                //
                // Exception: when `is_delete` is true the caller wants the
                // entire word erased (e.g. the user pressed Delete/Backspace),
                // so we fall back to full-replace in that case.
                let (backspace_count, suffix_offset, screen_char_count) = if is_delete {
                    let bs = INPUT_STATE.get_backspace_count(is_delete);
                    (bs, 0usize, bs)
                } else {
                    // Clone the display buffer so we hold no borrow into INPUT_STATE
                    // while calling get_diff_parts, which borrows `output`.
                    let displaying = INPUT_STATE.get_displaying_word().to_owned();
                    // `push(c)` was called just before this function, appending the
                    // typed char to display_buffer.  That char has NOT yet appeared on
                    // screen because we are about to block the key event and replace it
                    // ourselves.  Strip it so `old` reflects the true on-screen state.
                    let screen_end = displaying
                        .char_indices()
                        .next_back()
                        .map(|(i, _)| i)
                        .unwrap_or(displaying.len());
                    let screen = &displaying[..screen_end];
                    let sc = screen.chars().count();
                    let (bs, sfx) = get_diff_parts(screen, &output);
                    let offset = output.len() - sfx.len();
                    (bs, offset, sc)
                };
                let suffix = &output[suffix_offset..];
                debug!("Backspace count: {}", backspace_count);

                // When the entire on-screen word would be erased (no common
                // prefix), Chromium/Electron apps fire an "empty value" event
                // that swallows subsequent keystrokes.  Avoid this by keeping
                // one sentinel char on screen: type the new text first, then
                // navigate back to delete the sentinel.
                let needs_sentinel = !is_delete
                    && backspace_count > 0
                    && backspace_count == screen_char_count;

                if needs_sentinel {
                    // Keep one old char as a sentinel so the field never
                    // empties (Chromium/Electron kill pending events on
                    // empty).  Build the new word left-to-right:
                    let first_char_end = suffix
                        .char_indices()
                        .nth(1)
                        .map(|(i, _)| i)
                        .unwrap_or(suffix.len());
                    let first_char = &suffix[..first_char_end];
                    let rest = &suffix[first_char_end..];
                    // 1. Delete all old chars except the last (sentinel)
                    _ = send_backspace(handle, backspace_count - 1);
                    // 2. Type the first char of the new output
                    _ = send_string(handle, first_char);
                    // 3. Move left behind the first char (before sentinel)
                    _ = send_arrow_left(handle, 1);
                    // 4. Delete the sentinel
                    _ = send_backspace(handle, 1);
                    // 5. Move right past the first char
                    _ = send_arrow_right(handle, 1);
                    // 6. Type the rest of the output
                    if !rest.is_empty() {
                        _ = send_string(handle, rest);
                    }
                } else {
                    _ = send_backspace(handle, backspace_count);
                    if !suffix.is_empty() {
                        _ = send_string(handle, suffix);
                    }
                }
                debug!("Sent suffix: {:?}", suffix);
                INPUT_STATE.replace(output);
                if transform_result.letter_modification_removed
                    || transform_result.tone_mark_removed
                {
                    INPUT_STATE.stop_tracking();
                }
                return true;
            }
        }
    }
    false
}

fn do_restore_word(handle: Handle, is_capslock: bool) {
    unsafe {
        let backspace_count = INPUT_STATE.get_backspace_count(true);
        debug!("Backspace count: {}", backspace_count);
        _ = send_backspace(handle, backspace_count);
        let typing_buffer = INPUT_STATE.get_typing_buffer();
        let output = apply_capslock_to_output(typing_buffer.to_owned(), is_capslock);
        _ = send_string(handle, &output);
        debug!("Sent: {:?}", output);
        INPUT_STATE.replace(output);
    }
}

fn should_restore_transformed_word(
    method: TypingMethod,
    typing_buffer: &str,
    display_buffer: &str,
    is_valid_word: bool,
    is_allowed_word: bool,
) -> bool {
    let is_transformed_word = typing_buffer != display_buffer;
    if !is_transformed_word || is_valid_word || is_allowed_word {
        return false;
    }

    // Keep VNI shorthand words (like d9m -> đm) when ending a word with space/tab/enter.
    let is_vni_numeric_shortcut =
        method == TypingMethod::VNI && typing_buffer.chars().any(|c| c.is_numeric());
    !is_vni_numeric_shortcut
}

fn do_macro_replace(handle: Handle, target: &String) {
    unsafe {
        let backspace_count = INPUT_STATE.get_backspace_count(true);
        debug!("Backspace count: {}", backspace_count);
        _ = send_backspace(handle, backspace_count);
        _ = send_string(handle, target);
        debug!("Sent: {:?}", target);
        INPUT_STATE.replace(target.to_owned());
    }
}

/// Compute the tray title from the current INPUT_STATE and dispatch it
/// directly to the main queue, so the status bar updates instantly.
pub unsafe fn update_systray_title_immediately() {
    let is_enabled = INPUT_STATE.is_enabled();
    let is_gox = INPUT_STATE.is_gox_mode_enabled();
    let title = if is_enabled {
        if is_gox { "gõ" } else { "VN" }
    } else if is_gox {
        match INPUT_STATE.get_method() {
            TypingMethod::Telex => "gox",
            TypingMethod::VNI => "go4",
            TypingMethod::TelexVNI => "go+",
        }
    } else {
        "EN"
    };
    dispatch_set_systray_title(title);
}

unsafe fn toggle_vietnamese() {
    INPUT_STATE.toggle_vietnamese();
    update_systray_title_immediately();
    if let Some(event_sink) = UI_EVENT_SINK.get() {
        if let Err(e) = event_sink.submit_command(UPDATE_UI, (), Target::Auto) {
            debug!("Failed to submit UPDATE_UI command: {:?}", e);
        }
    }
}

unsafe fn auto_toggle_vietnamese() {
    if !INPUT_STATE.is_auto_toggle_enabled() {
        return;
    }
    let has_change = INPUT_STATE.update_active_app().is_some();
    if !has_change {
        return;
    }
    update_systray_title_immediately();
    if let Some(event_sink) = UI_EVENT_SINK.get() {
        if let Err(e) = event_sink.submit_command(UPDATE_UI, (), Target::Auto) {
            debug!("Failed to submit UPDATE_UI command: {:?}", e);
        }
    }
}

fn event_handler(
    handle: Handle,
    event_type: EventTapType,
    pressed_key: Option<PressedKey>,
    modifiers: KeyModifier,
) -> bool {
    unsafe {
        let pressed_key_code = pressed_key.and_then(|p| match p {
            PressedKey::Char(c) => Some(c),
            _ => None,
        });

        if event_type == EventTapType::FlagsChanged {
            if modifiers.is_empty() {
                // Modifier keys are released
                if HOTKEY_MATCHING && !HOTKEY_MATCHING_CIRCUIT_BREAK {
                    toggle_vietnamese();
                }
                HOTKEY_MODIFIERS = KeyModifier::MODIFIER_NONE;
                HOTKEY_MATCHING = false;
                HOTKEY_MATCHING_CIRCUIT_BREAK = false;
            } else {
                HOTKEY_MODIFIERS.set(modifiers, true);
            }
        }

        let is_hotkey_matched = INPUT_STATE
            .get_hotkey()
            .is_match(HOTKEY_MODIFIERS, pressed_key_code);
        if HOTKEY_MATCHING && !is_hotkey_matched {
            HOTKEY_MATCHING_CIRCUIT_BREAK = true;
        }
        HOTKEY_MATCHING = is_hotkey_matched;

        // If the hotkey matched on a key press, toggle immediately and
        // suppress the event so macOS does not insert the character
        // (e.g. Option+Z → Ω).  Set HOTKEY_MATCHING_CIRCUIT_BREAK so
        // the FlagsChanged handler does not toggle again on key release.
        if is_hotkey_matched && pressed_key_code.is_some() {
            toggle_vietnamese();
            HOTKEY_MATCHING_CIRCUIT_BREAK = true;
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
                        if raw_keycode == RAW_ARROW_UP || raw_keycode == RAW_ARROW_DOWN {
                            INPUT_STATE.new_word();
                        }
                        if raw_keycode == RAW_ARROW_LEFT || raw_keycode == RAW_ARROW_RIGHT {
                            // TODO: Implement a better cursor tracking on each word here
                            INPUT_STATE.new_word();
                        }
                    }
                    PressedKey::Char(keycode) => {
                        if INPUT_STATE.is_enabled() {
                            match keycode {
                                KEY_ENTER | KEY_TAB | KEY_SPACE | KEY_ESCAPE => {
                                    let typing_buffer = INPUT_STATE.get_typing_buffer();
                                    let display_word = INPUT_STATE.get_displaying_word();
                                    let is_valid_word = vi::validation::is_valid_word(display_word);
                                    let is_allowed_word = INPUT_STATE.is_allowed_word(display_word);
                                    if should_restore_transformed_word(
                                        INPUT_STATE.get_method(),
                                        typing_buffer,
                                        display_word,
                                        is_valid_word,
                                        is_allowed_word,
                                    ) {
                                        do_restore_word(handle, modifiers.is_capslock());
                                    }

                                    if INPUT_STATE.previous_word_is_stop_tracking_words() {
                                        INPUT_STATE.clear_previous_word();
                                    }

                                    if keycode == KEY_TAB || keycode == KEY_SPACE {
                                        if let Some(macro_target) = INPUT_STATE.get_macro_target() {
                                            debug!("Macro: {}", macro_target);
                                            do_macro_replace(handle, &macro_target)
                                        }
                                    }

                                    let had_content = !INPUT_STATE.is_buffer_empty();
                                    INPUT_STATE.new_word();
                                    if had_content && (keycode == KEY_SPACE || keycode == KEY_TAB) {
                                        INPUT_STATE.mark_resumable();
                                    }
                                }
                                KEY_DELETE => {
                                    if !modifiers.is_empty() && !modifiers.is_shift() {
                                        INPUT_STATE.new_word();
                                    } else if INPUT_STATE.is_buffer_empty() {
                                        // Buffer is empty — the user just started a new
                                        // word (e.g. after space).  Try to resume editing
                                        // the previous word so backspace + retype works.
                                        // If resume fails, reset to a fresh tracking state
                                        // so the next keystrokes are processed (e.g. after
                                        // stop_tracking from a duplicate pattern like "ww").
                                        if !INPUT_STATE.try_resume_previous_word() {
                                            INPUT_STATE.new_word();
                                        }
                                    } else {
                                        INPUT_STATE.pop();
                                        if !INPUT_STATE.is_buffer_empty() {
                                            return do_transform_keys(
                                                handle,
                                                true,
                                                modifiers.is_capslock(),
                                            );
                                        }
                                    }
                                }
                                c => {
                                    if "()[]{}<>/\\!@#$%^&*-_=+|~`,.;'\"/".contains(c)
                                        || (c.is_numeric() && modifiers.is_shift())
                                    {
                                        // If special characters detected, dismiss the current tracking word
                                        if c.is_numeric() {
                                            INPUT_STATE.push(c);
                                        }
                                        INPUT_STATE.new_word();
                                    } else {
                                        // Otherwise, process the character
                                        if modifiers.is_super() || modifiers.is_alt() {
                                            INPUT_STATE.new_word();
                                        } else if INPUT_STATE.is_tracking() {
                                            INPUT_STATE.push(normalize_input_char(
                                                c,
                                                modifiers.is_shift(),
                                            ));
                                            let ret = do_transform_keys(
                                                handle,
                                                false,
                                                modifiers.is_capslock(),
                                            );
                                            INPUT_STATE.stop_tracking_if_needed();
                                            return ret;
                                        }
                                    }
                                }
                            }
                        } else {
                            match keycode {
                                KEY_ENTER | KEY_TAB | KEY_SPACE | KEY_ESCAPE => {
                                    INPUT_STATE.new_word();
                                }
                                _ => {
                                    if !modifiers.is_empty() {
                                        INPUT_STATE.new_word();
                                    }
                                }
                            }
                        }
                    }
                };
            }
            None => {
                let previous_modifiers = INPUT_STATE.get_previous_modifiers();
                if previous_modifiers.is_empty() {
                    if modifiers.is_control() {
                        if !INPUT_STATE.get_typing_buffer().is_empty() {
                            do_restore_word(handle, modifiers.is_capslock());
                        }
                        INPUT_STATE.set_temporary_disabled();
                    }
                    if modifiers.is_super() || event_type == EventTapType::Other {
                        INPUT_STATE.new_word();
                    }
                }
            }
        }
        INPUT_STATE.save_previous_modifiers(modifiers);
    }
    false
}

#[cfg(test)]
mod tests {
    use super::{apply_capslock_to_output, normalize_input_char, should_restore_transformed_word};
    use crate::input::TypingMethod;

    #[test]
    fn restore_when_invalid_and_not_allowed() {
        let should_restore =
            should_restore_transformed_word(TypingMethod::Telex, "maaa", "màa", false, false);
        assert!(should_restore);
    }

    #[test]
    fn no_restore_for_valid_word() {
        let should_restore =
            should_restore_transformed_word(TypingMethod::Telex, "tieens", "tiến", true, false);
        assert!(!should_restore);
    }

    #[test]
    fn no_restore_for_allowed_word() {
        let should_restore =
            should_restore_transformed_word(TypingMethod::Telex, "ddc", "đc", false, true);
        assert!(!should_restore);
    }

    #[test]
    fn no_restore_for_vni_numeric_shorthand() {
        let should_restore =
            should_restore_transformed_word(TypingMethod::VNI, "d9m", "đm", false, false);
        assert!(!should_restore);
    }

    #[test]
    fn restore_for_vni_invalid_without_numeric_shorthand() {
        let should_restore =
            should_restore_transformed_word(TypingMethod::VNI, "dam", "đm", false, false);
        assert!(should_restore);
    }

    #[test]
    fn normalize_input_char_only_depends_on_shift() {
        assert_eq!(normalize_input_char('d', true), 'D');
        assert_eq!(normalize_input_char('d', false), 'd');
    }

    #[test]
    fn apply_capslock_to_transformed_output() {
        let lower = String::from("duyệt");
        assert_eq!(apply_capslock_to_output(lower.clone(), false), "duyệt");
        assert_eq!(apply_capslock_to_output(lower, true), "DUYỆT");
    }

    #[test]
    fn capslock_path_keeps_telex_tone_position() {
        let mut transformed = String::new();
        vi::telex::transform_buffer("duyeetj".chars(), &mut transformed);

        assert_eq!(apply_capslock_to_output(transformed, true), "DUYỆT");
    }

    #[test]
    fn no_send_needed_for_plain_letter_with_capslock_only_case_change() {
        // For plain letters with Caps Lock, OS already inserts uppercase characters.
        // We should not treat case-only difference as a transform event.
        let mut transformed = String::new();
        vi::telex::transform_buffer("z".chars(), &mut transformed);
        assert_eq!(transformed, "z");
    }
}

fn main() {
    let app_title = format!("gõkey v{APP_VERSION}");
    env_logger::init();
    let skip_permission = std::env::args().any(|a| a == "--skip-permission");
    if !skip_permission && !ensure_accessibility_permission() {
        // Show the Accessibility Permission Request screen
        let win = WindowDesc::new(ui::permission_request_ui_builder())
            .title(app_title)
            .window_size((500.0, 360.0))
            .resizable(false);
        let app = AppLauncher::with_window(win);
        _ = app.launch(());
    } else {
        // Start the GõKey application
        rebuild_keyboard_layout_map();
        let win = WindowDesc::new(ui::main_ui_builder())
            .title(app_title)
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
        add_app_change_callback(|| {
            unsafe { auto_toggle_vietnamese() };
        });
        _ = app.launch(UIDataAdapter::new());
    }
}
