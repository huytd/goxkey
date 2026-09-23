//! Keysym classification used by the composer.

use goxkey_core::TypingMethod;
use xkeysym::Keysym;

pub fn keysym_to_char(keysym: Keysym) -> Option<char> {
    keysym.key_char()
}

pub fn is_input_char(c: char) -> bool {
    c.is_alphabetic() || c.is_ascii_digit()
}

pub fn is_shift_key(keysym: Keysym) -> bool {
    matches!(
        keysym,
        Keysym::Shift_L | Keysym::Shift_R | Keysym::Caps_Lock | Keysym::Shift_Lock
    )
}

pub fn is_reset_modifier_key(keysym: Keysym) -> bool {
    matches!(
        keysym,
        Keysym::Control_L
            | Keysym::Control_R
            | Keysym::Alt_L
            | Keysym::Alt_R
            | Keysym::Super_L
            | Keysym::Super_R
            | Keysym::Meta_L
            | Keysym::Meta_R
            | Keysym::Hyper_L
            | Keysym::Hyper_R
            | Keysym::Mode_switch
            | Keysym::ISO_Level3_Shift
            | Keysym::ISO_Level5_Shift
    )
}

pub fn is_navigation_key(keysym: Keysym) -> bool {
    keysym.is_cursor_key()
        || matches!(
            keysym,
            Keysym::Delete
                | Keysym::Insert
                | Keysym::KP_Delete
                | Keysym::KP_Insert
                | Keysym::KP_Begin
        )
}

pub fn is_word_separator_key(keysym: Keysym) -> bool {
    matches!(
        keysym,
        Keysym::space
            | Keysym::Return
            | Keysym::Tab
            | Keysym::Escape
            | Keysym::KP_Enter
            | Keysym::KP_Space
            | Keysym::KP_Tab
            | Keysym::Linefeed
            | Keysym::Clear
    )
}

/// Text to insert ourselves for a separator key, if any.
///
/// Only Space is committed via `commit_text`. Forwarding Space races with a
/// pending `delete_surrounding_text` from the previous keystroke, which can
/// eat the space. Return/Enter/Tab/Escape must be forwarded so apps still
/// receive the real key (e.g. chat send on Enter, focus change on Tab).
pub fn separator_commit_text(keysym: Keysym) -> Option<&'static str> {
    match keysym {
        Keysym::space | Keysym::KP_Space => Some(" "),
        _ => None,
    }
}

/// True when `c` ends a leading run of digits in VNI mode.
///
/// `InputState::push` drops leading digits from both the typing buffer and the
/// display buffer in that case. The engine diffs the display buffer against
/// what it already committed, so that would delete the digits from the screen
/// (typing "10am" gave "am"). Starting a new word before the push keeps them.
pub fn ends_leading_number(method: TypingMethod, buffer: &str, c: char) -> bool {
    method == TypingMethod::VNI && buffer.starts_with(|ch: char| ch.is_numeric()) && !c.is_numeric()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keysym_to_char() {
        assert_eq!(keysym_to_char(Keysym::a), Some('a'));
        assert_eq!(keysym_to_char(Keysym::A), Some('A'));
        assert_eq!(keysym_to_char(Keysym::Control_L), None);
        assert_eq!(keysym_to_char(Keysym::Shift_L), None);
        assert_eq!(keysym_to_char(Keysym::Alt_L), None);
        assert_eq!(keysym_to_char(Keysym::Super_L), None);
        assert_eq!(keysym_to_char(Keysym::Left), None);
        assert_eq!(keysym_to_char(Keysym::BackSpace), Some('\u{08}'));
        assert_eq!(keysym_to_char(Keysym::space), Some(' '));
    }

    #[test]
    fn test_keysym_categories() {
        assert!(is_shift_key(Keysym::Shift_L));
        assert!(is_shift_key(Keysym::Shift_R));
        assert!(is_shift_key(Keysym::Caps_Lock));

        assert!(is_reset_modifier_key(Keysym::Control_L));
        assert!(is_reset_modifier_key(Keysym::Control_R));
        assert!(is_reset_modifier_key(Keysym::Alt_L));
        assert!(is_reset_modifier_key(Keysym::Alt_R));
        assert!(is_reset_modifier_key(Keysym::Super_L));
        assert!(is_reset_modifier_key(Keysym::Super_R));

        assert!(is_navigation_key(Keysym::Left));
        assert!(is_navigation_key(Keysym::Right));
        assert!(is_navigation_key(Keysym::Up));
        assert!(is_navigation_key(Keysym::Down));
        assert!(is_navigation_key(Keysym::Home));
        assert!(is_navigation_key(Keysym::End));
        assert!(is_navigation_key(Keysym::Page_Up));
        assert!(is_navigation_key(Keysym::Page_Down));
        assert!(is_navigation_key(Keysym::Delete));
        assert!(is_navigation_key(Keysym::Insert));

        assert!(is_word_separator_key(Keysym::space));
        assert!(is_word_separator_key(Keysym::Return));
        assert!(is_word_separator_key(Keysym::Tab));
        assert!(is_word_separator_key(Keysym::Escape));
        assert!(is_word_separator_key(Keysym::KP_Enter));
    }

    #[test]
    fn test_separator_commit_text() {
        assert_eq!(separator_commit_text(Keysym::space), Some(" "));
        assert_eq!(separator_commit_text(Keysym::KP_Space), Some(" "));
        // Action keys must be forwarded, not inserted as text.
        assert_eq!(separator_commit_text(Keysym::Tab), None);
        assert_eq!(separator_commit_text(Keysym::KP_Tab), None);
        assert_eq!(separator_commit_text(Keysym::Return), None);
        assert_eq!(separator_commit_text(Keysym::KP_Enter), None);
        assert_eq!(separator_commit_text(Keysym::Linefeed), None);
        assert_eq!(separator_commit_text(Keysym::Escape), None);
        assert_eq!(separator_commit_text(Keysym::Clear), None);
    }

    #[test]
    fn test_ends_leading_number() {
        assert!(ends_leading_number(TypingMethod::VNI, "1", 'a'));
        assert!(ends_leading_number(TypingMethod::VNI, "10", 'a'));
        assert!(!ends_leading_number(TypingMethod::VNI, "10", '0'));
        assert!(!ends_leading_number(TypingMethod::VNI, "", 'a'));
        // Digits after letters are VNI tone/mark keys, not a number.
        assert!(!ends_leading_number(TypingMethod::VNI, "a1", 'n'));
        assert!(!ends_leading_number(TypingMethod::Telex, "10", 'a'));
    }
}
