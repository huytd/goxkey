use std::sync::atomic::{AtomicU8, Ordering};

const LANG_VI: u8 = 0;
const LANG_EN: u8 = 1;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Lang {
    Vi,
    En,
}

static LANG: AtomicU8 = AtomicU8::new(LANG_EN);

/// Resolve the effective language from config, CLI flag, or OS preference.
fn resolve_lang(config_value: &str) -> Lang {
    // CLI --lang flag takes highest priority
    let pref = std::env::args()
        .skip_while(|a| a != "--lang")
        .nth(1)
        .unwrap_or_else(|| {
            // Then config value, then OS preference
            match config_value {
                "vi" => "vi".to_string(),
                "en" => "en".to_string(),
                _ => crate::platform::get_preferred_language(), // "auto" or unknown
            }
        });
    if pref.starts_with("vi") {
        Lang::Vi
    } else {
        Lang::En
    }
}

/// Initialize the language from config. Called once at startup.
pub fn init_lang(config_value: &str) {
    let lang = resolve_lang(config_value);
    LANG.store(
        match lang {
            Lang::Vi => LANG_VI,
            Lang::En => LANG_EN,
        },
        Ordering::Relaxed,
    );
}

/// Update the language at runtime (e.g. from UI settings).
pub fn set_lang(lang: Lang) {
    LANG.store(
        match lang {
            Lang::Vi => LANG_VI,
            Lang::En => LANG_EN,
        },
        Ordering::Relaxed,
    );
}

pub fn current_lang() -> Lang {
    match LANG.load(Ordering::Relaxed) {
        LANG_VI => Lang::Vi,
        _ => Lang::En,
    }
}

