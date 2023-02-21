use crate::{
    input::{rebuild_keyboard_layout_map, TypingMethod, INPUT_STATE},
    platform::{KeyModifier, KEY_SPACE, SYMBOL_ALT, SYMBOL_CTRL, SYMBOL_SHIFT, SYMBOL_SUPER},
};
use druid::{
    text::{
        format::{Formatter, Validation},
        TextInput,
    },
    theme::{BACKGROUND_DARK, BORDER_DARK, PLACEHOLDER_COLOR},
    widget::{Button, Checkbox, Container, Controller, Flex, Label, RadioGroup, Switch, TextBox},
    Data, Env, Event, EventCtx, KeyModifiers, Lens, Selector, Widget, WidgetExt,
};

pub const UPDATE_UI: Selector = Selector::new("gox-ui.update-ui");

// TODO:
// 2. Implement update for Controller to submit change to INPUT_STATE

pub fn format_letter_key(c: char) -> String {
    if c.is_ascii_whitespace() {
        String::from("Space")
    } else {
        c.to_ascii_uppercase().to_string()
    }
}

pub fn letter_key_to_char(input: &str) -> char {
    match input {
        "Space" => ' ',
        s => s.chars().last().unwrap(),
    }
}

struct LetterKeyController;
impl<W: Widget<UIDataAdapter>> Controller<UIDataAdapter, W> for LetterKeyController {
    fn event(
        &mut self,
        child: &mut W,
        ctx: &mut EventCtx,
        event: &Event,
        data: &mut UIDataAdapter,
        env: &Env,
    ) {
        if let &Event::KeyUp(_) = event {
            match data.letter_key.as_str() {
                "Space" => {}
                s => {
                    if let Some(last_char) = s.chars().last() {
                        data.letter_key = format_letter_key(last_char);
                    }
                }
            }
        }
        child.event(ctx, event, data, env)
    }
}

#[derive(Clone, Data, Lens, PartialEq, Eq)]
pub struct UIDataAdapter {
    is_enabled: bool,
    typing_method: TypingMethod,
    hotkey_display: String,
    // Hotkey config
    super_key: bool,
    ctrl_key: bool,
    alt_key: bool,
    shift_key: bool,
    letter_key: String,
}

impl UIDataAdapter {
    pub fn new() -> Self {
        let mut ret = Self {
            is_enabled: true,
            typing_method: TypingMethod::Telex,
            hotkey_display: String::new(),
            // TODO: These values should be read from hotkey object
            super_key: true,
            ctrl_key: true,
            alt_key: false,
            shift_key: false,
            letter_key: String::from("Space"),
        };
        ret.update();
        ret
    }

    pub fn update(&mut self) {
        unsafe {
            self.is_enabled = INPUT_STATE.is_enabled();
            self.typing_method = INPUT_STATE.get_method();
            self.hotkey_display = INPUT_STATE.get_hotkey().to_string();

            let (modifiers, keycode) = INPUT_STATE.get_hotkey().inner();
            self.super_key = modifiers.is_super();
            self.ctrl_key = modifiers.is_control();
            self.alt_key = modifiers.is_alt();
            self.shift_key = modifiers.is_shift();
            self.letter_key = format_letter_key(keycode);
        }
    }

    pub fn toggle_vietnamese(&mut self) {
        unsafe {
            INPUT_STATE.toggle_vietnamese();
        }
        self.update();
    }
}

pub struct UIController;

impl<W: Widget<UIDataAdapter>> Controller<UIDataAdapter, W> for UIController {
    fn event(
        &mut self,
        child: &mut W,
        ctx: &mut EventCtx,
        event: &Event,
        data: &mut UIDataAdapter,
        env: &Env,
    ) {
        match event {
            Event::Command(cmd) => match cmd.get(UPDATE_UI) {
                Some(_) => {
                    data.update();
                    rebuild_keyboard_layout_map();
                }
                None => {}
            },
            _ => {}
        }
        child.event(ctx, event, data, env)
    }

