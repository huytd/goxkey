use std::sync::Arc;

use crate::{
    input::{rebuild_keyboard_layout_map, TypingMethod, INPUT_STATE},
    platform::{
        self, KeyModifier, SystemTray, SystemTrayMenuItemKey, SYMBOL_ALT, SYMBOL_CTRL,
        SYMBOL_SHIFT, SYMBOL_SUPER,
    },
    UI_EVENT_SINK,
};
use druid::{
    commands::QUIT_APP,
    theme::{BACKGROUND_DARK, BORDER_DARK, PLACEHOLDER_COLOR},
    widget::{
        Button, Checkbox, Container, Controller, FillStrat, Flex, Image, Label, LineBreaking, List,
        RadioGroup, Scroll, Switch, TextBox,
    },
    Application, Color, Data, Env, Event, EventCtx, ImageBuf, Lens, Selector, Target, Widget,
    WidgetExt, WindowDesc,
};

pub const UPDATE_UI: Selector = Selector::new("gox-ui.update-ui");
const DELETE_MACRO: Selector<String> = Selector::new("gox-ui.delete-macro");
const ADD_MACRO: Selector = Selector::new("gox-ui.add-macro");

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

#[derive(Clone, Data, PartialEq, Eq)]
struct MacroEntry {
    from: String,
    to: String,
}

#[derive(Clone, Data, Lens, PartialEq, Eq)]
pub struct UIDataAdapter {
    is_enabled: bool,
    typing_method: TypingMethod,
    hotkey_display: String,
    // Macro config
    is_macro_enabled: bool,
    macro_table: Arc<Vec<MacroEntry>>,
    new_macro_from: String,
    new_macro_to: String,
    // Hotkey config
    super_key: bool,
    ctrl_key: bool,
    alt_key: bool,
    shift_key: bool,
    capslock_key: bool,
    letter_key: String,
    // system tray
    systray: SystemTray,
}

impl UIDataAdapter {
    pub fn new() -> Self {
        let mut ret = Self {
            is_enabled: true,
            typing_method: TypingMethod::Telex,
            hotkey_display: String::new(),
            is_macro_enabled: false,
            macro_table: Arc::new(Vec::new()),
            new_macro_from: String::new(),
            new_macro_to: String::new(),
            super_key: true,
            ctrl_key: true,
            alt_key: false,
            shift_key: false,
            capslock_key: false,
            letter_key: String::from("Space"),
            systray: SystemTray::new(),
        };
        ret.setup_system_tray_actions();
        ret.update();
        ret
    }

    pub fn update(&mut self) {
        unsafe {
            self.is_enabled = INPUT_STATE.is_enabled();
            self.typing_method = INPUT_STATE.get_method();
            self.hotkey_display = INPUT_STATE.get_hotkey().to_string();
            self.is_macro_enabled = INPUT_STATE.is_macro_enabled();
            self.macro_table = Arc::new(
                INPUT_STATE
                    .get_macro_table()
                    .iter()
                    .map(|(source, target)| MacroEntry {
                        from: source.to_string(),
                        to: target.to_string(),
                    })
                    .collect::<Vec<MacroEntry>>(),
            );

            let (modifiers, keycode) = INPUT_STATE.get_hotkey().inner();
            self.super_key = modifiers.is_super();
            self.ctrl_key = modifiers.is_control();
            self.alt_key = modifiers.is_alt();
            self.shift_key = modifiers.is_shift();
            self.letter_key = format_letter_key(keycode);

            match self.is_enabled {
                true => {
                    self.systray.set_title("VN");
                    self.systray
                        .set_menu_item_title(SystemTrayMenuItemKey::Enable, "Tắt gõ tiếng việt");
                }
                false => {
                    self.systray.set_title("EN");
                    self.systray
                        .set_menu_item_title(SystemTrayMenuItemKey::Enable, "Bật gõ tiếng việt");
                }
            }
            match self.typing_method {
                TypingMethod::VNI => {
                    self.systray
                        .set_menu_item_title(SystemTrayMenuItemKey::TypingMethodTelex, "Telex");
                    self.systray
                        .set_menu_item_title(SystemTrayMenuItemKey::TypingMethodVNI, "VNI ✓");
                }
                TypingMethod::Telex => {
                    self.systray
                        .set_menu_item_title(SystemTrayMenuItemKey::TypingMethodTelex, "Telex ✓");
                    self.systray
                        .set_menu_item_title(SystemTrayMenuItemKey::TypingMethodVNI, "VNI");
                }
            }
        }
    }

