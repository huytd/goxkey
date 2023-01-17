mod input;
mod platform;

use druid::{Widget, widget::{Label, Button, Flex}, WidgetExt, WindowDesc, AppLauncher, Data, Lens};
use input::InputState;
use lazy_static::lazy_static;
use log::debug;
use platform::{
    run_event_listener, send_backspace, send_string, Handle, KeyModifier, KEY_DELETE, KEY_ENTER,
    KEY_ESCAPE, KEY_SPACE, KEY_TAB,
};
use std::{sync::Mutex, thread};

lazy_static! {
    static ref INPUT_STATE: Mutex<InputState> = Mutex::new(InputState::new());
}

fn event_handler(handle: Handle, keycode: Option<char>, modifiers: KeyModifier) -> bool {
    let mut input_state = INPUT_STATE.lock().unwrap();

    match keycode {
        Some(keycode) => {
            // Toggle Vietnamese input mod with Ctrl + Cmd + Space key
            if modifiers.is_control() && modifiers.is_super() && keycode == KEY_SPACE {
                input_state.toggle_vietnamese();
                return true;
            }

            if input_state.enabled {
                match keycode {
                    KEY_ENTER | KEY_TAB | KEY_SPACE | KEY_ESCAPE => {
                        input_state.clear();
                    }
                    KEY_DELETE => {
                        input_state.pop();
                    }
                    c => {
                        if modifiers.is_super() || modifiers.is_control() || modifiers.is_alt() {
                            input_state.clear();
                        } else {
                            input_state.push(if modifiers.is_shift() {
                                c.to_ascii_uppercase()
                            } else {
                                c
                            });

                            if input_state.should_process(&keycode) {
                                let output = input_state.process_key();
                                if !input_state.buffer.eq(&output) {
                                    debug!("BUF {:?} - RET {:?}", input_state.buffer, output);
                                    let backspace_count = input_state.buffer.chars().count() - 1;
                                    debug!("  DEL {} - SEND {}", backspace_count, output);
                                    _ = send_backspace(handle, backspace_count);
                                    _ = send_string(handle, &output);
                                    input_state.replace(output);
                                    return true;
                                }
                            }
                        }
                    }
                }
            }
        },
        None => {
            input_state.clear();
        }
    }
    false
}

#[derive(Clone, Data, Lens)]
struct GoxData {
    input_status: String
}

impl GoxData {
    pub fn new() -> Self {
        let mut ret = Self {
            input_status: String::new()
        };
        let input_state = INPUT_STATE.lock().unwrap();
        ret.update(&input_state);
        ret
    }

    pub fn update(&mut self, input_state: &InputState) {
        self.input_status = format!("Gõ tiếng Việt = {}", if input_state.enabled { "ON" } else { "OFF" });
    }

    pub fn toggle_input(&mut self) {
        let mut input_state = INPUT_STATE.lock().unwrap();
        input_state.toggle_vietnamese();
        self.update(&input_state);
    }
}

fn main_ui_builder() -> impl Widget<GoxData> {
    let label = Label::new(|data: &String, _env: &_| data.clone())
        .lens(GoxData::input_status)
        .padding(5.0);
    let button = Button::new("Chuyển chế độ gõ")
        .on_click(|_, data: &mut GoxData, _| {
            data.toggle_input();
        })
        .padding(5.0);
    Flex::column().with_child(label).with_child(button).padding(5.0)
}

fn main() {
    env_logger::init();
    thread::spawn(|| {
        run_event_listener(&event_handler);
    });

    let win = WindowDesc::new(main_ui_builder)
        .title("gõkey")
        .window_size((300.0, 100.0));
    _ = AppLauncher::with_window(win).launch(GoxData::new());
}
