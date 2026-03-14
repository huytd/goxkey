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
    kurbo::{BezPath, Circle, Line, RoundedRect},
    piet::{FontFamily, Text, TextLayout, TextLayoutBuilder},
    theme,
    widget::{
        Button, Container, Controller, FillStrat, Flex, Image, Label, LineBreaking, List,
        Painter, Scroll, TextBox, ViewSwitcher,
    },
    Application, BoxConstraints, Color, Data, Env, Event, EventCtx, ImageBuf, LayoutCtx, Lens,
    LifeCycle, LifeCycleCtx, PaintCtx, Point, Rect, RenderContext, Screen, Selector, Size,
    Target, UpdateCtx, Widget, WidgetExt, WidgetPod, WindowDesc,
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
const DELETE_SELECTED_APP: Selector = Selector::new("gox-ui.delete-selected-app");
pub const WINDOW_WIDTH: f64 = 480.0;
pub const WINDOW_HEIGHT: f64 = 620.0;

// ── Design tokens ──────────────────────────────────────────────────────────────
const GREEN: Color = Color::rgb8(26, 138, 110);
const GREEN_BG: Color = Color::rgba8(26, 138, 110, 20);
const CARD_BG: Color = Color::rgb8(245, 245, 245);
const CARD_BORDER: Color = Color::rgba8(0, 0, 0, 30);
const DIVIDER: Color = Color::rgba8(0, 0, 0, 20);
const TEXT_PRIMARY: Color = Color::rgb8(17, 17, 17);
const TEXT_SECONDARY: Color = Color::rgb8(102, 102, 102);
const TEXT_SECTION: Color = Color::rgb8(153, 153, 153);
const BADGE_BG: Color = Color::rgb8(255, 255, 255);
const BADGE_BORDER: Color = Color::rgb8(204, 204, 204);
const BTN_RESET_BG: Color = Color::rgb8(240, 240, 240);
const BTN_RESET_BORDER: Color = Color::rgb8(204, 204, 204);
const WIN_BG: Color = Color::rgb8(255, 255, 255);

// ── Helpers ────────────────────────────────────────────────────────────────────

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

