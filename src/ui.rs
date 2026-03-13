use std::sync::Arc;

use crate::{
    input::{rebuild_keyboard_layout_map, TypingMethod, INPUT_STATE},
    platform::{
        defer_open_app_file_picker, is_launch_on_login, update_launch_on_login, KeyModifier,
        SystemTray, SystemTrayMenuItemKey, SYMBOL_ALT, SYMBOL_CTRL, SYMBOL_SHIFT, SYMBOL_SUPER,
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
    Application, Color, Data, Env, Event, EventCtx, ImageBuf, Lens, Screen, Selector, Target,
    Widget, WidgetExt, WindowDesc,
};
use log::error;

pub const UPDATE_UI: Selector = Selector::new("gox-ui.update-ui");
pub const SHOW_UI: Selector = Selector::new("gox-ui.show-ui");
const DELETE_MACRO: Selector<String> = Selector::new("gox-ui.delete-macro");
const ADD_MACRO: Selector = Selector::new("gox-ui.add-macro");
const DELETE_VN_APP: Selector<String> = Selector::new("gox-ui.delete-vn-app");
const DELETE_EN_APP: Selector<String> = Selector::new("gox-ui.delete-en-app");
const ADD_VN_APP: Selector = Selector::new("gox-ui.add-vn-app");
const ADD_EN_APP: Selector = Selector::new("gox-ui.add-en-app");
const SET_VN_APP_FROM_PICKER: Selector<String> = Selector::new("gox-ui.set-vn-app-from-picker");
const SET_EN_APP_FROM_PICKER: Selector<String> = Selector::new("gox-ui.set-en-app-from-picker");
pub const WINDOW_WIDTH: f64 = 335.0;
pub const WINDOW_HEIGHT: f64 = 375.0;

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
        if let &Event::MouseDown(_) = event {
            ctx.submit_command(druid::commands::SELECT_ALL);
        }
        if let &Event::KeyUp(_) = event {
            match data.letter_key.as_str() {
                "Space" => {}
                s => {
                    data.letter_key = format_letter_key(letter_key_to_char(s));
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

#[derive(Clone, Data, PartialEq, Eq)]
struct AppEntry {
    name: String,
}

#[derive(Clone, Data, Lens, PartialEq, Eq)]
pub struct UIDataAdapter {
    is_enabled: bool,
    typing_method: TypingMethod,
    hotkey_display: String,
    launch_on_login: bool,
    is_auto_toggle_enabled: bool,
    // Macro config
    is_macro_enabled: bool,
    macro_table: Arc<Vec<MacroEntry>>,
    new_macro_from: String,
    new_macro_to: String,
    // App language settings
    vn_apps: Arc<Vec<AppEntry>>,
    en_apps: Arc<Vec<AppEntry>>,
    new_vn_app: String,
    new_en_app: String,
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
            launch_on_login: false,
            is_auto_toggle_enabled: false,
            is_macro_enabled: false,
            macro_table: Arc::new(Vec::new()),
            new_macro_from: String::new(),
            new_macro_to: String::new(),
            vn_apps: Arc::new(Vec::new()),
            en_apps: Arc::new(Vec::new()),
            new_vn_app: String::new(),
            new_en_app: String::new(),
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
            self.is_auto_toggle_enabled = INPUT_STATE.is_auto_toggle_enabled();
            self.launch_on_login = is_launch_on_login();
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
            self.vn_apps = Arc::new(
                INPUT_STATE
                    .get_vn_apps()
                    .into_iter()
                    .map(|name| AppEntry { name })
                    .collect(),
            );
            self.en_apps = Arc::new(
                INPUT_STATE
                    .get_en_apps()
                    .into_iter()
                    .map(|name| AppEntry { name })
                    .collect(),
            );

            let (modifiers, keycode) = INPUT_STATE.get_hotkey().inner();
            self.super_key = modifiers.is_super();
            self.ctrl_key = modifiers.is_control();
            self.alt_key = modifiers.is_alt();
            self.shift_key = modifiers.is_shift();
            self.letter_key = format_letter_key(keycode);

            match self.is_enabled {
                true => {
                    let title = if INPUT_STATE.is_gox_mode_enabled() {
                        "gõ"
                    } else {
                        "VN"
                    };
                    self.systray.set_title(title);
                    self.systray
                        .set_menu_item_title(SystemTrayMenuItemKey::Enable, "Tắt gõ tiếng Việt");
                }
                false => {
                    let title = if INPUT_STATE.is_gox_mode_enabled() {
                        match self.typing_method {
                            TypingMethod::Telex => "gox",
                            TypingMethod::VNI => "go4",
                        }
                    } else {
                        "EN"
                    };
                    self.systray.set_title(title);
                    self.systray
                        .set_menu_item_title(SystemTrayMenuItemKey::Enable, "Bật gõ tiếng Việt");
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
            .set_menu_item_callback(SystemTrayMenuItemKey::ShowUI, || {
                UI_EVENT_SINK
                    .get()
                    .map(|event| Some(event.submit_command(SHOW_UI, (), Target::Auto)));
            });
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
                if cmd.get(SHOW_UI).is_some() {
                    ctx.set_handled();
                    ctx.window().bring_to_front_and_focus();
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
                if let Some(app_name) = cmd.get(DELETE_VN_APP) {
                    unsafe { INPUT_STATE.remove_vietnamese_app(app_name) };
                    data.update();
                }
                if let Some(app_name) = cmd.get(DELETE_EN_APP) {
                    unsafe { INPUT_STATE.remove_english_app(app_name) };
                    data.update();
                }
                if cmd.get(ADD_VN_APP).is_some() && !data.new_vn_app.is_empty() {
                    unsafe { INPUT_STATE.add_vietnamese_app(&data.new_vn_app.clone()) };
                    data.new_vn_app = String::new();
                    data.update();
                }
                if cmd.get(ADD_EN_APP).is_some() && !data.new_en_app.is_empty() {
                    unsafe { INPUT_STATE.add_english_app(&data.new_en_app.clone()) };
                    data.new_en_app = String::new();
                    data.update();
                }
                if let Some(name) = cmd.get(SET_VN_APP_FROM_PICKER) {
                    data.new_vn_app = name.clone();
                }
                if let Some(name) = cmd.get(SET_EN_APP_FROM_PICKER) {
                    data.new_en_app = name.clone();
                }
            }
            Event::WindowCloseRequested => {
                ctx.set_handled();
                ctx.window().hide();
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

            if old_data.launch_on_login != data.launch_on_login {
                if let Err(err) = update_launch_on_login(data.launch_on_login) {
                    error!("{}", err);
                }
            }

            // Update hotkey
            {
                let mut new_mod = KeyModifier::new();
                new_mod.apply(
                    data.super_key,
                    data.ctrl_key,
                    data.alt_key,
                    data.shift_key,
                    data.capslock_key,
                );
                let key_code = letter_key_to_char(&data.letter_key);
                if !INPUT_STATE.get_hotkey().is_match(new_mod, key_code) {
                    INPUT_STATE.set_hotkey(&format!(
                        "{}{}",
                        new_mod,
                        match key_code {
                            Some(' ') => String::from("space"),
                            Some(c) => c.to_string(),
                            _ => String::new(),
                        }
                    ));
                }
            }

            if old_data.is_macro_enabled != data.is_macro_enabled {
                INPUT_STATE.toggle_macro_enabled();
            }

            if old_data.is_auto_toggle_enabled != data.is_auto_toggle_enabled {
                INPUT_STATE.toggle_auto_toggle();
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
                            .with_child(Label::new("Khởi động cùng OS"))
                            .with_child(Checkbox::new("").lens(UIDataAdapter::launch_on_login))
                            .cross_axis_alignment(druid::widget::CrossAxisAlignment::Start)
                            .main_axis_alignment(druid::widget::MainAxisAlignment::SpaceBetween)
                            .must_fill_main_axis(true)
                            .expand_width()
                            .padding(8.0),
                    )
                    .with_child(
                        Flex::row()
                            .with_child(Label::new("Bật tắt theo ứng dụng"))
                            .with_child(
                                Checkbox::new("").lens(UIDataAdapter::is_auto_toggle_enabled),
                            )
                            .cross_axis_alignment(druid::widget::CrossAxisAlignment::Start)
                            .main_axis_alignment(druid::widget::MainAxisAlignment::SpaceBetween)
                            .must_fill_main_axis(true)
                            .expand_width()
                            .padding(8.0),
                    )
                    .with_child(
                        Flex::row()
                            .with_child(Button::new("Danh sách ứng dụng").on_click(|ctx, _, _| {
                                let new_win_position = ctx.window().get_position() - (50.0, 50.0);
                                let new_window = WindowDesc::new(app_settings_ui_builder())
                                    .title("Danh sách ứng dụng")
                                    .window_size((420.0, 360.0))
                                    .with_min_size((420.0, 360.0))
                                    .set_always_on_top(true)
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
                                let new_win_position = ctx.window().get_position() - (50.0, 50.0);
                                let new_window = WindowDesc::new(macro_editor_ui_builder())
                                    .title("Bảng gõ tắt")
                                    .window_size((320.0, 320.0))
                                    .with_min_size((320.0, 320.0))
                                    .set_always_on_top(true)
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
                        .on_click(|event, _, _| {
                            event.window().hide();
                        }),
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
        .with_spacer(10.0)
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
            .expand(),
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
                        .on_click(|ctx, _, _| ctx.window().close())
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
            Button::new("×").on_click(|ctx, data: &mut MacroEntry, _| {
                ctx.submit_command(DELETE_MACRO.with(data.from.clone()).to(Target::Global))
            }),
            1.0,
        )
        .main_axis_alignment(druid::widget::MainAxisAlignment::SpaceBetween)
        .cross_axis_alignment(druid::widget::CrossAxisAlignment::Baseline)
        .expand_width()
        .border(Color::GRAY, 0.5)
}

fn app_row_item(delete_selector: Selector<String>) -> impl Widget<AppEntry> {
    Flex::row()
        .with_flex_child(
            Label::dynamic(|e: &AppEntry, _| {
                std::path::Path::new(&e.name)
                    .file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or(&e.name)
                    .to_string()
            })
            .with_line_break_mode(LineBreaking::Clip)
            .align_left()
            .padding((4.0, 2.0)),
            1.0,
        )
        .with_child(
            Button::new("×")
                .fix_width(28.0)
                .on_click(move |ctx, data: &mut AppEntry, _| {
                    ctx.submit_command(delete_selector.with(data.name.clone()).to(Target::Global))
                }),
        )
        .cross_axis_alignment(druid::widget::CrossAxisAlignment::Center)
        .expand_width()
        .border(Color::GRAY, 0.5)
}

pub fn app_settings_ui_builder() -> impl Widget<UIDataAdapter> {
    Flex::column()
        .cross_axis_alignment(druid::widget::CrossAxisAlignment::Start)
        .main_axis_alignment(druid::widget::MainAxisAlignment::Start)
        .with_child(
            Flex::row()
                .with_child(Label::new("Danh sách ứng dụng"))
                .main_axis_alignment(druid::widget::MainAxisAlignment::Center)
                .expand_width()
                .padding((0.0, 0.0, 0.0, 8.0)),
        )
        .with_flex_child(
            Flex::row()
                .cross_axis_alignment(druid::widget::CrossAxisAlignment::Start)
                // Vietnamese apps column
                .with_flex_child(
                    Flex::column()
                        .cross_axis_alignment(druid::widget::CrossAxisAlignment::Start)
                        .with_child(
                            Label::new("Ứng dụng tiếng Việt")
                                .padding((0.0, 0.0, 0.0, 4.0)),
                        )
                        .with_flex_child(
                            {
                                let mut scroll = Scroll::new(
                                    List::new(move || app_row_item(DELETE_VN_APP))
                                        .lens(UIDataAdapter::vn_apps)
                                        .expand_width(),
                                );
                                scroll.set_enabled_scrollbars(
                                    druid::scroll_component::ScrollbarsEnabled::Vertical,
                                );
                                scroll.set_horizontal_scroll_enabled(false);
                                scroll
                            }
                            .expand(),
                            1.0,
                        )
                        .with_default_spacer()
                        .with_child(
                            Flex::row()
                                .with_flex_child(
                                    TextBox::new()
                                        .with_placeholder("Tên ứng dụng")
                                        .expand_width()
                                        .lens(UIDataAdapter::new_vn_app),
                                    1.0,
                                )
                                .with_spacer(4.0)
                                .with_child(
                                    Button::new("...").fix_width(32.0).on_click(|_, _, _| {
                                        defer_open_app_file_picker(Box::new(|name| {
                                            if let Some(name) = name {
                                                if let Some(sink) = UI_EVENT_SINK.get() {
                                                    let _ = sink.submit_command(
                                                        SET_VN_APP_FROM_PICKER,
                                                        name,
                                                        Target::Auto,
                                                    );
                                                }
                                            }
                                        }));
                                    }),
                                )
                                .with_spacer(4.0)
                                .with_child(
                                    Button::new("Thêm").on_click(|ctx, _, _| {
                                        ctx.submit_command(ADD_VN_APP.to(Target::Global))
                                    }),
                                )
                                .expand_width(),
                        )
                        .expand()
                        .padding(4.0),
                    1.0,
                )
                .with_spacer(8.0)
                // English apps column
                .with_flex_child(
                    Flex::column()
                        .cross_axis_alignment(druid::widget::CrossAxisAlignment::Start)
                        .with_child(
                            Label::new("Ứng dụng tiếng Anh")
                                .padding((0.0, 0.0, 0.0, 4.0)),
                        )
                        .with_flex_child(
                            {
                                let mut scroll = Scroll::new(
                                    List::new(move || app_row_item(DELETE_EN_APP))
                                        .lens(UIDataAdapter::en_apps)
                                        .expand_width(),
                                );
                                scroll.set_enabled_scrollbars(
                                    druid::scroll_component::ScrollbarsEnabled::Vertical,
                                );
                                scroll.set_horizontal_scroll_enabled(false);
                                scroll
                            }
                            .expand(),
                            1.0,
                        )
                        .with_default_spacer()
                        .with_child(
                            Flex::row()
                                .with_flex_child(
                                    TextBox::new()
                                        .with_placeholder("Tên ứng dụng")
                                        .expand_width()
                                        .lens(UIDataAdapter::new_en_app),
                                    1.0,
                                )
                                .with_spacer(4.0)
                                .with_child(
                                    Button::new("...").fix_width(32.0).on_click(|_, _, _| {
                                        defer_open_app_file_picker(Box::new(|name| {
                                            if let Some(name) = name {
                                                if let Some(sink) = UI_EVENT_SINK.get() {
                                                    let _ = sink.submit_command(
                                                        SET_EN_APP_FROM_PICKER,
                                                        name,
                                                        Target::Auto,
                                                    );
                                                }
                                            }
                                        }));
                                    }),
                                )
                                .with_spacer(4.0)
                                .with_child(
                                    Button::new("Thêm").on_click(|ctx, _, _| {
                                        ctx.submit_command(ADD_EN_APP.to(Target::Global))
                                    }),
                                )
                                .expand_width(),
                        )
                        .expand()
                        .padding(4.0),
                    1.0,
                )
                .expand(),
            1.0,
        )
        .with_child(
            Flex::row()
                .with_child(
                    Button::new("Đóng")
                        .on_click(|ctx, _, _| ctx.window().close())
                        .fix_width(100.0)
                        .fix_height(28.0),
                )
                .main_axis_alignment(druid::widget::MainAxisAlignment::End)
                .expand_width()
                .padding(6.0),
        )
        .must_fill_main_axis(true)
        .expand()
        .padding(8.0)
}

pub fn center_window_position() -> (f64, f64) {
    let screen_rect = Screen::get_display_rect();

    let x = (screen_rect.width() - WINDOW_WIDTH) / 2.0;
    let y = (screen_rect.height() - WINDOW_HEIGHT) / 2.0;

    (x, y)
}
