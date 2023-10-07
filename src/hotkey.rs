use std::fmt::Display;

use crate::platform::{
    KeyModifier, KEY_DELETE, KEY_ENTER, KEY_ESCAPE, KEY_SPACE, KEY_TAB, SYMBOL_ALT, SYMBOL_CTRL,
    SYMBOL_SHIFT, SYMBOL_SUPER,
};

pub struct Hotkey {
    modifiers: KeyModifier,
    keycode: Option<char>,
}

impl Hotkey {
    pub fn from_str(input: &str) -> Self {
        let mut modifiers = KeyModifier::new();
        let mut keycode: Option<char> = None;
        input
            .split('+')
            .for_each(|token| match token.trim().to_uppercase().as_str() {
                "SHIFT" => modifiers.add_shift(),
                "ALT" => modifiers.add_alt(),
                "SUPER" => modifiers.add_super(),
                "CTRL" => modifiers.add_control(),
                "ENTER" => keycode = Some(KEY_ENTER),
                "SPACE" => keycode = Some(KEY_SPACE),
                "TAB" => keycode = Some(KEY_TAB),
                "DELETE" => keycode = Some(KEY_DELETE),
                "ESC" => keycode = Some(KEY_ESCAPE),
                c => {
                    keycode = c.chars().last();
                }
            });
        Self { modifiers, keycode }
    }

    pub fn is_match(&self, mut modifiers: KeyModifier, keycode: Option<char>) -> bool {
        // Caps Lock should not interfere with any hotkey
        modifiers.remove(KeyModifier::MODIFIER_CAPSLOCK);
        let letter_matched = keycode.eq(&self.keycode)
            || keycode
                .and_then(|a| self.keycode.map(|b| a.eq_ignore_ascii_case(&b)))
                .is_some_and(|c| c == true);
        self.modifiers == modifiers && letter_matched
    }

    pub fn inner(&self) -> (KeyModifier, Option<char>) {
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
            Some(KEY_ENTER) => write!(f, "Enter"),
            Some(KEY_SPACE) => write!(f, "Space"),
            Some(KEY_TAB) => write!(f, "Tab"),
            Some(KEY_DELETE) => write!(f, "Del"),
            Some(KEY_ESCAPE) => write!(f, "Esc"),
            Some(c) => write!(f, "{}", c.to_ascii_uppercase()),
            _ => write!(f, ""),
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
    assert_eq!(hotkey.keycode, Some('Z'));
    assert!(hotkey.is_match(actual_modifier, Some('z')));
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
    assert_eq!(hotkey.keycode, Some('W'));
    assert!(hotkey.is_match(actual_modifier, Some('W')));
}

#[test]
fn test_parse_with_named_keycode() {
    let hotkey = Hotkey::from_str("super+ctrl+space");
    let mut actual_modifier = KeyModifier::new();
    actual_modifier.add_super();
    actual_modifier.add_control();
    assert_eq!(hotkey.modifiers, actual_modifier);
    assert_eq!(hotkey.keycode, Some(KEY_SPACE));
    assert!(hotkey.is_match(actual_modifier, Some(KEY_SPACE)));
}

#[test]
fn test_can_match_with_or_without_capslock() {
    let hotkey = Hotkey::from_str("super+ctrl+space");
    let mut actual_modifier = KeyModifier::new();
    actual_modifier.add_super();
    actual_modifier.add_control();
    assert_eq!(hotkey.is_match(actual_modifier, Some(' ')), true);

    actual_modifier.add_capslock();
    assert!(hotkey.is_match(actual_modifier, Some(' ')));
}

#[test]
fn test_parse_with_just_modifiers() {
    let hotkey = Hotkey::from_str("ctrl+shift");
    let mut actual_modifier = KeyModifier::new();
    actual_modifier.add_control();
    actual_modifier.add_shift();
    assert_eq!(hotkey.modifiers, actual_modifier);
    assert_eq!(hotkey.keycode, None);
    assert!(hotkey.is_match(actual_modifier, None));
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