/// Translate a key to the current locale.
pub fn t(key: &'static str) -> &'static str {
    let lang = current_lang();
    match (lang, key) {
        // ── System tray menu ────────────────────────────────────────────
        (Lang::Vi, "menu.open_panel") => "Mở bảng điều khiển",
        (Lang::En, "menu.open_panel") => "Open Control Panel",

        (Lang::Vi, "menu.disable_vietnamese") => "Tắt gõ tiếng Việt",
        (Lang::En, "menu.disable_vietnamese") => "Disable Vietnamese",

        (Lang::Vi, "menu.enable_vietnamese") => "Bật gõ tiếng Việt",
        (Lang::En, "menu.enable_vietnamese") => "Enable Vietnamese",

        (Lang::Vi, "menu.quit") => "Thoát ứng dụng",
        (Lang::En, "menu.quit") => "Quit",

        // ── Accessibility permission dialog ─────────────────────────────
        (Lang::Vi, "perm.title") => {
            "Chờ đã! Bạn cần phải cấp quyền Accessibility\ncho ứng dụng GõKey trước khi sử dụng."
        }
        (Lang::En, "perm.title") => {
            "Wait! You need to grant Accessibility\npermission for GõKey before using."
        }

        (Lang::Vi, "perm.subtitle") => {
            "Bạn vui lòng thoát khỏi ứng dụng\nvà mở lại sau khi đã cấp quyền."
        }
        (Lang::En, "perm.subtitle") => {
            "Please quit the application\nand reopen after granting permission."
        }

        (Lang::Vi, "perm.exit") => "Thoát",
        (Lang::En, "perm.exit") => "Exit",

        // ── Tab labels ──────────────────────────────────────────────────
        (Lang::Vi, "tab.general") => "Chung",
        (Lang::En, "tab.general") => "General",

        (Lang::Vi, "tab.apps") => "Ứng dụng",
        (Lang::En, "tab.apps") => "Apps",

        (Lang::Vi, "tab.text_expansion") => "Gõ tắt",
        (Lang::En, "tab.text_expansion") => "Text Expansion",

        // ── General tab ─────────────────────────────────────────────────
        (Lang::Vi, "general.input_mode") => "Chế độ gõ",
        (Lang::En, "general.input_mode") => "Input mode",

        (Lang::Vi, "general.language") => "Ngôn ngữ",
        (Lang::En, "general.language") => "Language",

        (Lang::Vi, "general.ui_language") => "Ngôn ngữ giao diện",
        (Lang::En, "general.ui_language") => "UI language",

        (Lang::Vi, "general.ui_language_desc") => "Thay đổi ngôn ngữ giao diện",
        (Lang::En, "general.ui_language_desc") => "Change the interface language",

        (Lang::Vi, "general.system") => "Hệ thống",
        (Lang::En, "general.system") => "System",

        (Lang::Vi, "general.shortcut") => "Phím tắt",
        (Lang::En, "general.shortcut") => "Shortcut",

        (Lang::Vi, "general.vietnamese_input") => "Gõ tiếng Việt",
        (Lang::En, "general.vietnamese_input") => "Vietnamese input",

        (Lang::Vi, "general.enable_vietnamese") => "Bật chế độ gõ tiếng Việt",
        (Lang::En, "general.enable_vietnamese") => "Enable Vietnamese typing mode",

        (Lang::Vi, "general.input_method") => "Kiểu gõ",
        (Lang::En, "general.input_method") => "Input method",

        (Lang::Vi, "general.w_literal") => "Chế độ W nguyên bản",
        (Lang::En, "general.w_literal") => "W literal mode",

        (Lang::Vi, "general.w_literal_desc") => "Gõ w ra w; dùng uw, ow, aw cho ư, ơ, ă",
        (Lang::En, "general.w_literal_desc") => "Type w for w; use uw, ow, aw for ư, ơ, ă",

        (Lang::Vi, "general.launch_at_login") => "Khởi động cùng hệ thống",
        (Lang::En, "general.launch_at_login") => "Launch at login",

        (Lang::Vi, "general.launch_at_login_desc") => "Tự động mở GõKey khi đăng nhập",
        (Lang::En, "general.launch_at_login_desc") => "Start gõkey when you log in",

        (Lang::Vi, "general.toggle_shortcut") => "Bật/tắt tiếng Việt",
        (Lang::En, "general.toggle_shortcut") => "Toggle Vietnamese input",

        (Lang::Vi, "general.toggle_shortcut_desc") => "Phím tắt bật/tắt chế độ gõ",
        (Lang::En, "general.toggle_shortcut_desc") => "Keyboard shortcut to toggle on/off",

        (Lang::Vi, "general.reset_defaults") => "Đặt lại mặc định",
        (Lang::En, "general.reset_defaults") => "Reset defaults",

        (Lang::Vi, "general.done") => "Xong",
        (Lang::En, "general.done") => "Done",

        // ── Apps tab ────────────────────────────────────────────────────
        (Lang::Vi, "apps.description") => "Đặt ngôn ngữ gõ cho từng ứng dụng.",
        (Lang::En, "apps.description") => "Set input language per application.",

        (Lang::Vi, "apps.per_app_toggle") => "Chế độ theo ứng dụng",
        (Lang::En, "apps.per_app_toggle") => "Per-app toggle",

        (Lang::Vi, "apps.per_app_toggle_desc") => "Bật/tắt theo từng ứng dụng",
        (Lang::En, "apps.per_app_toggle_desc") => "Enable/disable per application",

        (Lang::Vi, "apps.vietnamese") => "Tiếng Việt",
        (Lang::En, "apps.vietnamese") => "Vietnamese",

        (Lang::Vi, "apps.english") => "Tiếng Anh",
        (Lang::En, "apps.english") => "English",

        // ── Text expansion tab ──────────────────────────────────────────
        (Lang::Vi, "macro.description") => "Tự động mở rộng từ viết tắt thành văn bản đầy đủ.",
        (Lang::En, "macro.description") => "Expand shorthand into full text automatically.",

        (Lang::Vi, "macro.text_expansion") => "Gõ tắt",
        (Lang::En, "macro.text_expansion") => "Text expansion",

        (Lang::Vi, "macro.enable") => "Bật chế độ gõ tắt",
        (Lang::En, "macro.enable") => "Enable shorthand expansion",

        (Lang::Vi, "macro.auto_capitalize") => "Tự động viết hoa",
        (Lang::En, "macro.auto_capitalize") => "Auto capitalize",

        (Lang::Vi, "macro.auto_capitalize_desc") => "Áp dụng kiểu viết hoa từ chữ viết tắt",
        (Lang::En, "macro.auto_capitalize_desc") => "Apply capitalization from typed shorthand",

        (Lang::Vi, "macro.shorthand") => "Viết tắt",
        (Lang::En, "macro.shorthand") => "Shorthand",

        (Lang::Vi, "macro.replacement") => "Thay thế",
        (Lang::En, "macro.replacement") => "Replacement",

        (Lang::Vi, "macro.load") => "Tải",
        (Lang::En, "macro.load") => "Load",

        (Lang::Vi, "macro.export") => "Xuất",
        (Lang::En, "macro.export") => "Export",

        // ── Common buttons ──────────────────────────────────────────────
        (Lang::Vi, "button.add") => "Thêm",
        (Lang::En, "button.add") => "Add",

        (Lang::Vi, "button.cancel") => "Huỷ",
        (Lang::En, "button.cancel") => "Cancel",

        (Lang::Vi, "button.save") => "Lưu",
        (Lang::En, "button.save") => "Save",

        // ── Shortcut editor ─────────────────────────────────────────────
        (Lang::Vi, "shortcut.new") => "Phím tắt mới",
        (Lang::En, "shortcut.new") => "New Shortcut",

        (Lang::Vi, "shortcut.hint") => "Nhấn phím. Cho phép chỉ dùng phím bổ trợ.",
        (Lang::En, "shortcut.hint") => "Press keys. Modifier-only combos are allowed.",

        // ── Shortcut capture widget ─────────────────────────────────────
        (Lang::Vi, "shortcut.type_prompt") => "Nhập phím tắt…",
        (Lang::En, "shortcut.type_prompt") => "Type a shortcut…",

        (Lang::Vi, "shortcut.press_keys") => "Nhấn phím…",
        (Lang::En, "shortcut.press_keys") => "Press keys…",

        (Lang::Vi, "shortcut.click_and_press") => "Nhấp và nhấn phím…",
        (Lang::En, "shortcut.click_and_press") => "Click and press keys…",

        // Fallback
        _ => key,
    }
}
