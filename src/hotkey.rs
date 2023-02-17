use std::{ascii::AsciiExt, fmt::Display};

use crate::platform::{
    KeyModifier, KEY_DELETE, KEY_ENTER, KEY_ESCAPE, KEY_SPACE, KEY_TAB, SYMBOL_ALT, SYMBOL_CTRL,
    SYMBOL_SHIFT, SYMBOL_SUPER,
};

pub struct Hotkey {
    modifiers: KeyModifier,
    keycode: char,
}

impl Hotkey {
    pub fn from_str(input: &str) -> Self {
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

    pub fn from(modifiers: KeyModifier, keycode: char) -> Self {
        Self { modifiers, keycode }
    }

    pub fn is_match(&self, modifiers: KeyModifier, keycode: &char) -> bool {
        return self.modifiers == modifiers && self.keycode.eq_ignore_ascii_case(keycode);
    }

    pub fn inner(&self) -> (KeyModifier, char) {
        (self.modifiers, self.keycode)
    }
}

impl Display for Hotkey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.modifiers.is_control() {
            write!(f, "{} ", SYMBOL_CTRL)?;
        }
        if self.modifiers.is_shift() {
            write!(f, "{} ", SYMBOL_SHIFT)?;
        }
        if self.modifiers.is_alt() {
            write!(f, "{} ", SYMBOL_ALT)?;
        }
        if self.modifiers.is_super() {
            write!(f, "{} ", SYMBOL_SUPER)?;
        }
        match self.keycode {
            KEY_ENTER => write!(f, "Enter"),
            KEY_SPACE => write!(f, "Space"),
            KEY_TAB => write!(f, "Tab"),
            KEY_DELETE => write!(f, "Del"),
            KEY_ESCAPE => write!(f, "Esc"),
            c => write!(f, "{}", c.to_ascii_uppercase()),
        }
    }
}

#[test]
fn test_parse() {
    let hotkey = Hotkey::from_str("super+shift+z");
    let mut actual_modifier = KeyModifier::new();
    actual_modifier.add_shift();
    actual_modifier.add_super();
    assert_eq!(hotkey.modifiers, actual_modifier);
    assert_eq!(hotkey.keycode, 'Z');
    assert_eq!(hotkey.is_match(actual_modifier, &'z'), true);
}

#[test]
fn test_parse_long_input() {
    let hotkey = Hotkey::from_str("super+shift+ctrl+alt+w");
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
    let hotkey = Hotkey::from_str("super+ctrl+space");
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
        format!("{}", Hotkey::from_str("super+ctrl+space")),
        format!("{} {} Space", SYMBOL_CTRL, SYMBOL_SUPER)
    );

    assert_eq!(
        format!("{}", Hotkey::from_str("super+alt+z")),
        format!("{} {} Z", SYMBOL_ALT, SYMBOL_SUPER)
    );

    assert_eq!(
        format!("{}", Hotkey::from_str("ctrl+shift+o")),
        format!("{} {} O", SYMBOL_CTRL, SYMBOL_SHIFT)
    );
}