    fn update(
        &mut self,
        child: &mut W,
        ctx: &mut druid::UpdateCtx,
        old_data: &UIDataAdapter,
        data: &UIDataAdapter,
        env: &Env,
    ) {
        unsafe {
            if old_data.typing_method != data.typing_method {
                INPUT_STATE.set_method(data.typing_method);
            }

            if !data.letter_key.is_empty() {
                let mut new_mod = KeyModifier::new();
                new_mod.apply(data.super_key, data.ctrl_key, data.alt_key, data.shift_key);
                let key_code = letter_key_to_char(&data.letter_key);
                if !INPUT_STATE.get_hotkey().is_match(new_mod, &key_code) {
                    INPUT_STATE.set_hotkey(&format!(
                        "{}{}",
                        new_mod,
                        match key_code {
                            ' ' => String::from("space"),
                            c => c.to_string(),
                        }
                    ));
                }
            }
        }
        child.update(ctx, old_data, data, env);
    }
}

pub fn main_ui_builder() -> impl Widget<UIDataAdapter> {
    Flex::column()
        .cross_axis_alignment(druid::widget::CrossAxisAlignment::Start)
        .main_axis_alignment(druid::widget::MainAxisAlignment::Start)
        .with_child(
            Container::new(
                Flex::column()
                    .cross_axis_alignment(druid::widget::CrossAxisAlignment::Start)
                    .main_axis_alignment(druid::widget::MainAxisAlignment::Start)
                    .with_child(
                        Flex::row()
                            .with_child(Label::new("Chế độ gõ tiếng Việt"))
                            .with_child(Switch::new().lens(UIDataAdapter::is_enabled).on_click(
                                |_, data, _| {
                                    data.toggle_vietnamese();
                                },
                            ))
                            .cross_axis_alignment(druid::widget::CrossAxisAlignment::Start)
                            .main_axis_alignment(druid::widget::MainAxisAlignment::SpaceBetween)
                            .must_fill_main_axis(true)
                            .expand_width()
                            .padding(8.0),
                    )
                    .with_child(
                        Flex::row()
                            .with_child(Label::new("Kiểu gõ"))
                            .with_child(
                                RadioGroup::new(vec![
                                    ("Telex", TypingMethod::Telex),
                                    ("VNI", TypingMethod::VNI),
                                ])
                                .lens(UIDataAdapter::typing_method),
                            )
                            .cross_axis_alignment(druid::widget::CrossAxisAlignment::Start)
                            .main_axis_alignment(druid::widget::MainAxisAlignment::SpaceBetween)
                            .must_fill_main_axis(true)
                            .expand_width()
                            .padding(8.0),
                    )
                    .with_child(
                        Flex::row()
                            .with_child(Label::new("Bật tắt gõ tiếng Việt"))
                            .with_child(
                                Label::dynamic(|data: &UIDataAdapter, _| {
                                    data.hotkey_display.to_owned()
                                })
                                .border(PLACEHOLDER_COLOR, 1.0)
                                .rounded(4.0),
                            )
                            .cross_axis_alignment(druid::widget::CrossAxisAlignment::Start)
                            .main_axis_alignment(druid::widget::MainAxisAlignment::SpaceBetween)
                            .must_fill_main_axis(true)
                            .expand_width()
                            .padding(8.0),
                    )
                    .with_child(
                        Flex::row()
                            .with_child(Checkbox::new(SYMBOL_SUPER).lens(UIDataAdapter::super_key))
                            .with_child(Checkbox::new(SYMBOL_CTRL).lens(UIDataAdapter::ctrl_key))
                            .with_child(Checkbox::new(SYMBOL_ALT).lens(UIDataAdapter::alt_key))
                            .with_child(Checkbox::new(SYMBOL_SHIFT).lens(UIDataAdapter::shift_key))
                            .with_child(
                                TextBox::new()
                                    .lens(UIDataAdapter::letter_key)
                                    .controller(LetterKeyController),
                            )
                            .cross_axis_alignment(druid::widget::CrossAxisAlignment::End)
                            .main_axis_alignment(druid::widget::MainAxisAlignment::SpaceBetween)
                            .must_fill_main_axis(true)
                            .expand_width()
                            .padding(8.0),
                    ),
            )
            .border(BORDER_DARK, 1.0)
            .rounded(4.0)
            .background(BACKGROUND_DARK),
        )
        .with_spacer(8.0)
        .with_child(
            Flex::row()
                .with_child(Button::new("Cài đặt mặc định").fix_height(28.0))
                .with_spacer(8.0)
                .with_child(Button::new("Đóng").fix_width(100.0).fix_height(28.0))
                .cross_axis_alignment(druid::widget::CrossAxisAlignment::Start)
                .main_axis_alignment(druid::widget::MainAxisAlignment::End)
                .must_fill_main_axis(true)
                .expand_width(),
        )
        .padding(8.0)
        .controller(UIController)
}
