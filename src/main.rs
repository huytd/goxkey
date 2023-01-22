mod input;
mod platform;
mod ui;

use druid::{AppLauncher, ExtEventSink, Target, WindowDesc};
use input::{user_attempted_to_restore_a_word, InputState, INPUT_STATE};
use log::debug;
use once_cell::sync::OnceCell;
use platform::{
    run_event_listener, send_backspace, send_string, Handle, KeyModifier, KEY_DELETE, KEY_ENTER,
    KEY_ESCAPE, KEY_SPACE, KEY_TAB,
};
use std::{sync::MutexGuard, thread};
use ui::{UIDataAdapter, UPDATE_UI};

static UI_EVENT_SINK: OnceCell<ExtEventSink> = OnceCell::new();

fn process_character(
    input_state: &mut MutexGuard<InputState>,
    handle: Handle,
    c: char,
    modifiers: KeyModifier,
) -> bool {
    if modifiers.is_super() || modifiers.is_control() || modifiers.is_alt() {
        input_state.new_word();
    } else if input_state.should_track {
        input_state.push(if modifiers.is_shift() {
            c.to_ascii_uppercase()
        } else {
            c
        });
        debug!("BUFFER: {:?}", input_state.buffer);
        if input_state.should_process(&c) {
            let output = input_state.process_key();
            debug!("TRANSFORMED: {:?}", output);
            if !input_state.buffer.eq(&output) {
                let backspace_count = input_state.get_backspace_count();
                debug!("BACKSPACE: {}", backspace_count);
                _ = send_backspace(handle, backspace_count);
                _ = send_string(handle, &output);
                if user_attempted_to_restore_a_word(&input_state.display_buffer, &output) {
                    input_state.stop_tracking();
                } else {
                    input_state.replace(output);
                }
                return true;
            }
        }
    }
    return false;
}

fn event_handler(handle: Handle, keycode: Option<char>, modifiers: KeyModifier) -> bool {
    let mut input_state = INPUT_STATE.lock().unwrap();

    match keycode {
        Some(keycode) => {
            // Toggle Vietnamese input mod with Ctrl + Cmd + Space key
            if modifiers.is_control() && modifiers.is_super() && keycode == KEY_SPACE {
                input_state.toggle_vietnamese();
                if let Some(event_sink) = UI_EVENT_SINK.get() {
                    _ = event_sink.submit_command(UPDATE_UI, (), Target::Auto);
                }
                return true;
            }

            if input_state.enabled {
                match keycode {
                    KEY_ENTER | KEY_TAB | KEY_SPACE | KEY_ESCAPE => {
                        input_state.new_word();
                    }
                    KEY_DELETE => {
                        input_state.pop();
                    }
                    c => {
                        return process_character(&mut input_state, handle, c, modifiers);
                    }
                }
            }
        }
        None => {
            input_state.new_word();
        }
    }
    false
}

fn main() {
    env_logger::init();

    let win = WindowDesc::new(ui::main_ui_builder)
        .title("gõkey")
        .window_size((320.0, 200.0))
        .resizable(false);
    let app = AppLauncher::with_window(win);
    let event_sink = app.get_external_handle();
    _ = UI_EVENT_SINK.set(event_sink);

    thread::spawn(|| {
        run_event_listener(&event_handler);
    });

    _ = app.launch(UIDataAdapter::new());
}
