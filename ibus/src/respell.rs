//! Turn a displayed Vietnamese word back into the keys that type it.
//!
//! Used when Backspace removes the last *visible* character: the remaining
//! text has to become the new typing buffer so later tone and mark keys keep
//! working on it.

use goxkey_core::TypingMethod;

/// Each row lists a vowel with no tone, then sắc, huyền, hỏi, ngã, nặng.
const TONE_ROWS: [&str; 12] = [
    "aáàảãạ", "ăắằẳẵặ", "âấầẩẫậ", "eéèẻẽẹ", "êếềểễệ", "iíìỉĩị", "oóòỏõọ", "ôốồổỗộ",
    "ơớờởỡợ", "uúùủũụ", "ưứừửữự", "yýỳỷỹỵ",
];

/// Split a lowercase character into its toneless form and tone index
/// (0 = none, 1..=5 = sắc, huyền, hỏi, ngã, nặng).
fn split_tone(c: char) -> (char, usize) {
    for row in TONE_ROWS {
        if let Some(tone) = row.chars().position(|x| x == c) {
            return (row.chars().next().unwrap(), tone);
        }
    }
    (c, 0)
}

/// Keys that produce a toneless lowercase letter, e.g. `ơ` -> "ow" / "o7".
fn letter_keys(c: char, method: TypingMethod) -> Option<&'static str> {
    let telex = method != TypingMethod::VNI;
    Some(match c {
        'ă' => if telex { "aw" } else { "a8" },
        'â' => if telex { "aa" } else { "a6" },
        'ê' => if telex { "ee" } else { "e6" },
        'ô' => if telex { "oo" } else { "o6" },
        'ơ' => if telex { "ow" } else { "o7" },
        'ư' => if telex { "uw" } else { "u7" },
        'đ' => if telex { "dd" } else { "d9" },
        _ => return None,
    })
}

fn tone_key(tone: usize, method: TypingMethod) -> char {
    if method == TypingMethod::VNI {
        char::from_digit(tone as u32, 10).unwrap()
    } else {
        ['s', 'f', 'r', 'x', 'j'][tone - 1]
    }
}

/// Keys that type `word` with `method`, putting the tone key last.
///
/// The result is not guaranteed to round-trip (e.g. unusual vowel clusters),
/// so callers must check it by transforming it again.
pub fn respell(word: &str, method: TypingMethod) -> String {
    let mut keys = String::with_capacity(word.len() + 2);
    let mut tone = 0;
    let all_upper = word.chars().filter(|c| c.is_alphabetic()).all(char::is_uppercase);

    for c in word.chars() {
        let upper = c.is_uppercase();
        let lower = c.to_lowercase().next().unwrap_or(c);
        let (base, t) = split_tone(lower);
        if t != 0 {
            tone = t;
        }
        let spelled = letter_keys(base, method)
            .map(str::to_string)
            .unwrap_or_else(|| base.to_string());
        if upper {
            keys.push_str(&spelled.to_uppercase());
        } else {
            keys.push_str(&spelled);
        }
    }

    if tone != 0 {
        let key = tone_key(tone, method);
        keys.push(if all_upper { key.to_ascii_uppercase() } else { key });
    }
    keys
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn telex() {
        assert_eq!(respell("việ", TypingMethod::Telex), "vieej");
        assert_eq!(respell("tiế", TypingMethod::Telex), "tiees");
        assert_eq!(respell("đươ", TypingMethod::Telex), "dduwow");
        assert_eq!(respell("Đă", TypingMethod::Telex), "DDaw");
        assert_eq!(respell("VIỆ", TypingMethod::Telex), "VIEEJ");
        assert_eq!(respell("nam", TypingMethod::Telex), "nam");
    }

    #[test]
    fn vni() {
        assert_eq!(respell("việ", TypingMethod::VNI), "vie65");
        assert_eq!(respell("đươ", TypingMethod::VNI), "d9u7o7");
        assert_eq!(respell("hò", TypingMethod::VNI), "ho2");
    }
}
