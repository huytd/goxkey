use druid::{Color, Env, Key};
use std::sync::Arc;

pub const GREEN: Color = Color::rgb8(26, 138, 110);
pub const GREEN_BG: Color = Color::rgba8(26, 138, 110, 20);

#[derive(Clone, Copy, Debug)]
pub struct Theme {
    pub win_bg: Color,
    pub card_bg: Color,
    pub card_border: Color,
    pub divider: Color,
    pub text_primary: Color,
    pub text_secondary: Color,
    pub text_section: Color,
    pub badge_bg: Color,
    pub badge_border: Color,
    pub badge_text: Color,
    pub btn_reset_bg: Color,
    pub btn_reset_border: Color,
    pub btn_reset_text: Color,
    pub segmented_bg: Color,
    pub segmented_border: Color,
    pub segmented_text: Color,
    pub segmented_ring: Color,
    pub input_bg: Color,
    pub input_border: Color,
    pub input_text: Color,
    pub input_placeholder: Color,
    pub tab_border: Color,
    pub tab_inactive: Color,
    pub list_row_hover: Color,
    pub toggle_off: Color,
    pub checkbox_border: Color,
    pub tooltip_bg: Color,
}

pub static THEME: Key<Arc<Theme>> = Key::new("goxkey.theme");
pub static IS_DARK: Key<bool> = Key::new("goxkey.is_dark");

pub fn light_theme() -> Theme {
    Theme {
        win_bg: Color::rgb8(255, 255, 255),
        card_bg: Color::rgb8(245, 245, 245),
        card_border: Color::rgba8(0, 0, 0, 30),
        divider: Color::rgba8(0, 0, 0, 20),
        text_primary: Color::rgb8(17, 17, 17),
        text_secondary: Color::rgb8(102, 102, 102),
        text_section: Color::rgb8(153, 153, 153),
        badge_bg: Color::rgb8(255, 255, 255),
        badge_border: Color::rgb8(204, 204, 204),
        badge_text: Color::rgb8(85, 85, 85),
        btn_reset_bg: Color::rgb8(240, 240, 240),
        btn_reset_border: Color::rgb8(204, 204, 204),
        btn_reset_text: Color::rgb8(51, 51, 51),
        segmented_bg: Color::WHITE,
        segmented_border: Color::rgb8(221, 221, 221),
        segmented_text: Color::rgb8(136, 136, 136),
        segmented_ring: Color::rgb8(187, 187, 187),
        input_bg: Color::WHITE,
        input_border: Color::rgb8(204, 204, 204),
        input_text: Color::rgb8(17, 17, 17),
        input_placeholder: Color::rgba8(0, 0, 0, 80),
        tab_border: Color::rgb8(221, 221, 221),
        tab_inactive: Color::rgb8(153, 153, 153),
        list_row_hover: Color::rgba8(0, 0, 0, 8),
        toggle_off: Color::rgb8(187, 187, 187),
        checkbox_border: Color::rgb8(204, 204, 204),
        tooltip_bg: Color::rgb8(40, 40, 40),
    }
}

pub fn dark_theme() -> Theme {
    Theme {
        win_bg: Color::rgb8(30, 30, 30),
        card_bg: Color::rgb8(45, 45, 45),
        card_border: Color::rgba8(255, 255, 255, 20),
        divider: Color::rgba8(255, 255, 255, 15),
        text_primary: Color::rgb8(245, 245, 245),
        text_secondary: Color::rgb8(170, 170, 170),
        text_section: Color::rgb8(120, 120, 120),
        badge_bg: Color::rgb8(60, 60, 60),
        badge_border: Color::rgb8(100, 100, 100),
        badge_text: Color::rgb8(200, 200, 200),
        btn_reset_bg: Color::rgb8(60, 60, 60),
        btn_reset_border: Color::rgb8(80, 80, 80),
        btn_reset_text: Color::rgb8(220, 220, 220),
        segmented_bg: Color::rgb8(45, 45, 45),
        segmented_border: Color::rgb8(70, 70, 70),
        segmented_text: Color::rgb8(150, 150, 150),
        segmented_ring: Color::rgb8(100, 100, 100),
        input_bg: Color::rgb8(45, 45, 45),
        input_border: Color::rgb8(70, 70, 70),
        input_text: Color::rgb8(245, 245, 245),
        input_placeholder: Color::rgba8(255, 255, 255, 80),
        tab_border: Color::rgb8(70, 70, 70),
        tab_inactive: Color::rgb8(120, 120, 120),
        list_row_hover: Color::rgba8(255, 255, 255, 8),
        toggle_off: Color::rgb8(80, 80, 80),
        checkbox_border: Color::rgb8(80, 80, 80),
        tooltip_bg: Color::rgb8(60, 60, 60),
    }
}

pub fn get_theme(is_dark: bool) -> Theme {
    if is_dark {
        dark_theme()
    } else {
        light_theme()
    }
}

pub fn theme_from_env(env: &Env) -> Theme {
    get_theme(env.get(&IS_DARK))
}

pub const BADGE_VI_BG: Color = Color::rgba8(26, 138, 110, 20);
pub const BADGE_VI_BORDER: Color = Color::rgb8(26, 138, 110);
pub const BADGE_EN_BG: Color = Color::rgba8(58, 115, 199, 18);
pub const BADGE_EN_BORDER: Color = Color::rgb8(58, 115, 199);