// ── Data structures ────────────────────────────────────────────────────────────

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
    // Tab navigation (0=General, 1=Apps, 2=Shortcuts, 3=Advanced)
    active_tab: u32,
    // Apps tab selected row (combined vn+en list, -1 = none)
    selected_app_index: i32,
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
            active_tab: 0,
            selected_app_index: -1,
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
                            TypingMethod::TelexVNI => "go+",
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
                    self.systray
                        .set_menu_item_title(SystemTrayMenuItemKey::TypingMethodTelexVNI, "Telex+VNI");
                }
                TypingMethod::Telex => {
                    self.systray
                        .set_menu_item_title(SystemTrayMenuItemKey::TypingMethodTelex, "Telex ✓");
                    self.systray
                        .set_menu_item_title(SystemTrayMenuItemKey::TypingMethodVNI, "VNI");
                    self.systray
                        .set_menu_item_title(SystemTrayMenuItemKey::TypingMethodTelexVNI, "Telex+VNI");
                }
                TypingMethod::TelexVNI => {
                    self.systray
                        .set_menu_item_title(SystemTrayMenuItemKey::TypingMethodTelex, "Telex");
                    self.systray
                        .set_menu_item_title(SystemTrayMenuItemKey::TypingMethodVNI, "VNI");
                    self.systray
                        .set_menu_item_title(SystemTrayMenuItemKey::TypingMethodTelexVNI, "Telex+VNI ✓");
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
            .set_menu_item_callback(SystemTrayMenuItemKey::TypingMethodTelexVNI, || {
                unsafe {
                    INPUT_STATE.set_method(TypingMethod::TelexVNI);
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

// ── UIController ───────────────────────────────────────────────────────────────

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
                    // In the new Apps tab design, adding via picker immediately commits
                    unsafe { INPUT_STATE.add_english_app(&data.new_en_app.clone()) };
                    data.new_en_app = String::new();
                    data.update();
                }
                if cmd.get(DELETE_SELECTED_APP).is_some() {
                    let idx = data.selected_app_index;
                    if idx >= 0 {
                        let vn_len = data.vn_apps.len() as i32;
                        if idx < vn_len {
                            if let Some(entry) = data.vn_apps.get(idx as usize) {
                                let name = entry.name.clone();
                                unsafe { INPUT_STATE.remove_vietnamese_app(&name) };
                            }
                        } else {
                            let en_idx = (idx - vn_len) as usize;
                            if let Some(entry) = data.en_apps.get(en_idx) {
                                let name = entry.name.clone();
                                unsafe { INPUT_STATE.remove_english_app(&name) };
                            }
                        }
                        data.selected_app_index = -1;
                        data.update();
                    }
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
        ctx: &mut UpdateCtx,
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

// ── LetterKeyController ────────────────────────────────────────────────────────

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

// ══════════════════════════════════════════════════════════════════════════════
// Custom widget: ToggleSwitch
// ══════════════════════════════════════════════════════════════════════════════

struct ToggleSwitch;

impl Widget<bool> for ToggleSwitch {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut bool, _env: &Env) {
        match event {
            Event::MouseDown(_) => {
                ctx.set_active(true);
                ctx.request_paint();
            }
            Event::MouseUp(_) => {
                if ctx.is_active() {
                    ctx.set_active(false);
                    *data = !*data;
                    ctx.request_paint();
                }
            }
            _ => {}
        }
    }

    fn lifecycle(&mut self, _ctx: &mut LifeCycleCtx, _event: &LifeCycle, _data: &bool, _env: &Env) {}

    fn update(&mut self, ctx: &mut UpdateCtx, old_data: &bool, data: &bool, _env: &Env) {
        if old_data != data {
            ctx.request_paint();
        }
    }

    fn layout(&mut self, _ctx: &mut LayoutCtx, _bc: &BoxConstraints, _data: &bool, _env: &Env) -> Size {
        Size::new(36.0, 20.0)
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &bool, _env: &Env) {
        let size = ctx.size();
        let radius = size.height / 2.0;
        let track_rect = RoundedRect::new(0.0, 0.0, size.width, size.height, radius);
        let track_color = if *data { GREEN } else { Color::rgb8(187, 187, 187) };
        ctx.fill(track_rect, &track_color);

        let knob_r = radius - 2.0;
        let knob_x = if *data {
            size.width - radius
        } else {
            radius
        };
        let knob_center = Point::new(knob_x, size.height / 2.0);
        ctx.fill(Circle::new(knob_center, knob_r), &Color::WHITE);
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// Custom widget: StyledCheckbox
// ══════════════════════════════════════════════════════════════════════════════

struct StyledCheckbox;

impl Widget<bool> for StyledCheckbox {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut bool, _env: &Env) {
        match event {
            Event::MouseDown(_) => {
                ctx.set_active(true);
                ctx.request_paint();
            }
            Event::MouseUp(_) => {
                if ctx.is_active() {
                    ctx.set_active(false);
                    *data = !*data;
                    ctx.request_paint();
                }
            }
            _ => {}
        }
    }

    fn lifecycle(&mut self, _ctx: &mut LifeCycleCtx, _event: &LifeCycle, _data: &bool, _env: &Env) {}

    fn update(&mut self, ctx: &mut UpdateCtx, old_data: &bool, data: &bool, _env: &Env) {
        if old_data != data {
            ctx.request_paint();
        }
    }

    fn layout(&mut self, _ctx: &mut LayoutCtx, _bc: &BoxConstraints, _data: &bool, _env: &Env) -> Size {
        Size::new(18.0, 18.0)
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &bool, _env: &Env) {
        let box_rect = RoundedRect::new(1.0, 1.0, 17.0, 17.0, 4.0);
        if *data {
            ctx.fill(box_rect, &GREEN);
            // Draw white checkmark: M4,9 L7,12 L13,6
            let mut path = BezPath::new();
            path.move_to((3.5, 9.0));
            path.line_to((7.0, 12.5));
            path.line_to((14.0, 5.5));
            ctx.stroke(path, &Color::WHITE, 1.8);
        } else {
            ctx.fill(box_rect, &Color::WHITE);
            ctx.stroke(box_rect, &Color::rgb8(204, 204, 204), 1.0);
        }
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// Custom widget: SegmentedControl
// ══════════════════════════════════════════════════════════════════════════════

struct SegmentedControl {
    options: Vec<(String, TypingMethod)>,
    rects: Vec<Rect>,
}

impl SegmentedControl {
    fn new(options: Vec<(&str, TypingMethod)>) -> Self {
        Self {
            options: options.into_iter().map(|(s, m)| (s.to_string(), m)).collect(),
            rects: Vec::new(),
        }
    }
}

impl Widget<TypingMethod> for SegmentedControl {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut TypingMethod, _env: &Env) {
        if let Event::MouseDown(mouse) = event {
            for (i, rect) in self.rects.iter().enumerate() {
                if rect.contains(mouse.pos) {
                    *data = self.options[i].1;
                    ctx.request_paint();
                    break;
                }
            }
        }
    }

    fn lifecycle(&mut self, _ctx: &mut LifeCycleCtx, _event: &LifeCycle, _data: &TypingMethod, _env: &Env) {}

    fn update(&mut self, ctx: &mut UpdateCtx, old_data: &TypingMethod, data: &TypingMethod, _env: &Env) {
        if old_data != data {
            ctx.request_paint();
        }
    }

    fn layout(&mut self, _ctx: &mut LayoutCtx, bc: &BoxConstraints, _data: &TypingMethod, _env: &Env) -> Size {
        let w = bc.max().width;
        let h = 34.0;
        let n = self.options.len() as f64;
        let gap = 8.0;
        let total_gap = gap * (n - 1.0);
        let btn_w = (w - total_gap) / n;
        self.rects = (0..self.options.len())
            .map(|i| {
                let x = i as f64 * (btn_w + gap);
                Rect::new(x, 0.0, x + btn_w, h)
            })
            .collect();
        Size::new(w, h)
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &TypingMethod, _env: &Env) {
        for (i, (label, method)) in self.options.iter().enumerate() {
            let rect = self.rects[i];
            let is_active = method == data;
            let rr = RoundedRect::new(rect.x0, rect.y0, rect.x1, rect.y1, 8.0);

            if is_active {
                ctx.fill(rr, &GREEN_BG);
                ctx.stroke(rr, &GREEN, 1.5);
            } else {
                ctx.fill(rr, &Color::WHITE);
                ctx.stroke(rr, &Color::rgb8(221, 221, 221), 0.5);
            }

            // Radio dot
            let dot_cx = rect.x0 + 14.0;
            let dot_cy = rect.y0 + rect.height() / 2.0;
            let outer = Circle::new((dot_cx, dot_cy), 5.0);
            let ring_color = if is_active { GREEN } else { Color::rgb8(187, 187, 187) };
            ctx.stroke(outer, &ring_color, 1.5);
            if is_active {
                ctx.fill(Circle::new((dot_cx, dot_cy), 2.5), &GREEN);
            }

            // Label text
            let text_color = if is_active { GREEN } else { Color::rgb8(136, 136, 136) };
            let font_size = 13.0;
            let layout = ctx
                .text()
                .new_text_layout(label.clone())
                .font(FontFamily::SYSTEM_UI, font_size)
                .text_color(text_color)
                .build()
                .unwrap();
            let text_w = layout.size().width;
            let text_h = layout.size().height;
            let text_x = rect.x0 + (rect.width() - text_w + 14.0) / 2.0 + 7.0;
            let text_y = rect.y0 + (rect.height() - text_h) / 2.0;
            ctx.draw_text(&layout, (text_x, text_y));
        }
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// Custom widget: TabBar
// ══════════════════════════════════════════════════════════════════════════════

struct TabBar {
    tab_rects: Vec<Rect>,
}

impl TabBar {
    fn new() -> Self {
        Self { tab_rects: Vec::new() }
    }

    fn draw_icon_general(ctx: &mut PaintCtx, cx: f64, cy: f64, color: &Color) {
        // Three horizontal bars (hamburger/list icon)
        for (i, (w, y_off)) in [(14.0, -4.5), (9.0, 0.0), (11.0, 4.5)].iter().enumerate() {
            let _ = i;
            let rr = RoundedRect::new(cx - 7.0, cy + y_off - 1.5, cx - 7.0 + w, cy + y_off + 1.5, 1.5);
            ctx.fill(rr, color);
        }
    }

    fn draw_icon_apps(ctx: &mut PaintCtx, cx: f64, cy: f64, color: &Color) {
        // 2x2 grid of squares
        for (dx, dy) in [(-4.5, -4.5), (1.5, -4.5), (-4.5, 1.5), (1.5, 1.5)] {
            let rr = RoundedRect::new(cx + dx, cy + dy, cx + dx + 4.5, cy + dy + 4.5, 1.0);
            ctx.fill(rr, color);
        }
    }

    fn draw_icon_shortcuts(ctx: &mut PaintCtx, cx: f64, cy: f64, color: &Color) {
        // Keyboard outline rect
        let rr = RoundedRect::new(cx - 8.0, cy - 5.0, cx + 8.0, cy + 5.0, 2.0);
        ctx.stroke(rr, color, 1.5);
        // Two small key rects inside
        let k1 = RoundedRect::new(cx - 6.0, cy - 1.5, cx - 2.0, cy + 1.5, 1.0);
        let k2 = RoundedRect::new(cx + 2.0, cy - 1.5, cx + 6.0, cy + 1.5, 1.0);
        ctx.fill(k1, color);
        ctx.fill(k2, color);
    }

    fn draw_icon_advanced(ctx: &mut PaintCtx, cx: f64, cy: f64, color: &Color) {
        // Clock circle
        ctx.stroke(Circle::new((cx, cy), 7.0), color, 1.5);
        // Clock hands
        let mut hand = BezPath::new();
        hand.move_to((cx, cy));
        hand.line_to((cx, cy - 4.0));
        ctx.stroke(hand.clone(), color, 1.5);
        let mut hand2 = BezPath::new();
        hand2.move_to((cx, cy));
        hand2.line_to((cx + 3.0, cy + 2.0));
        ctx.stroke(hand2, color, 1.5);
    }
}

impl Widget<u32> for TabBar {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut u32, _env: &Env) {
        if let Event::MouseDown(mouse) = event {
            for (i, rect) in self.tab_rects.iter().enumerate() {
                if rect.contains(mouse.pos) {
                    *data = i as u32;
                    ctx.request_paint();
                    break;
                }
            }
        }
    }

    fn lifecycle(&mut self, _ctx: &mut LifeCycleCtx, _event: &LifeCycle, _data: &u32, _env: &Env) {}

    fn update(&mut self, ctx: &mut UpdateCtx, old_data: &u32, data: &u32, _env: &Env) {
        if old_data != data {
            ctx.request_paint();
        }
    }

    fn layout(&mut self, _ctx: &mut LayoutCtx, bc: &BoxConstraints, _data: &u32, _env: &Env) -> Size {
        let w = bc.max().width;
        let h = 58.0;
        let tab_w = w / 4.0;
        self.tab_rects = (0..4)
            .map(|i| Rect::new(i as f64 * tab_w, 0.0, (i + 1) as f64 * tab_w, h))
            .collect();
        Size::new(w, h)
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &u32, _env: &Env) {
        let size = ctx.size();

        // Tab bar bottom border
        ctx.stroke(
            Line::new((0.0, size.height), (size.width, size.height)),
            &Color::rgb8(221, 221, 221),
            0.5,
        );

        let labels = ["General", "Apps", "Shortcuts", "Advanced"];
        let icon_fns: [fn(&mut PaintCtx, f64, f64, &Color); 4] = [
            TabBar::draw_icon_general,
            TabBar::draw_icon_apps,
            TabBar::draw_icon_shortcuts,
            TabBar::draw_icon_advanced,
        ];

        for (i, rect) in self.tab_rects.iter().enumerate() {
            let is_active = i as u32 == *data;
            let color = if is_active { GREEN } else { Color::rgb8(153, 153, 153) };
            let cx = rect.x0 + rect.width() / 2.0;
            let icon_cy = rect.y0 + 18.0;

            icon_fns[i](ctx, cx, icon_cy, &color);

            // Label
            let label = labels[i];
            let font_size = 10.0;
            let layout = ctx
                .text()
                .new_text_layout(label)
                .font(FontFamily::SYSTEM_UI, font_size)
                .text_color(color.clone())
                .build()
                .unwrap();
            let lw = layout.size().width;
            ctx.draw_text(&layout, (cx - lw / 2.0, icon_cy + 11.0));

            // Active underline
            if is_active {
                ctx.fill(
                    Rect::new(rect.x0, size.height - 2.0, rect.x1, size.height),
                    &GREEN,
                );
            }
        }
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// Custom widget: KeyBadge
// ══════════════════════════════════════════════════════════════════════════════

struct KeyBadge {
    label: String,
}

impl KeyBadge {
    fn new(label: impl Into<String>) -> Self {
        Self { label: label.into() }
    }
}

impl Widget<()> for KeyBadge {
    fn event(&mut self, _ctx: &mut EventCtx, _event: &Event, _data: &mut (), _env: &Env) {}
    fn lifecycle(&mut self, _ctx: &mut LifeCycleCtx, _event: &LifeCycle, _data: &(), _env: &Env) {}
    fn update(&mut self, _ctx: &mut UpdateCtx, _old: &(), _data: &(), _env: &Env) {}

    fn layout(&mut self, _ctx: &mut LayoutCtx, _bc: &BoxConstraints, _data: &(), _env: &Env) -> Size {
        let char_w = self.label.chars().count() as f64 * 8.0;
        Size::new((char_w + 14.0).max(26.0), 24.0)
    }

    fn paint(&mut self, ctx: &mut PaintCtx, _data: &(), _env: &Env) {
        let size = ctx.size();
        let rr = RoundedRect::new(0.0, 0.0, size.width, size.height, 5.0);
        ctx.fill(rr, &BADGE_BG);
        ctx.stroke(rr, &BADGE_BORDER, 0.5);

        let layout = ctx
            .text()
            .new_text_layout(self.label.clone())
            .font(FontFamily::SYSTEM_UI, 12.0)
            .text_color(Color::rgb8(85, 85, 85))
            .build()
            .unwrap();
        let lw = layout.size().width;
        let lh = layout.size().height;
        ctx.draw_text(&layout, ((size.width - lw) / 2.0, (size.height - lh) / 2.0));
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// Layout helpers
// ══════════════════════════════════════════════════════════════════════════════

fn section_label(text: &'static str) -> impl Widget<UIDataAdapter> {
    Painter::new(move |ctx, _data: &UIDataAdapter, _env| {
        let size = ctx.size();
        let layout = ctx
            .text()
            .new_text_layout(text.to_uppercase())
            .font(FontFamily::SYSTEM_UI, 11.0)
            .text_color(TEXT_SECTION)
            .build()
            .unwrap();
        ctx.draw_text(&layout, (0.0, (size.height - layout.size().height) / 2.0));
    })
    .fix_height(18.0)
    .expand_width()
    .padding((0.0, 0.0, 0.0, 6.0))
}

/// A horizontal divider line inside a card
fn card_divider() -> impl Widget<UIDataAdapter> {
    Painter::new(|ctx, _data: &UIDataAdapter, _env| {
        let size = ctx.size();
        ctx.fill(Rect::new(14.0, 0.0, size.width - 14.0, 0.5), &DIVIDER);
    })
    .fix_height(0.5)
    .expand_width()
}

/// Row: bold title + gray subtitle on the left, trailing widget on the right
fn settings_row<TW: Widget<UIDataAdapter> + 'static>(
    title: &'static str,
    subtitle: &'static str,
    trailing: TW,
) -> impl Widget<UIDataAdapter> {
    Flex::row()
        .with_flex_child(
            Flex::column()
                .cross_axis_alignment(druid::widget::CrossAxisAlignment::Start)
                .with_child(
                    Painter::new(move |ctx, _data: &UIDataAdapter, _env| {
                        let layout = ctx
                            .text()
                            .new_text_layout(title)
                            .font(FontFamily::SYSTEM_UI, 13.0)
                            .text_color(TEXT_PRIMARY)
                            .build()
                            .unwrap();
                        ctx.draw_text(&layout, (0.0, 0.0));
                    })
                    .fix_height(18.0)
                    .expand_width(),
                )
                .with_child(
                    Painter::new(move |ctx, _data: &UIDataAdapter, _env| {
                        let layout = ctx
                            .text()
                            .new_text_layout(subtitle)
                            .font(FontFamily::SYSTEM_UI, 12.0)
                            .text_color(TEXT_SECONDARY)
                            .build()
                            .unwrap();
                        ctx.draw_text(&layout, (0.0, 0.0));
                    })
                    .fix_height(16.0)
                    .expand_width(),
                ),
            1.0,
        )
        .with_child(trailing)
        .cross_axis_alignment(druid::widget::CrossAxisAlignment::Center)
        .main_axis_alignment(druid::widget::MainAxisAlignment::SpaceBetween)
        .must_fill_main_axis(true)
        .expand_width()
        .padding((14.0, 10.0))
}

fn settings_card<TW: Widget<UIDataAdapter> + 'static>(inner: TW) -> impl Widget<UIDataAdapter> {
    Container::new(inner)
        .background(CARD_BG)
        .border(CARD_BORDER, 0.5)
        .rounded(10.0)
}

// ══════════════════════════════════════════════════════════════════════════════
// Tab content builders
// ══════════════════════════════════════════════════════════════════════════════

fn general_tab() -> impl Widget<UIDataAdapter> {
    // INPUT MODE card
    let input_mode_card = settings_card(
        Flex::column()
            .with_child(
                // Vietnamese input toggle row
                Flex::row()
                    .with_flex_child(
                        Flex::column()
                            .cross_axis_alignment(druid::widget::CrossAxisAlignment::Start)
                            .with_child(
                                Painter::new(|ctx, _data: &UIDataAdapter, _env| {
                                    let layout = ctx
                                        .text()
                                        .new_text_layout("Vietnamese input")
                                        .font(FontFamily::SYSTEM_UI, 13.0)
                                        .text_color(TEXT_PRIMARY)
                                        .build()
                                        .unwrap();
                                    ctx.draw_text(&layout, (0.0, 0.0));
                                })
                                .fix_height(18.0)
                                .expand_width(),
                            )
                            .with_child(
                                Painter::new(|ctx, _data: &UIDataAdapter, _env| {
                                    let layout = ctx
                                        .text()
                                        .new_text_layout("Enable Vietnamese typing mode")
                                        .font(FontFamily::SYSTEM_UI, 12.0)
                                        .text_color(TEXT_SECONDARY)
                                        .build()
                                        .unwrap();
                                    ctx.draw_text(&layout, (0.0, 0.0));
                                })
                                .fix_height(16.0)
                                .expand_width(),
                            ),
                        1.0,
                    )
                    .with_child(
                        ToggleSwitch
                            .lens(UIDataAdapter::is_enabled)
                            .on_click(|_, data: &mut UIDataAdapter, _| {
                                data.toggle_vietnamese();
                            }),
                    )
                    .cross_axis_alignment(druid::widget::CrossAxisAlignment::Center)
                    .main_axis_alignment(druid::widget::MainAxisAlignment::SpaceBetween)
                    .must_fill_main_axis(true)
                    .expand_width()
                    .padding((14.0, 10.0)),
            )
            .with_child(card_divider())
            .with_child(
                // Input method segmented control
                Flex::column()
                    .cross_axis_alignment(druid::widget::CrossAxisAlignment::Start)
                    .with_child(
                        Painter::new(|ctx, _data: &UIDataAdapter, _env| {
                            let layout = ctx
                                .text()
                                .new_text_layout("Input method")
                                .font(FontFamily::SYSTEM_UI, 13.0)
                                .text_color(TEXT_PRIMARY)
                                .build()
                                .unwrap();
                            ctx.draw_text(&layout, (0.0, 0.0));
                        })
                        .fix_height(18.0)
                        .expand_width(),
                    )
                    .with_spacer(8.0)
                    .with_child(
                        SegmentedControl::new(vec![
                            ("Telex", TypingMethod::Telex),
                            ("VNI", TypingMethod::VNI),
                            ("Telex + VNI", TypingMethod::TelexVNI),
                        ])
                        .lens(UIDataAdapter::typing_method)
                        .expand_width(),
                    )
                    .expand_width()
                    .padding((14.0, 10.0)),
            ),
    );

    // SYSTEM card
    let system_card = settings_card(
        Flex::column()
            .with_child(settings_row(
                "Launch at login",
                "Start gõkey when you log in",
                StyledCheckbox.lens(UIDataAdapter::launch_on_login),
            ))
            .with_child(card_divider())
            .with_child(settings_row(
                "Per-app toggle",
                "Enable/disable per application",
                StyledCheckbox.lens(UIDataAdapter::is_auto_toggle_enabled),
            )),
    );

    // SHORTCUT card — key badges built from hotkey_display
    let shortcut_card = settings_card(
        Flex::row()
            .with_flex_child(
                Flex::column()
                    .cross_axis_alignment(druid::widget::CrossAxisAlignment::Start)
                    .with_child(
                        Painter::new(|ctx, _data: &UIDataAdapter, _env| {
                            let layout = ctx
                                .text()
                                .new_text_layout("Toggle Vietnamese input")
                                .font(FontFamily::SYSTEM_UI, 13.0)
                                .text_color(TEXT_PRIMARY)
                                .build()
                                .unwrap();
                            ctx.draw_text(&layout, (0.0, 0.0));
                        })
                        .fix_height(18.0)
                        .expand_width(),
                    )
                    .with_child(
                        Painter::new(|ctx, _data: &UIDataAdapter, _env| {
                            let layout = ctx
                                .text()
                                .new_text_layout("Keyboard shortcut to toggle on/off")
                                .font(FontFamily::SYSTEM_UI, 12.0)
                                .text_color(TEXT_SECONDARY)
                                .build()
                                .unwrap();
                            ctx.draw_text(&layout, (0.0, 0.0));
                        })
                        .fix_height(16.0)
                        .expand_width(),
                    ),
                1.0,
            )
            .with_child(HotkeyBadgesWidget::new())
            .cross_axis_alignment(druid::widget::CrossAxisAlignment::Center)
            .main_axis_alignment(druid::widget::MainAxisAlignment::SpaceBetween)
            .must_fill_main_axis(true)
            .expand_width()
            .padding((14.0, 10.0)),
    );

    // Footer buttons
    let footer = Flex::row()
        .with_flex_spacer(1.0)
        .with_child(
            Painter::new(|ctx, _data: &UIDataAdapter, _env| {
                let size = ctx.size();
                let rr = RoundedRect::new(0.0, 0.0, size.width, size.height, 7.0);
                ctx.fill(rr, &BTN_RESET_BG);
                ctx.stroke(rr, &BTN_RESET_BORDER, 0.5);
                let layout = ctx
                    .text()
                    .new_text_layout("Reset defaults")
                    .font(FontFamily::SYSTEM_UI, 13.0)
                    .text_color(Color::rgb8(51, 51, 51))
                    .build()
                    .unwrap();
                let lw = layout.size().width;
                let lh = layout.size().height;
                ctx.draw_text(&layout, ((size.width - lw) / 2.0, (size.height - lh) / 2.0));
            })
            .fix_size(120.0, 30.0),
        )
        .with_spacer(8.0)
        .with_child(
            Painter::new(|ctx, _data: &UIDataAdapter, _env| {
                let size = ctx.size();
                let rr = RoundedRect::new(0.0, 0.0, size.width, size.height, 7.0);
                ctx.fill(rr, &GREEN);
                let layout = ctx
                    .text()
                    .new_text_layout("Done")
                    .font(FontFamily::SYSTEM_UI, 13.0)
                    .text_color(Color::WHITE)
                    .build()
                    .unwrap();
                let lw = layout.size().width;
                let lh = layout.size().height;
                ctx.draw_text(&layout, ((size.width - lw) / 2.0, (size.height - lh) / 2.0));
            })
            .fix_size(70.0, 30.0)
            .on_click(|ctx, _data: &mut UIDataAdapter, _env| {
                ctx.window().hide();
            }),
        )
        .expand_width();

    Flex::column()
        .cross_axis_alignment(druid::widget::CrossAxisAlignment::Start)
        .with_child(section_label("Input mode"))
        .with_child(input_mode_card)
        .with_spacer(20.0)
        .with_child(section_label("System"))
        .with_child(system_card)
        .with_spacer(20.0)
        .with_child(section_label("Shortcut"))
        .with_child(shortcut_card)
        .with_flex_spacer(1.0)
        .with_child(footer)
        .padding((24.0, 20.0, 24.0, 24.0))
}

// ══════════════════════════════════════════════════════════════════════════════
// Custom widget: AppsListWidget — unified VN + EN app list
// ══════════════════════════════════════════════════════════════════════════════

/// A combined entry for the merged app list
#[derive(Clone)]
struct CombinedAppEntry {
    display_name: String,
    full_name: String,
    is_vn: bool,
}

struct AppsListWidget {
    row_rects: Vec<Rect>,
    /// Avatar background colors cycling through a small palette
    avatar_colors: Vec<Color>,
}

impl AppsListWidget {
    fn new() -> Self {
        Self {
            row_rects: Vec::new(),
            avatar_colors: vec![
                Color::rgb8(196, 60, 48),   // red
                Color::rgb8(72, 163, 101),  // green
                Color::rgb8(58, 115, 199),  // blue
                Color::rgb8(133, 86, 178),  // purple
                Color::rgb8(203, 131, 46),  // orange
            ],
        }
    }

    fn build_entries(data: &UIDataAdapter) -> Vec<CombinedAppEntry> {
        let mut entries: Vec<CombinedAppEntry> = data
            .vn_apps
            .iter()
            .map(|e| CombinedAppEntry {
                display_name: std::path::Path::new(&e.name)
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or(&e.name)
                    .to_string(),
                full_name: e.name.clone(),
                is_vn: true,
            })
            .collect();
        for e in data.en_apps.iter() {
            entries.push(CombinedAppEntry {
                display_name: std::path::Path::new(&e.name)
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or(&e.name)
                    .to_string(),
                full_name: e.name.clone(),
                is_vn: false,
            });
        }
        entries
    }

    fn initials(name: &str) -> String {
        // Take first two uppercase chars of the display name
        let mut chars = name.chars().filter(|c| c.is_alphabetic());
        let first = chars.next().map(|c| c.to_ascii_uppercase()).unwrap_or('?');
        let second = chars.next().map(|c| c.to_ascii_uppercase());
        if let Some(s) = second {
            format!("{}{}", first, s)
        } else {
            format!("{}", first)
        }
    }
}

const ROW_HEIGHT: f64 = 52.0;
// Badge colors
const BADGE_VI_BG: Color = Color::rgba8(26, 138, 110, 20);
const BADGE_VI_BORDER: Color = Color::rgb8(26, 138, 110);
const BADGE_EN_BG: Color = Color::rgba8(58, 115, 199, 18);
const BADGE_EN_BORDER: Color = Color::rgb8(58, 115, 199);

impl Widget<UIDataAdapter> for AppsListWidget {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut UIDataAdapter, _env: &Env) {
        if let Event::MouseDown(mouse) = event {
            for (i, rect) in self.row_rects.iter().enumerate() {
                if rect.contains(mouse.pos) {
                    data.selected_app_index = i as i32;
                    ctx.request_paint();
                    break;
                }
            }
        }
    }

    fn lifecycle(&mut self, _ctx: &mut LifeCycleCtx, _event: &LifeCycle, _data: &UIDataAdapter, _env: &Env) {}

    fn update(&mut self, ctx: &mut UpdateCtx, old_data: &UIDataAdapter, data: &UIDataAdapter, _env: &Env) {
        if old_data.vn_apps != data.vn_apps
            || old_data.en_apps != data.en_apps
            || old_data.selected_app_index != data.selected_app_index
        {
            ctx.request_paint();
        }
    }

    fn layout(&mut self, _ctx: &mut LayoutCtx, bc: &BoxConstraints, data: &UIDataAdapter, _env: &Env) -> Size {
        let entries = Self::build_entries(data);
        let w = bc.max().width;
        self.row_rects = entries
            .iter()
            .enumerate()
            .map(|(i, _)| Rect::new(0.0, i as f64 * ROW_HEIGHT, w, (i + 1) as f64 * ROW_HEIGHT))
            .collect();
        let h = (entries.len() as f64 * ROW_HEIGHT).max(0.0);
        Size::new(w, h)
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &UIDataAdapter, _env: &Env) {
        let entries = Self::build_entries(data);
        let size = ctx.size();

        for (i, entry) in entries.iter().enumerate() {
            let rect = self.row_rects[i];
            let is_selected = data.selected_app_index == i as i32;

            // Row background highlight if selected
            if is_selected {
                ctx.fill(
                    RoundedRect::new(rect.x0, rect.y0, rect.x1, rect.y1, 0.0),
                    &Color::rgba8(0, 0, 0, 8),
                );
            }

            // Divider between rows
            if i > 0 {
                ctx.fill(
                    Rect::new(54.0, rect.y0, size.width - 14.0, rect.y0 + 0.5),
                    &DIVIDER,
                );
            }

            // App icon avatar (colored square with initials)
            let avatar_x = 14.0;
            let avatar_y = rect.y0 + (ROW_HEIGHT - 36.0) / 2.0;
            let avatar_rect = RoundedRect::new(avatar_x, avatar_y, avatar_x + 36.0, avatar_y + 36.0, 8.0);
            let color = &self.avatar_colors[i % self.avatar_colors.len()];
            ctx.fill(avatar_rect, color);

            // Initials text
            let initials = Self::initials(&entry.display_name);
            let init_layout = ctx
                .text()
                .new_text_layout(initials)
                .font(FontFamily::SYSTEM_UI, 13.0)
                .text_color(Color::WHITE)
                .build()
                .unwrap();
            let iw = init_layout.size().width;
            let ih = init_layout.size().height;
            ctx.draw_text(&init_layout, (avatar_x + (36.0 - iw) / 2.0, avatar_y + (36.0 - ih) / 2.0));

            // App name
            let name_layout = ctx
                .text()
                .new_text_layout(entry.display_name.clone())
                .font(FontFamily::SYSTEM_UI, 14.0)
                .text_color(TEXT_PRIMARY)
                .build()
                .unwrap();
            let name_y = rect.y0 + (ROW_HEIGHT - name_layout.size().height) / 2.0;
            ctx.draw_text(&name_layout, (60.0, name_y));

            // Language badge (VI or EN)
            let badge_label = if entry.is_vn { "VI" } else { "EN" };
            let badge_bg = if entry.is_vn { BADGE_VI_BG } else { BADGE_EN_BG };
            let badge_border = if entry.is_vn { BADGE_VI_BORDER } else { BADGE_EN_BORDER };
            let badge_text_color = if entry.is_vn { BADGE_VI_BORDER } else { BADGE_EN_BORDER };

            let badge_layout = ctx
                .text()
                .new_text_layout(badge_label)
                .font(FontFamily::SYSTEM_UI, 11.0)
                .text_color(badge_text_color)
                .build()
                .unwrap();
            let bw = badge_layout.size().width + 14.0;
            let bh = 22.0;
            let badge_x = size.width - bw - 14.0;
            let badge_y = rect.y0 + (ROW_HEIGHT - bh) / 2.0;
            let badge_rr = RoundedRect::new(badge_x, badge_y, badge_x + bw, badge_y + bh, 5.0);
            ctx.fill(badge_rr, &badge_bg);
            ctx.stroke(badge_rr, &badge_border, 1.0);
            let bl_w = badge_layout.size().width;
            let bl_h = badge_layout.size().height;
            ctx.draw_text(&badge_layout, (badge_x + (bw - bl_w) / 2.0, badge_y + (bh - bl_h) / 2.0));
        }
    }
}

fn apps_tab() -> impl Widget<UIDataAdapter> {
    // Description text
    let description = Painter::new(|ctx, _: &UIDataAdapter, _| {
        let layout = ctx
            .text()
            .new_text_layout("Set input language per application.")
            .font(FontFamily::SYSTEM_UI, 13.0)
            .text_color(TEXT_PRIMARY)
            .build()
            .unwrap();
        ctx.draw_text(&layout, (0.0, 0.0));
    })
    .fix_height(18.0)
    .expand_width();

    // Legend: VI badge + "Vietnamese"  EN badge + "English"
    let legend = Painter::new(|ctx, _: &UIDataAdapter, _| {
        let mut x = 0.0;

        // VI badge
        let vi_layout = ctx
            .text()
            .new_text_layout("VI")
            .font(FontFamily::SYSTEM_UI, 11.0)
            .text_color(BADGE_VI_BORDER)
            .build()
            .unwrap();
        let bw = vi_layout.size().width + 14.0;
        let bh = 22.0;
        let badge_y = (26.0 - bh) / 2.0;
        let vi_rr = RoundedRect::new(x, badge_y, x + bw, badge_y + bh, 5.0);
        ctx.fill(vi_rr, &BADGE_VI_BG);
        ctx.stroke(vi_rr, &BADGE_VI_BORDER, 1.0);
        ctx.draw_text(&vi_layout, (x + (bw - vi_layout.size().width) / 2.0, badge_y + (bh - vi_layout.size().height) / 2.0));
        x += bw + 8.0;

        // "Vietnamese" label
        let vn_label = ctx
            .text()
            .new_text_layout("Vietnamese")
            .font(FontFamily::SYSTEM_UI, 13.0)
            .text_color(TEXT_PRIMARY)
            .build()
            .unwrap();
        ctx.draw_text(&vn_label, (x, (26.0 - vn_label.size().height) / 2.0));
        x += vn_label.size().width + 20.0;

        // EN badge
        let en_layout = ctx
            .text()
            .new_text_layout("EN")
            .font(FontFamily::SYSTEM_UI, 11.0)
            .text_color(BADGE_EN_BORDER)
            .build()
            .unwrap();
        let bw_en = en_layout.size().width + 14.0;
        let en_rr = RoundedRect::new(x, badge_y, x + bw_en, badge_y + bh, 5.0);
        ctx.fill(en_rr, &BADGE_EN_BG);
        ctx.stroke(en_rr, &BADGE_EN_BORDER, 1.0);
        ctx.draw_text(&en_layout, (x + (bw_en - en_layout.size().width) / 2.0, badge_y + (bh - en_layout.size().height) / 2.0));
        x += bw_en + 8.0;

        // "English" label
        let en_label = ctx
            .text()
            .new_text_layout("English")
            .font(FontFamily::SYSTEM_UI, 13.0)
            .text_color(TEXT_PRIMARY)
            .build()
            .unwrap();
        ctx.draw_text(&en_label, (x, (26.0 - en_label.size().height) / 2.0));
    })
    .fix_height(26.0)
    .expand_width();

    // Unified app list inside a card
    let app_list = {
        let mut scroll = Scroll::new(AppsListWidget::new().expand_width());
        scroll.set_enabled_scrollbars(druid::scroll_component::ScrollbarsEnabled::Vertical);
        scroll.set_horizontal_scroll_enabled(false);
        scroll
    };

    // Bottom toolbar: + and - buttons
    let toolbar = Painter::new(|ctx, _: &UIDataAdapter, _| {
        let size = ctx.size();
        // Top divider
        ctx.fill(Rect::new(0.0, 0.0, size.width, 0.5), &DIVIDER);
    })
    .fix_height(44.0)
    .expand_width();

    let add_btn = Painter::new(|ctx, _: &UIDataAdapter, _| {
        let size = ctx.size();
        // "+" symbol
        let layout = ctx
            .text()
            .new_text_layout("+")
            .font(FontFamily::SYSTEM_UI, 18.0)
            .text_color(TEXT_PRIMARY)
            .build()
            .unwrap();
        let lw = layout.size().width;
        let lh = layout.size().height;
        ctx.draw_text(&layout, ((size.width - lw) / 2.0, (size.height - lh) / 2.0));
    })
    .fix_size(44.0, 44.0)
    .on_click(|_, _, _| {
        defer_open_app_file_picker(Box::new(|name| {
            if let Some(name) = name {
                if let Some(sink) = UI_EVENT_SINK.get() {
                    let _ = sink.submit_command(SET_EN_APP_FROM_PICKER, name, Target::Auto);
                }
            }
        }));
    });

    let remove_btn = Painter::new(|ctx, data: &UIDataAdapter, _| {
        let size = ctx.size();
        let is_enabled = data.selected_app_index >= 0;
        let color = if is_enabled { TEXT_PRIMARY } else { Color::rgb8(187, 187, 187) };
        // Vertical divider before "-"
        ctx.fill(Rect::new(0.0, 10.0, 0.5, size.height - 10.0), &DIVIDER);
        // "−" symbol
        let layout = ctx
            .text()
            .new_text_layout("−")
            .font(FontFamily::SYSTEM_UI, 18.0)
            .text_color(color)
            .build()
            .unwrap();
        let lw = layout.size().width;
        let lh = layout.size().height;
        ctx.draw_text(&layout, ((size.width - lw) / 2.0 + 0.5, (size.height - lh) / 2.0));
    })
    .fix_size(44.0, 44.0)
    .on_click(|ctx, data: &mut UIDataAdapter, _| {
        if data.selected_app_index >= 0 {
            ctx.submit_command(DELETE_SELECTED_APP.to(Target::Global));
        }
    });

    let bottom_bar = Flex::row()
        .with_child(add_btn)
        .with_child(remove_btn)
        .with_flex_spacer(1.0)
        .expand_width();

    let card = Container::new(
        Flex::column()
            .with_flex_child(app_list.expand(), 1.0)
            .with_child(
                Painter::new(|ctx, _: &UIDataAdapter, _| {
                    let size = ctx.size();
                    ctx.fill(Rect::new(0.0, 0.0, size.width, 0.5), &DIVIDER);
                })
                .fix_height(0.5)
                .expand_width(),
            )
            .with_child(bottom_bar),
    )
    .background(CARD_BG)
    .border(CARD_BORDER, 0.5)
    .rounded(10.0);

    Flex::column()
        .cross_axis_alignment(druid::widget::CrossAxisAlignment::Start)
        .with_child(description)
        .with_spacer(12.0)
        .with_child(legend)
        .with_spacer(12.0)
        .with_flex_child(card.expand_height(), 1.0)
        .must_fill_main_axis(true)
        .expand()
        .padding((24.0, 20.0, 24.0, 24.0))
}

fn shortcuts_tab() -> impl Widget<UIDataAdapter> {
    let card = settings_card(
        Flex::column()
            .with_child(
                Flex::row()
                    .with_flex_child(
                        Flex::column()
                            .cross_axis_alignment(druid::widget::CrossAxisAlignment::Start)
                            .with_child(
                                Painter::new(|ctx, _: &UIDataAdapter, _| {
                                    let layout = ctx
                                        .text()
                                        .new_text_layout("Toggle Vietnamese input")
                                        .font(FontFamily::SYSTEM_UI, 13.0)
                                        .text_color(TEXT_PRIMARY)
                                        .build()
                                        .unwrap();
                                    ctx.draw_text(&layout, (0.0, 0.0));
                                })
                                .fix_height(18.0)
                                .expand_width(),
                            )
                            .with_child(
                                Painter::new(|ctx, data: &UIDataAdapter, _| {
                                    let layout = ctx
                                        .text()
                                        .new_text_layout(data.hotkey_display.clone())
                                        .font(FontFamily::SYSTEM_UI, 12.0)
                                        .text_color(TEXT_SECONDARY)
                                        .build()
                                        .unwrap();
                                    ctx.draw_text(&layout, (0.0, 0.0));
                                })
                                .fix_height(16.0)
                                .expand_width(),
                            ),
                        1.0,
                    )
                    .cross_axis_alignment(druid::widget::CrossAxisAlignment::Center)
                    .expand_width()
                    .padding((14.0, 10.0)),
            )
            .with_child(card_divider())
            .with_child(
                Flex::column()
                    .cross_axis_alignment(druid::widget::CrossAxisAlignment::Start)
                    .with_child(
                        Painter::new(|ctx, _: &UIDataAdapter, _| {
                            let layout = ctx
                                .text()
                                .new_text_layout("Modifiers")
                                .font(FontFamily::SYSTEM_UI, 12.0)
                                .text_color(TEXT_SECONDARY)
                                .build()
                                .unwrap();
                            ctx.draw_text(&layout, (0.0, 0.0));
                        })
                        .fix_height(16.0)
                        .expand_width(),
                    )
                    .with_spacer(8.0)
                    .with_child(
                        Flex::row()
                            .with_child(
                                StyledCheckbox
                                    .lens(UIDataAdapter::super_key)
                                    .padding((0.0, 0.0, 6.0, 0.0)),
                            )
                            .with_child(
                                Painter::new(|ctx, _: &UIDataAdapter, _| {
                                    let layout = ctx
                                        .text()
                                        .new_text_layout(SYMBOL_SUPER)
                                        .font(FontFamily::SYSTEM_UI, 13.0)
                                        .text_color(TEXT_PRIMARY)
                                        .build()
                                        .unwrap();
                                    ctx.draw_text(&layout, (0.0, 0.0));
                                })
                                .fix_height(20.0)
                                .fix_width(24.0),
                            )
                            .with_spacer(12.0)
                            .with_child(
                                StyledCheckbox
                                    .lens(UIDataAdapter::ctrl_key)
                                    .padding((0.0, 0.0, 6.0, 0.0)),
                            )
                            .with_child(
                                Painter::new(|ctx, _: &UIDataAdapter, _| {
                                    let layout = ctx
                                        .text()
                                        .new_text_layout(SYMBOL_CTRL)
                                        .font(FontFamily::SYSTEM_UI, 13.0)
                                        .text_color(TEXT_PRIMARY)
                                        .build()
                                        .unwrap();
                                    ctx.draw_text(&layout, (0.0, 0.0));
                                })
                                .fix_height(20.0)
                                .fix_width(24.0),
                            )
                            .with_spacer(12.0)
                            .with_child(
                                StyledCheckbox
                                    .lens(UIDataAdapter::alt_key)
                                    .padding((0.0, 0.0, 6.0, 0.0)),
                            )
                            .with_child(
                                Painter::new(|ctx, _: &UIDataAdapter, _| {
                                    let layout = ctx
                                        .text()
                                        .new_text_layout(SYMBOL_ALT)
                                        .font(FontFamily::SYSTEM_UI, 13.0)
                                        .text_color(TEXT_PRIMARY)
                                        .build()
                                        .unwrap();
                                    ctx.draw_text(&layout, (0.0, 0.0));
                                })
                                .fix_height(20.0)
                                .fix_width(24.0),
                            )
                            .with_spacer(12.0)
                            .with_child(
                                StyledCheckbox
                                    .lens(UIDataAdapter::shift_key)
                                    .padding((0.0, 0.0, 6.0, 0.0)),
                            )
                            .with_child(
                                Painter::new(|ctx, _: &UIDataAdapter, _| {
                                    let layout = ctx
                                        .text()
                                        .new_text_layout(SYMBOL_SHIFT)
                                        .font(FontFamily::SYSTEM_UI, 13.0)
                                        .text_color(TEXT_PRIMARY)
                                        .build()
                                        .unwrap();
                                    ctx.draw_text(&layout, (0.0, 0.0));
                                })
                                .fix_height(20.0)
                                .fix_width(24.0),
                            )
                            .cross_axis_alignment(druid::widget::CrossAxisAlignment::Center),
                    )
                    .with_spacer(12.0)
                    .with_child(
                        Painter::new(|ctx, _: &UIDataAdapter, _| {
                            let layout = ctx
                                .text()
                                .new_text_layout("Key")
                                .font(FontFamily::SYSTEM_UI, 12.0)
                                .text_color(TEXT_SECONDARY)
                                .build()
                                .unwrap();
                            ctx.draw_text(&layout, (0.0, 0.0));
                        })
                        .fix_height(16.0)
                        .expand_width(),
                    )
                    .with_spacer(6.0)
                    .with_child(
                        TextBox::new()
                            .lens(UIDataAdapter::letter_key)
                            .controller(LetterKeyController)
                            .fix_width(80.0),
                    )
                    .expand_width()
                    .padding((14.0, 10.0)),
            ),
    );

    Flex::column()
        .cross_axis_alignment(druid::widget::CrossAxisAlignment::Start)
        .with_child(section_label("Keyboard shortcut"))
        .with_child(card)
        .with_flex_spacer(1.0)
        .padding((24.0, 20.0, 24.0, 24.0))
}

fn advanced_tab() -> impl Widget<UIDataAdapter> {
    let macro_card = settings_card(
        Flex::column()
            .with_child(settings_row(
                "Text expansion",
                "Expand shorthand into full text",
                StyledCheckbox.lens(UIDataAdapter::is_macro_enabled),
            ))
            .with_child(card_divider())
            .with_child(
                Flex::column()
                    .cross_axis_alignment(druid::widget::CrossAxisAlignment::Start)
                    .with_child(
                        Painter::new(|ctx, _: &UIDataAdapter, _| {
                            let layout = ctx
                                .text()
                                .new_text_layout("Macro table")
                                .font(FontFamily::SYSTEM_UI, 12.0)
                                .text_color(TEXT_SECONDARY)
                                .build()
                                .unwrap();
                            ctx.draw_text(&layout, (0.0, 0.0));
                        })
                        .fix_height(16.0)
                        .expand_width(),
                    )
                    .with_spacer(8.0)
                    .with_flex_child(
                        {
                            let mut scroll = Scroll::new(
                                List::new(macro_row_item)
                                    .lens(UIDataAdapter::macro_table)
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
                    .with_spacer(8.0)
                    .with_child(
                        Flex::row()
                            .with_flex_child(
                                TextBox::new()
                                    .with_placeholder("Shorthand")
                                    .expand_width()
                                    .lens(UIDataAdapter::new_macro_from),
                                2.0,
                            )
                            .with_spacer(6.0)
                            .with_flex_child(
                                TextBox::new()
                                    .with_placeholder("Replacement")
                                    .expand_width()
                                    .lens(UIDataAdapter::new_macro_to),
                                2.0,
                            )
                            .with_spacer(6.0)
                            .with_child(Button::new("Add").on_click(|ctx, _, _| {
                                ctx.submit_command(ADD_MACRO.to(Target::Global))
                            }))
                            .expand_width(),
                    )
                    .expand_width()
                    .padding((14.0, 10.0)),
            ),
    );

    Flex::column()
        .cross_axis_alignment(druid::widget::CrossAxisAlignment::Start)
        .with_child(section_label("Text expansion"))
        .with_flex_child(macro_card.expand_height(), 1.0)
        .padding((24.0, 20.0, 24.0, 24.0))
}

// ══════════════════════════════════════════════════════════════════════════════
// HotkeyBadgesWidget — renders key badges from hotkey_display string
// ══════════════════════════════════════════════════════════════════════════════

struct HotkeyBadgesWidget {
    badges: Vec<WidgetPod<(), KeyBadge>>,
    last_display: String,
}

impl HotkeyBadgesWidget {
    fn new() -> Self {
        Self {
            badges: Vec::new(),
            last_display: String::new(),
        }
    }

    fn rebuild_badges(&mut self, display: &str) {
        // Split on spaces, each token becomes a badge
        self.badges = display
            .split_whitespace()
            .map(|token| WidgetPod::new(KeyBadge::new(token)))
            .collect();
        self.last_display = display.to_string();
    }
}

impl Widget<UIDataAdapter> for HotkeyBadgesWidget {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, _data: &mut UIDataAdapter, env: &Env) {
        for badge in &mut self.badges {
            badge.event(ctx, event, &mut (), env);
        }
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &UIDataAdapter, env: &Env) {
        if let LifeCycle::WidgetAdded = event {
            self.rebuild_badges(&data.hotkey_display);
        }
        for badge in &mut self.badges {
            badge.lifecycle(ctx, event, &(), env);
        }
    }

    fn update(&mut self, ctx: &mut UpdateCtx, _old: &UIDataAdapter, data: &UIDataAdapter, env: &Env) {
        if data.hotkey_display != self.last_display {
            self.rebuild_badges(&data.hotkey_display);
            ctx.children_changed();
        }
        for badge in &mut self.badges {
            badge.update(ctx, &(), env);
        }
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, _data: &UIDataAdapter, env: &Env) -> Size {
        let gap = 4.0;
        let mut x = 0.0;
        let mut max_h = 0.0f64;
        let loose = bc.loosen();
        for badge in &mut self.badges {
            let s = badge.layout(ctx, &loose, &(), env);
            badge.set_origin(ctx, Point::new(x, 0.0));
            x += s.width + gap;
            max_h = max_h.max(s.height);
        }
        Size::new((x - gap).max(0.0), max_h.max(24.0))
    }

    fn paint(&mut self, ctx: &mut PaintCtx, _data: &UIDataAdapter, env: &Env) {
        for badge in &mut self.badges {
            badge.paint(ctx, &(), env);
        }
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// List row helpers (unchanged functionality)
// ══════════════════════════════════════════════════════════════════════════════

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
        .border(Color::rgb8(224, 224, 224), 0.5)
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
        .border(Color::rgb8(224, 224, 224), 0.5)
}

// ══════════════════════════════════════════════════════════════════════════════
// Main UI builder
// ══════════════════════════════════════════════════════════════════════════════

pub fn main_ui_builder() -> impl Widget<UIDataAdapter> {
    Flex::column()
        .with_child(TabBar::new().lens(UIDataAdapter::active_tab).fix_height(58.0))
        .with_flex_child(
            ViewSwitcher::new(
                |data: &UIDataAdapter, _env| data.active_tab,
                |tab, _data, _env| match tab {
                    1 => Box::new(apps_tab()),
                    2 => Box::new(shortcuts_tab()),
                    3 => Box::new(advanced_tab()),
                    _ => Box::new(general_tab()),
                },
            )
            .expand(),
            1.0,
        )
        .background(WIN_BG)
        .controller(UIController)
}

// ══════════════════════════════════════════════════════════════════════════════
// Permission request UI (unchanged)
// ══════════════════════════════════════════════════════════════════════════════

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

// ══════════════════════════════════════════════════════════════════════════════
// Kept for backward-compat (used in UIController hotkey window close handling)
// ══════════════════════════════════════════════════════════════════════════════

pub fn macro_editor_ui_builder() -> impl Widget<UIDataAdapter> {
    Flex::column()
        .cross_axis_alignment(druid::widget::CrossAxisAlignment::Start)
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
        .with_child(
            Flex::row()
                .with_flex_child(
                    TextBox::new()
                        .with_placeholder("Shorthand")
                        .expand_width()
                        .lens(UIDataAdapter::new_macro_from),
                    2.0,
                )
                .with_flex_child(
                    TextBox::new()
                        .with_placeholder("Replacement")
                        .expand_width()
                        .lens(UIDataAdapter::new_macro_to),
                    2.0,
                )
                .with_child(
                    Button::new("Add")
                        .on_click(|ctx, _, _| ctx.submit_command(ADD_MACRO.to(Target::Global))),
                )
                .expand_width(),
        )
        .padding(8.0)
}

pub fn app_settings_ui_builder() -> impl Widget<UIDataAdapter> {
    apps_tab()
}

pub fn center_window_position() -> (f64, f64) {
    let screen_rect = Screen::get_display_rect();
    let x = (screen_rect.width() - WINDOW_WIDTH) / 2.0;
    let y = (screen_rect.height() - WINDOW_HEIGHT) / 2.0;
    (x, y)
}
