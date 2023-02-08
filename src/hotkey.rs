use std::fmt::Display;

use crate::platform::{
    KeyModifier, KEY_DELETE, KEY_ENTER, KEY_ESCAPE, KEY_SPACE, KEY_TAB, SYMBOL_ALT, SYMBOL_CTRL,
    SYMBOL_SHIFT, SYMBOL_SUPER,
};

pub struct Hotkey {
    modifiers: KeyModifier,
    keycode: char,
}

impl Hotkey {
    pub fn from(input: &str) -> Self {
        let mut modifiers = KeyModifier::new();
        let mut keycode: char = '\0';
        input
            .split('+')
            .for_each(|token| match token.trim().to_uppercase().as_str() {
                "SHIFT" => modifiers.add_shift(),
                "ALT" => modifiers.add_alt(),
                "SUPER" => modifiers.add_super(),
                "CTRL" => modifiers.add_control(),
                "ENTER" => keycode = KEY_ENTER,
                "SPACE" => keycode = KEY_SPACE,
                "TAB" => keycode = KEY_TAB,
                "DELETE" => keycode = KEY_DELETE,
                "ESC" => keycode = KEY_ESCAPE,
                c => {
                    keycode = c.chars().last().unwrap();
                }
            });
        Self { modifiers, keycode }
    }

    pub fn is_match(&self, modifiers: KeyModifier, keycode: &char) -> bool {
        return self.modifiers == modifiers && self.keycode.eq_ignore_ascii_case(keycode);
    }
}

impl Display for Hotkey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.modifiers.is_control() {
            _ = write!(f, "{} ", SYMBOL_CTRL);
        }
        if self.modifiers.is_shift() {
            _ = write!(f, "{} ", SYMBOL_SHIFT);
        }
        if self.modifiers.is_alt() {
            _ = write!(f, "{} ", SYMBOL_ALT);
        }
        if self.modifiers.is_super() {
            _ = write!(f, "{} ", SYMBOL_SUPER);
        }
        write!(
            f,
            "{}",
            match self.keycode {
                KEY_ENTER => "Enter".to_owned(),
                KEY_SPACE => "Space".to_owned(),
                KEY_TAB => "Tab".to_owned(),
                KEY_DELETE => "Del".to_owned(),
                KEY_ESCAPE => "Esc".to_owned(),
                c => format!("{}", c.to_uppercase()),
            }
        )
    }
}

#[test]
fn test_parse() {
    let hotkey = Hotkey::from("super+shift+z");
    let mut actual_modifier = KeyModifier::new();
    actual_modifier.add_shift();
    actual_modifier.add_super();
    assert_eq!(hotkey.modifiers, actual_modifier);
    assert_eq!(hotkey.keycode, 'Z');
    assert_eq!(hotkey.is_match(actual_modifier, &'z'), true);
}

#[test]
fn test_parse_long_input() {
    let hotkey = Hotkey::from("super+shift+ctrl+alt+w");
    let mut actual_modifier = KeyModifier::new();
    actual_modifier.add_shift();
    actual_modifier.add_super();
    actual_modifier.add_control();
    actual_modifier.add_alt();
    assert_eq!(hotkey.modifiers, actual_modifier);
    assert_eq!(hotkey.keycode, 'W');
    assert_eq!(hotkey.is_match(actual_modifier, &'W'), true);
}

#[test]
fn test_parse_with_named_keycode() {
    let hotkey = Hotkey::from("super+ctrl+space");
    let mut actual_modifier = KeyModifier::new();
    actual_modifier.add_super();
    actual_modifier.add_control();
    assert_eq!(hotkey.modifiers, actual_modifier);
    assert_eq!(hotkey.keycode, KEY_SPACE);
    assert_eq!(hotkey.is_match(actual_modifier, &KEY_SPACE), true);
}

#[test]
fn test_display() {
    assert_eq!(
        format!("{}", Hotkey::from("super+ctrl+space")),
        format!("{} {} Space", SYMBOL_CTRL, SYMBOL_SUPER)
    );

    assert_eq!(
        format!("{}", Hotkey::from("super+alt+z")),
        format!("{} {} Z", SYMBOL_ALT, SYMBOL_SUPER)
    );

    assert_eq!(
        format!("{}", Hotkey::from("ctrl+shift+o")),
        format!("{} {} O", SYMBOL_CTRL, SYMBOL_SHIFT)
    );
}