    fn setup_system_tray_actions(&mut self) {
        self.systray
            .set_menu_item_callback(SystemTrayMenuItemKey::Enable, || {
                unsafe {
                    INPUT_STATE.toggle_vietnamese();
                }
                UI_EVENT_SINK
                    .get()
                    .map(|event| Some(event.submit_command(UPDATE_UI, (), Target::Auto)));
            });
        self.systray
            .set_menu_item_callback(SystemTrayMenuItemKey::TypingMethodTelex, || {
                unsafe {
                    INPUT_STATE.set_method(TypingMethod::Telex);
                }
                UI_EVENT_SINK
                    .get()
                    .map(|event| Some(event.submit_command(UPDATE_UI, (), Target::Auto)));
            });
        self.systray
            .set_menu_item_callback(SystemTrayMenuItemKey::TypingMethodVNI, || {
                unsafe {
                    INPUT_STATE.set_method(TypingMethod::VNI);
                }
                UI_EVENT_SINK
                    .get()
                    .map(|event| Some(event.submit_command(UPDATE_UI, (), Target::Auto)));
            });
        self.systray
            .set_menu_item_callback(SystemTrayMenuItemKey::Exit, || {
                UI_EVENT_SINK
                    .get()
                    .map(|event| Some(event.submit_command(QUIT_APP, (), Target::Auto)));
            });
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
            Event::Command(cmd) => {
                if cmd.get(UPDATE_UI).is_some() {
                    data.update();
                    rebuild_keyboard_layout_map();
                }
                if let Some(source) = cmd.get(DELETE_MACRO) {
                    unsafe { INPUT_STATE.delete_macro(source) };
                    data.update();
                }
                if cmd.get(ADD_MACRO).is_some()
                    && !data.new_macro_from.is_empty()
                    && !data.new_macro_to.is_empty()
                {
                    unsafe {
                        INPUT_STATE
                            .add_macro(data.new_macro_from.clone(), data.new_macro_to.clone())
                    };
                    data.new_macro_from = String::new();
                    data.new_macro_to = String::new();
                    data.update();
                }
            }
            Event::WindowCloseRequested => {
                ctx.set_handled();
                ctx.submit_command(platform::HIDE_COMMAND);
            }
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
                new_mod.apply(
                    data.super_key,
                    data.ctrl_key,
                    data.alt_key,
                    data.shift_key,
                    data.capslock_key,
                );
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

            if old_data.is_macro_enabled != data.is_macro_enabled {
                INPUT_STATE.toggle_macro_enabled();
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
                                RadioGroup::column(vec![
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
                            .with_child(Label::new("Gõ tắt"))
                            .with_child(Checkbox::new("").lens(UIDataAdapter::is_macro_enabled))
                            .cross_axis_alignment(druid::widget::CrossAxisAlignment::Start)
                            .main_axis_alignment(druid::widget::MainAxisAlignment::SpaceBetween)
                            .must_fill_main_axis(true)
                            .expand_width()
                            .padding(8.0),
                    )
                    .with_child(
                        Flex::row()
                            .with_child(Button::new("Bảng gõ tắt").on_click(|ctx, _, _| {
                                let new_win_position = ctx.window().get_position() - (50.0, 50.0); // offset a bit
                                let new_window = WindowDesc::new(macro_editor_ui_builder())
                                    .title("Bảng gõ tắt")
                                    .window_size((320.0, 320.0))
                                    .with_min_size((320.0, 320.0))
                                    .set_position(new_win_position);
                                ctx.new_window(new_window);
                            }))
                            .cross_axis_alignment(druid::widget::CrossAxisAlignment::Start)
                            .main_axis_alignment(druid::widget::MainAxisAlignment::End)
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
                .with_child(
                    Button::new("Đóng")
                        .fix_width(100.0)
                        .fix_height(28.0)
                        .on_click(|event, _, _| event.submit_command(platform::HIDE_COMMAND)),
                )
                .cross_axis_alignment(druid::widget::CrossAxisAlignment::Start)
                .main_axis_alignment(druid::widget::MainAxisAlignment::End)
                .must_fill_main_axis(true)
                .expand_width(),
        )
        .padding(8.0)
        .controller(UIController)
}

pub fn permission_request_ui_builder() -> impl Widget<()> {
    let image_data = ImageBuf::from_data(include_bytes!("../assets/accessibility.png")).unwrap();
    Flex::column()
        .cross_axis_alignment(druid::widget::CrossAxisAlignment::Start)
        .main_axis_alignment(druid::widget::MainAxisAlignment::Start)
        .with_child(
            Label::new("Chờ đã! Bạn cần phải cấp quyền Accessibility cho ứng dụng GõKey trước khi sử dụng.")
                .with_line_break_mode(LineBreaking::WordWrap)
                .padding(6.0)
        )
        .with_child(
            Container::new(Image::new(image_data).fill_mode(FillStrat::Cover))
                .rounded(4.0)
                .padding(6.0)
        )
        .with_child(
            Label::new("Bạn vui lòng thoát khỏi ứng dụng và mở lại sau khi đã cấp quyền.")
                .with_line_break_mode(LineBreaking::WordWrap)
                .padding(6.0)
        )
        .with_child(
            Flex::row()
                .cross_axis_alignment(druid::widget::CrossAxisAlignment::End)
                .main_axis_alignment(druid::widget::MainAxisAlignment::End)
                .with_child(
                    Button::new("Thoát")
                        .fix_width(100.0)
                        .fix_height(28.0)
                        .on_click(|_, _, _| {
                            Application::global().quit();
                        })
                        .padding(6.0)
                )
                .must_fill_main_axis(true)
        )
        .must_fill_main_axis(true)
        .padding(6.0)
}

pub fn macro_editor_ui_builder() -> impl Widget<UIDataAdapter> {
    Flex::column()
        .cross_axis_alignment(druid::widget::CrossAxisAlignment::Start)
        .main_axis_alignment(druid::widget::MainAxisAlignment::SpaceBetween)
        .with_child(
            Flex::row()
                .with_child(Label::new("Bảng gõ tắt"))
                .main_axis_alignment(druid::widget::MainAxisAlignment::Center)
                .expand_width(),
        )
        .with_flex_child(
            {
                let mut scroll = Scroll::new(
                    List::new(macro_row_item)
                        .lens(UIDataAdapter::macro_table)
                        .expand_width(),
                );
                scroll.set_enabled_scrollbars(druid::scroll_component::ScrollbarsEnabled::Vertical);
                scroll.set_horizontal_scroll_enabled(false);
                scroll
            }
            .expand_width(),
            1.0,
        )
        .with_default_spacer()
        .with_child(
            Flex::row()
                .with_flex_child(
                    TextBox::new()
                        .with_placeholder("Gõ tắt mới")
                        .with_text_alignment(druid::text::TextAlignment::Start)
                        .expand_width()
                        .lens(UIDataAdapter::new_macro_from),
                    2.0,
                )
                .with_flex_child(
                    TextBox::new()
                        .with_placeholder("thay thế")
                        .with_text_alignment(druid::text::TextAlignment::Start)
                        .expand_width()
                        .lens(UIDataAdapter::new_macro_to),
                    2.0,
                )
                .with_flex_child(
                    Button::new("Thêm")
                        .on_click(|ctx, _, _| ctx.submit_command(ADD_MACRO.to(Target::Global))),
                    1.0,
                )
                .main_axis_alignment(druid::widget::MainAxisAlignment::SpaceBetween)
                .cross_axis_alignment(druid::widget::CrossAxisAlignment::Baseline)
                .expand_width()
                .border(Color::GRAY, 0.5),
        )
        .with_child(
            Flex::row()
                .with_child(
                    Button::new("Đóng")
                        .on_click(|ctx, _, _| {
                            ctx.submit_command(platform::CLOSE_COMMAND.to(Target::Auto))
                        })
                        .fix_width(100.0)
                        .fix_height(28.0),
                )
                .main_axis_alignment(druid::widget::MainAxisAlignment::End)
                .expand_width()
                .padding(6.0),
        )
        .must_fill_main_axis(true)
        .expand_width()
        .padding(8.0)
}

fn macro_row_item() -> impl Widget<MacroEntry> {
    Flex::row()
        .with_flex_child(
            Label::dynamic(|e: &MacroEntry, _| e.from.clone())
                .with_line_break_mode(LineBreaking::WordWrap)
                .align_left(),
            2.0,
        )
        .with_flex_child(
            Label::dynamic(|e: &MacroEntry, _| e.to.clone())
                .with_line_break_mode(LineBreaking::WordWrap)
                .align_left(),
            2.0,
        )
        .with_flex_child(
            Button::new("❌").on_click(|ctx, data: &mut MacroEntry, _| {
                ctx.submit_command(DELETE_MACRO.with(data.from.clone()).to(Target::Global))
            }),
            1.0,
        )
        .main_axis_alignment(druid::widget::MainAxisAlignment::SpaceBetween)
        .cross_axis_alignment(druid::widget::CrossAxisAlignment::Baseline)
        .expand_width()
        .border(Color::GRAY, 0.5)
}
