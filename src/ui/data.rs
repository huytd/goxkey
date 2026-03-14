use std::sync::Arc;

use crate::{
    input::{TypingMethod, INPUT_STATE},
    platform::{is_launch_on_login, SystemTray, SystemTrayMenuItemKey},
    UI_EVENT_SINK,
};
use druid::{commands::QUIT_APP, Data, Lens, Target};

use super::{format_letter_key, SHOW_UI, UPDATE_UI};

#[derive(Clone, Data, PartialEq, Eq)]
pub(super) struct MacroEntry {
    pub(super) from: String,
    pub(super) to: String,
}

#[derive(Clone, Data, PartialEq, Eq)]
pub(super) struct AppEntry {
    pub(super) name: String,
}

#[derive(Clone, Data, Lens, PartialEq, Eq)]
pub struct UIDataAdapter {
    pub(super) is_enabled: bool,
    pub(super) typing_method: TypingMethod,
    pub(super) hotkey_display: String,
    pub(super) launch_on_login: bool,
    pub(super) is_auto_toggle_enabled: bool,
    // Macro config
    pub(super) is_macro_enabled: bool,
    pub(super) macro_table: Arc<Vec<MacroEntry>>,
    pub(super) new_macro_from: String,
    pub(super) new_macro_to: String,
    // App language settings
    pub(super) vn_apps: Arc<Vec<AppEntry>>,
    pub(super) en_apps: Arc<Vec<AppEntry>>,
    pub(super) new_en_app: String,
    // Hotkey config
    pub(super) super_key: bool,
    pub(super) ctrl_key: bool,
    pub(super) alt_key: bool,
    pub(super) shift_key: bool,
    pub(super) capslock_key: bool,
    pub(super) letter_key: String,
    // Tab navigation (0=General, 1=Apps, 2=Shortcuts, 3=Advanced)
    pub(super) active_tab: u32,
    // Apps tab selected row (combined vn+en list, -1 = none)
    pub(super) selected_app_index: i32,
    // system tray
    pub(super) systray: SystemTray,
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
                    self.systray.set_menu_item_title(
                        SystemTrayMenuItemKey::TypingMethodTelexVNI,
                        "Telex+VNI",
                    );
                }
                TypingMethod::Telex => {
                    self.systray
                        .set_menu_item_title(SystemTrayMenuItemKey::TypingMethodTelex, "Telex ✓");
                    self.systray
                        .set_menu_item_title(SystemTrayMenuItemKey::TypingMethodVNI, "VNI");
                    self.systray.set_menu_item_title(
                        SystemTrayMenuItemKey::TypingMethodTelexVNI,
                        "Telex+VNI",
                    );
                }
                TypingMethod::TelexVNI => {
                    self.systray
                        .set_menu_item_title(SystemTrayMenuItemKey::TypingMethodTelex, "Telex");
                    self.systray
                        .set_menu_item_title(SystemTrayMenuItemKey::TypingMethodVNI, "VNI");
                    self.systray.set_menu_item_title(
                        SystemTrayMenuItemKey::TypingMethodTelexVNI,
                        "Telex+VNI ✓",
                    );
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
