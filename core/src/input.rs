use std::collections::BTreeMap;
use std::fmt::Display;
use std::str::FromStr;

use log::debug;
use once_cell::sync::Lazy;
use vi::TransformResult;

use crate::config::CONFIG_MANAGER;
use crate::hotkey::Hotkey;
use crate::key_modifier::KeyModifier;

const MAX_POSSIBLE_WORD_LENGTH: usize = 10;
const TONE_DUPLICATE_PATTERNS: [&str; 17] = [
    "ss", "ff", "jj", "rr", "xx", "ww", "kk", "tt", "nn", "mm", "yy", "hh", "ii", "aaa", "eee",
    "ooo", "ddd",
];

pub static mut INPUT_STATE: Lazy<InputState> = Lazy::new(InputState::new);
pub static mut HOTKEY_MODIFIERS: KeyModifier = KeyModifier::MODIFIER_NONE;
pub static mut HOTKEY_MATCHING: bool = false;
pub static mut HOTKEY_MATCHING_CIRCUIT_BREAK: bool = false;

pub const STOP_TRACKING_WORDS: [&str; 4] = [";", "'", "?", "/"];

enum CapPattern {
    Lower,
    TitleCase,
    AllCaps,
}

fn detect_cap_pattern(s: &str) -> CapPattern {
    let mut chars = s.chars().filter(|c| c.is_alphabetic());
    match chars.next() {
        Some(first) if first.is_uppercase() => {
            if chars.all(|c| c.is_uppercase()) {
                CapPattern::AllCaps
            } else {
                CapPattern::TitleCase
            }
        }
        _ => CapPattern::Lower,
    }
}

fn apply_cap_pattern(s: &str, pattern: CapPattern) -> String {
    match pattern {
        CapPattern::Lower => s.to_string(),
        CapPattern::AllCaps => s.to_uppercase(),
        CapPattern::TitleCase => {
            let mut chars = s.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first.to_uppercase().to_string() + chars.as_str(),
            }
        }
    }
}

fn contains_case_insensitive_ascii(haystack: &str, needle: &str) -> bool {
    if needle.is_empty() {
        return true;
    }
    if needle.len() > haystack.len() {
        return false;
    }
    let needle = needle.as_bytes();
    haystack
        .as_bytes()
        .windows(needle.len())
        .any(|w| w.iter().zip(needle).all(|(a, b)| a.eq_ignore_ascii_case(b)))
}

fn mask_standalone_w(buffer: &str) -> String {
    const HORN_BREVE_ELIGIBLE: &str = "uoaUOA\u{01b0}\u{01a1}\u{0103}\
         \u{00fa}\u{00f3}\u{00e1}\u{00f9}\u{00f2}\u{00e0}\
         \u{1ee7}\u{1ecf}\u{1ea3}\u{0169}\u{00f5}\u{00e3}\u{1ecd}\u{1ea1}\
         \u{00da}\u{00d3}\u{00c1}\u{00d9}\u{00d2}\u{00c0}\
         \u{1ee6}\u{1ece}\u{1ea2}\u{0168}\u{00d5}\u{00c3}\u{1ecc}\u{1ea0}";
    let chars: Vec<char> = buffer.chars().collect();
    let mut result = String::with_capacity(buffer.len() + 4);
    for (i, &ch) in chars.iter().enumerate() {
        if ch == 'w' || ch == 'W' {
            let preceded_by_eligible = i > 0 && HORN_BREVE_ELIGIBLE.contains(chars[i - 1]);
            let preceded_by_w_after_eligible = i >= 2
                && (chars[i - 1] == 'w' || chars[i - 1] == 'W')
                && HORN_BREVE_ELIGIBLE.contains(chars[i - 2]);
            if preceded_by_eligible || preceded_by_w_after_eligible {
                result.push(ch);
            } else {
                result.push(if ch == 'w' { '\x01' } else { '\x02' });
            }
        } else {
            result.push(ch);
        }
    }
    result
}

/// Compute the minimal edit needed to transform what is currently displayed (`old`)
/// into the desired output (`new`) by finding their longest common prefix.
pub fn get_diff_parts<'a>(old: &str, new: &'a str) -> (usize, &'a str) {
    let mut old_chars = old.chars();
    let mut new_chars = new.char_indices();

    let mut common = 0usize;
    let diverge_byte = loop {
        match (old_chars.next(), new_chars.next()) {
            (Some(a), Some((_, b))) if a == b => {
                common += 1;
            }
            (_, Some((byte_pos, _))) => break byte_pos,
            (_, None) => break new.len(),
        }
    };

    let old_len = old.chars().count();
    let backspace_count = old_len.saturating_sub(common);
    let suffix = &new[diverge_byte..];

    (backspace_count, suffix)
}

#[allow(clippy::upper_case_acronyms)]
#[cfg_attr(feature = "druid", derive(druid::Data))]
#[derive(PartialEq, Eq, Clone, Copy)]
pub enum TypingMethod {
    VNI,
    Telex,
    TelexVNI,
}

impl FromStr for TypingMethod {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s.to_ascii_lowercase().as_str() {
            "vni" => TypingMethod::VNI,
            "telexvni" => TypingMethod::TelexVNI,
            _ => TypingMethod::Telex,
        })
    }
}

impl Display for TypingMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::VNI => "vni",
                Self::Telex => "telex",
                Self::TelexVNI => "telexvni",
            }
        )
    }
}

pub struct InputState {
    buffer: String,
    display_buffer: String,
    method: TypingMethod,
    hotkey: Hotkey,
    enabled: bool,
    should_track: bool,
    previous_word: String,
    previous_display: String,
    can_resume_previous_word: bool,
    active_app: String,
    is_macro_enabled: bool,
    is_macro_autocap_enabled: bool,
    macro_table: BTreeMap<String, String>,
    temporary_disabled: bool,
    previous_modifiers: KeyModifier,
    is_auto_toggle_enabled: bool,
    is_gox_mode_enabled: bool,
    is_w_literal_enabled: bool,
}

impl InputState {
    pub fn new() -> Self {
        let config = CONFIG_MANAGER.lock().unwrap();
        Self {
            buffer: String::new(),
            display_buffer: String::new(),
            method: TypingMethod::from_str(config.get_method()).unwrap(),
            hotkey: Hotkey::from_str(config.get_hotkey()),
            enabled: true,
            should_track: true,
            previous_word: String::new(),
            previous_display: String::new(),
            can_resume_previous_word: false,
            active_app: String::new(),
            is_macro_enabled: config.is_macro_enabled(),
            is_macro_autocap_enabled: config.is_macro_autocap_enabled(),
            macro_table: config.get_macro_table().clone(),
            temporary_disabled: false,
            previous_modifiers: KeyModifier::empty(),
            is_auto_toggle_enabled: config.is_auto_toggle_enabled(),
            is_gox_mode_enabled: config.is_gox_mode_enabled(),
            is_w_literal_enabled: config.is_w_literal_enabled(),
        }
    }

    pub fn update_active_app(&mut self, current_active_app: &str) -> Option<()> {
        if current_active_app == self.active_app {
            return None;
        }
        self.active_app = current_active_app.to_string();
        let config = CONFIG_MANAGER.lock().unwrap();
        if config.is_vietnamese_app(&self.active_app) {
            self.enabled = true;
        }
        if config.is_english_app(&self.active_app) {
            self.enabled = false;
        }
        Some(())
    }

    pub fn active_app(&self) -> &str {
        &self.active_app
    }

    pub fn set_temporary_disabled(&mut self) {
        self.temporary_disabled = true;
    }

    pub fn is_gox_mode_enabled(&self) -> bool {
        self.is_gox_mode_enabled
    }

    pub fn is_w_literal_enabled(&self) -> bool {
        self.is_w_literal_enabled
    }

    pub fn toggle_w_literal(&mut self) {
        self.is_w_literal_enabled = !self.is_w_literal_enabled;
        CONFIG_MANAGER
            .lock()
            .unwrap()
            .set_w_literal_enabled(self.is_w_literal_enabled);
    }

    pub fn is_enabled(&self) -> bool {
        !self.temporary_disabled && self.enabled
    }

    pub fn is_tracking(&self) -> bool {
        self.should_track
    }

    pub fn is_buffer_empty(&self) -> bool {
        self.buffer.is_empty()
    }

    pub fn new_word(&mut self) {
        if !self.buffer.is_empty() {
            self.clear();
        }
        if self.temporary_disabled {
            self.temporary_disabled = false;
        }
        self.should_track = true;
        self.can_resume_previous_word = false;
    }

    pub fn mark_resumable(&mut self) {
        self.can_resume_previous_word = true;
    }

    pub fn try_resume_previous_word(&mut self) -> bool {
        if !self.can_resume_previous_word || self.previous_word.is_empty() {
            return false;
        }
        self.buffer = self.previous_word.clone();
        self.display_buffer = self.previous_display.clone();
        self.should_track = true;
        self.can_resume_previous_word = false;
        true
    }

    pub fn get_macro_target(&self) -> Option<String> {
        if !self.is_macro_enabled {
            return None;
        }
        if let Some(target) = self.macro_table.get(&self.display_buffer) {
            return Some(target.clone());
        }
        if self.is_macro_autocap_enabled {
            let lower = self.display_buffer.to_lowercase();
            if let Some(target) = self.macro_table.get(&lower) {
                let pattern = detect_cap_pattern(&self.display_buffer);
                return Some(apply_cap_pattern(target, pattern));
            }
        }
        None
    }

    pub fn is_macro_autocap_enabled(&self) -> bool {
        self.is_macro_autocap_enabled
    }

    pub fn toggle_macro_autocap(&mut self) {
        self.is_macro_autocap_enabled = !self.is_macro_autocap_enabled;
        CONFIG_MANAGER
            .lock()
            .unwrap()
            .set_macro_autocap_enabled(self.is_macro_autocap_enabled);
    }

    pub fn get_typing_buffer(&self) -> &str {
        &self.buffer
    }

    pub fn get_displaying_word(&self) -> &str {
        &self.display_buffer
    }

    pub fn stop_tracking(&mut self) {
        self.clear();
        self.should_track = false;
    }

    pub fn toggle_vietnamese(&mut self) {
        self.enabled = !self.enabled;
        self.temporary_disabled = false;
        let mut config = CONFIG_MANAGER.lock().unwrap();
        if self.enabled {
            config.add_vietnamese_app(&self.active_app);
        } else {
            config.add_english_app(&self.active_app);
        }
        self.new_word();
    }

    pub fn add_vietnamese_app(&mut self, app_name: &str) {
        CONFIG_MANAGER.lock().unwrap().add_vietnamese_app(app_name);
    }

    pub fn add_english_app(&mut self, app_name: &str) {
        CONFIG_MANAGER.lock().unwrap().add_english_app(app_name);
    }

    pub fn remove_vietnamese_app(&mut self, app_name: &str) {
        CONFIG_MANAGER
            .lock()
            .unwrap()
            .remove_vietnamese_app(app_name);
    }

    pub fn remove_english_app(&mut self, app_name: &str) {
        CONFIG_MANAGER.lock().unwrap().remove_english_app(app_name);
    }

    pub fn get_vn_apps(&self) -> Vec<String> {
        CONFIG_MANAGER.lock().unwrap().get_vn_apps()
    }

    pub fn get_en_apps(&self) -> Vec<String> {
        CONFIG_MANAGER.lock().unwrap().get_en_apps()
    }

    pub fn set_method(&mut self, method: TypingMethod) {
        self.method = method;
        self.new_word();
        CONFIG_MANAGER
            .lock()
            .unwrap()
            .set_method(&method.to_string());
    }

    pub fn get_method(&self) -> TypingMethod {
        self.method
    }

    pub fn set_hotkey(&mut self, key_sequence: &str) {
        self.hotkey = Hotkey::from_str(key_sequence);
        CONFIG_MANAGER.lock().unwrap().set_hotkey(key_sequence);
    }

    pub fn get_hotkey(&self) -> &Hotkey {
        &self.hotkey
    }

    pub fn is_auto_toggle_enabled(&self) -> bool {
        self.is_auto_toggle_enabled
    }

    pub fn toggle_auto_toggle(&mut self) {
        self.is_auto_toggle_enabled = !self.is_auto_toggle_enabled;
        CONFIG_MANAGER
            .lock()
            .unwrap()
            .set_auto_toggle_enabled(self.is_auto_toggle_enabled);
    }

    pub fn is_macro_enabled(&self) -> bool {
        self.is_macro_enabled
    }

    pub fn toggle_macro_enabled(&mut self) {
        self.is_macro_enabled = !self.is_macro_enabled;
        CONFIG_MANAGER
            .lock()
            .unwrap()
            .set_macro_enabled(self.is_macro_enabled);
    }

    pub fn get_macro_table(&self) -> &BTreeMap<String, String> {
        &self.macro_table
    }

    pub fn delete_macro(&mut self, from: &String) {
        self.macro_table.remove(from);
        CONFIG_MANAGER.lock().unwrap().delete_macro(from);
    }

    pub fn add_macro(&mut self, from: String, to: String) {
        CONFIG_MANAGER
            .lock()
            .unwrap()
            .add_macro(from.clone(), to.clone());
        self.macro_table.insert(from, to);
    }

    pub fn export_macros_to_file(&self, path: &str) -> std::io::Result<()> {
        use crate::config::build_kv_string;
        use std::fs::File;
        use std::io::Write;
        let mut file = File::create(path)?;
        for (k, v) in &self.macro_table {
            writeln!(file, "{}", build_kv_string(k, v))?;
        }
        Ok(())
    }

    pub fn import_macros_from_file(&mut self, path: &str) -> std::io::Result<usize> {
        use crate::config::parse_kv_string;
        use std::fs::File;
        use std::io::{BufRead, BufReader};
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        let mut count = 0;
        for line in reader.lines() {
            let line = line?;
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            if let Some((from, to)) = parse_kv_string(line) {
                self.add_macro(from, to);
                count += 1;
            }
        }
        Ok(count)
    }

    pub fn transform_keys(&self) -> Result<(String, TransformResult), ()> {
        let effective_buffer = if self.is_w_literal_enabled
            && matches!(self.method, TypingMethod::Telex | TypingMethod::TelexVNI)
        {
            mask_standalone_w(&self.buffer)
        } else {
            self.buffer.clone()
        };

        if self.method == TypingMethod::TelexVNI {
            let buffer = effective_buffer;
            let result = std::panic::catch_unwind(move || {
                let has_digits = buffer.chars().any(|c| c.is_ascii_digit());
                if has_digits {
                    let mut output = String::new();
                    let transform_result = vi::vni::transform_buffer(buffer.chars(), &mut output);
                    (output, transform_result)
                } else {
                    let mut output = String::new();
                    let transform_result =
                        vi::telex::transform_buffer(buffer.chars(), &mut output);
                    let output = output.replace('\x01', "w").replace('\x02', "W");
                    (output, transform_result)
                }
            });
            return result.map_err(|_| ());
        }

        let method = self.method;
        let buffer = effective_buffer;
        let is_w_literal = self.is_w_literal_enabled;
        let result = std::panic::catch_unwind(move || {
            let mut output = String::new();
            let transform_result = match method {
                TypingMethod::VNI => vi::vni::transform_buffer(buffer.chars(), &mut output),
                TypingMethod::Telex | TypingMethod::TelexVNI => {
                    vi::telex::transform_buffer(buffer.chars(), &mut output)
                }
            };
            let output = if is_w_literal {
                output.replace('\x01', "w").replace('\x02', "W")
            } else {
                output
            };
            (output, transform_result)
        });
        if let Ok((output, transform_result)) = result {
            return Ok((output, transform_result));
        }
        Err(())
    }

    pub fn should_send_keyboard_event(&self, word: &str) -> bool {
        !self.display_buffer.eq(word)
    }

    pub fn should_dismiss_selection_if_needed(&self) -> bool {
        const DISMISS_APPS: [&str; 3] = ["Firefox", "Floorp", "Zen"];
        return DISMISS_APPS.iter().any(|app| self.active_app.contains(app));
    }

    pub fn get_backspace_count(&self, is_delete: bool, in_text_selection: bool) -> usize {
        let dp_len = self.display_buffer.chars().count();
        let backspace_count = if is_delete && dp_len >= 1 {
            dp_len
        } else {
            dp_len - 1
        };

        if in_text_selection {
            backspace_count + 1
        } else {
            backspace_count
        }
    }

    pub fn replace(&mut self, buf: String) {
        self.display_buffer = buf;
    }

    pub fn push(&mut self, c: char) {
        if let Some(first_char) = self.buffer.chars().next() {
            if first_char.is_numeric() {
                self.buffer.remove(0);
                self.display_buffer.remove(0);
            }
        }
        if self.buffer.len() <= MAX_POSSIBLE_WORD_LENGTH {
            self.buffer.push(c);
            self.display_buffer.push(c);
            debug!(
                "Input buffer: {:?} - Display buffer: {:?}",
                self.buffer, self.display_buffer
            );
        }
    }

    pub fn pop(&mut self) {
        self.buffer.pop();
        if self.buffer.is_empty() {
            self.display_buffer.clear();
            self.new_word();
        }
    }

    pub fn clear(&mut self) {
        self.previous_word = self.buffer.to_owned();
        self.previous_display = self.display_buffer.to_owned();
        self.buffer.clear();
        self.display_buffer.clear();
    }

    pub fn get_previous_word(&self) -> &str {
        &self.previous_word
    }

    pub fn clear_previous_word(&mut self) {
        self.previous_word.clear();
    }

    pub fn previous_word_is_stop_tracking_words(&self) -> bool {
        if self.previous_word.len() != 1 {
            return false;
        }
        matches!(
            self.previous_word.as_bytes()[0],
            b';' | b'\'' | b'?' | b'/'
        )
    }

    pub fn should_stop_tracking(&mut self) -> bool {
        let len = self.buffer.len();
        if len > MAX_POSSIBLE_WORD_LENGTH {
            return true;
        }
        if TONE_DUPLICATE_PATTERNS
            .iter()
            .any(|p| contains_case_insensitive_ascii(&self.buffer, p))
        {
            return true;
        }

        if self.previous_word_is_stop_tracking_words() {
            return true;
        }

        false
    }

    pub fn stop_tracking_if_needed(&mut self) {
        if self.should_stop_tracking() {
            self.stop_tracking();
            debug!("! Stop tracking");
        }
    }

    pub fn get_previous_modifiers(&self) -> KeyModifier {
        self.previous_modifiers
    }

    pub fn save_previous_modifiers(&mut self, modifiers: KeyModifier) {
        self.previous_modifiers = modifiers;
    }

    pub fn is_allowed_word(&self, word: &str) -> bool {
        let config = CONFIG_MANAGER.lock().unwrap();
        return config.is_allowed_word(word);
    }

    pub fn should_restore_word(&self) -> bool {
        let typing_buffer = self.get_typing_buffer();
        let display_buffer = self.get_displaying_word();

        let is_transformed_word = typing_buffer != display_buffer;
        if !is_transformed_word {
            return false;
        }

        let is_valid_word = vi::validation::is_valid_word(display_buffer);
        if is_valid_word {
            return false;
        }

        if self.is_allowed_word(display_buffer) {
            return false;
        }

        let is_vni_numeric_shortcut =
            self.method == TypingMethod::VNI && typing_buffer.chars().any(|c| c.is_numeric());
        !is_vni_numeric_shortcut
    }
}

#[cfg(test)]
mod diff_tests {
    use super::get_diff_parts;

    #[test]
    fn tone_on_vowel_preserves_consonant_prefix() {
        let (bs, sfx) = get_diff_parts("mô", "mộ");
        assert_eq!(bs, 1);
        assert_eq!(sfx, "ộ");
    }

    #[test]
    fn circumflex_application() {
        let (bs, sfx) = get_diff_parts("mo", "mô");
        assert_eq!(bs, 1);
        assert_eq!(sfx, "ô");
    }

    #[test]
    fn multi_char_prefix_preserved() {
        let (bs, sfx) = get_diff_parts("tieng", "tiếng");
        assert_eq!(bs, 3);
        assert_eq!(sfx, "ếng");
    }

    #[test]
    fn longer_common_prefix() {
        let (bs, sfx) = get_diff_parts("nguyen", "nguyên");
        assert_eq!(bs, 2);
        assert_eq!(sfx, "ên");
    }

    #[test]
    fn identical_strings_no_op() {
        let (bs, sfx) = get_diff_parts("mộ", "mộ");
        assert_eq!(bs, 0);
        assert_eq!(sfx, "");
    }

    #[test]
    fn both_empty() {
        let (bs, sfx) = get_diff_parts("", "");
        assert_eq!(bs, 0);
        assert_eq!(sfx, "");
    }

    #[test]
    fn old_empty_new_nonempty() {
        let (bs, sfx) = get_diff_parts("", "mộ");
        assert_eq!(bs, 0);
        assert_eq!(sfx, "mộ");
    }

    #[test]
    fn old_nonempty_new_empty() {
        let (bs, sfx) = get_diff_parts("mô", "");
        assert_eq!(bs, 2);
        assert_eq!(sfx, "");
    }

    #[test]
    fn new_is_prefix_of_old() {
        let (bs, sfx) = get_diff_parts("mộng", "mộ");
        assert_eq!(bs, 2);
        assert_eq!(sfx, "");
    }

    #[test]
    fn old_is_prefix_of_new() {
        let (bs, sfx) = get_diff_parts("mộ", "mộng");
        assert_eq!(bs, 0);
        assert_eq!(sfx, "ng");
    }

    #[test]
    fn no_common_prefix() {
        let (bs, sfx) = get_diff_parts("abc", "xyz");
        assert_eq!(bs, 3);
        assert_eq!(sfx, "xyz");
    }

    #[test]
    fn char_count_not_byte_count() {
        let (bs, sfx) = get_diff_parts("ộ", "ô");
        assert_eq!(bs, 1);
        assert_eq!(sfx, "ô");
    }

    #[test]
    fn all_multibyte_no_common_prefix() {
        let (bs, sfx) = get_diff_parts("ộ", "ể");
        assert_eq!(bs, 1);
        assert_eq!(sfx, "ể");
    }

    #[test]
    fn telex_moo_to_mo_hat() {
        let (bs, sfx) = get_diff_parts("moo", "mô");
        assert_eq!(bs, 2);
        assert_eq!(sfx, "ô");
    }

    #[test]
    fn telex_cas_to_ca_sac() {
        let (bs, sfx) = get_diff_parts("cas", "cá");
        assert_eq!(bs, 2);
        assert_eq!(sfx, "á");
    }

    #[test]
    fn telex_viet_transform() {
        let (bs, sfx) = get_diff_parts("viet", "việt");
        assert_eq!(bs, 2);
        assert_eq!(sfx, "ệt");
    }

    #[test]
    fn tone_cycling_preserves_prefix() {
        let (bs, sfx) = get_diff_parts("tiến", "tiền");
        assert_eq!(bs, 2);
        assert_eq!(sfx, "ền");
    }

    #[test]
    fn suffix_is_valid_utf8_slice_of_new() {
        let new = "nguyên";
        let (_, sfx) = get_diff_parts("nguyen", new);
        let new_start = new.as_ptr() as usize;
        let sfx_start = sfx.as_ptr() as usize;
        assert!(sfx_start >= new_start);
        assert!(sfx_start + sfx.len() <= new_start + new.len());
        assert_eq!(sfx, "ên");
    }
}

#[cfg(test)]
mod mask_w_tests {
    use super::mask_standalone_w;

    #[test]
    fn standalone_w_is_masked() {
        assert_eq!(mask_standalone_w("w"), "\x01");
        assert_eq!(mask_standalone_w("rw"), "r\x01");
    }

    #[test]
    fn standalone_upper_w_is_masked() {
        assert_eq!(mask_standalone_w("W"), "\x02");
        assert_eq!(mask_standalone_w("RW"), "R\x02");
    }

    #[test]
    fn w_after_eligible_vowel_is_not_masked() {
        assert_eq!(mask_standalone_w("aw"), "aw");
        assert_eq!(mask_standalone_w("uw"), "uw");
        assert_eq!(mask_standalone_w("ow"), "ow");
    }

    #[test]
    fn ww_after_eligible_vowel_not_masked() {
        assert_eq!(mask_standalone_w("aww"), "aww");
        assert_eq!(mask_standalone_w("uww"), "uww");
        assert_eq!(mask_standalone_w("oww"), "oww");
        assert_eq!(mask_standalone_w("raww"), "raww");
    }

    #[test]
    fn standalone_ww_both_masked() {
        assert_eq!(mask_standalone_w("ww"), "\x01\x01");
        assert_eq!(mask_standalone_w("rww"), "r\x01\x01");
    }

    #[test]
    fn mixed_case_ww_after_eligible() {
        assert_eq!(mask_standalone_w("aWW"), "aWW");
        assert_eq!(mask_standalone_w("AWw"), "AWw");
    }
}

#[cfg(test)]
mod tracking_tests {
    use super::InputState;

    #[test]
    fn stop_tracking_disables_tracking() {
        let mut state = InputState::new();
        state.push('r');
        assert!(state.is_tracking());
        state.stop_tracking();
        assert!(!state.is_tracking());
        assert!(state.is_buffer_empty());
    }

    #[test]
    fn new_word_re_enables_tracking_after_stop() {
        let mut state = InputState::new();
        state.push('r');
        state.stop_tracking();
        assert!(!state.is_tracking());
        state.new_word();
        assert!(state.is_tracking());
    }

    #[test]
    fn pop_to_empty_then_new_word_re_enables_tracking() {
        let mut state = InputState::new();
        state.push('r');
        state.push('a');
        state.push('w');
        state.push('w');
        state.stop_tracking();
        assert!(!state.is_tracking());
        assert!(state.is_buffer_empty());

        state.new_word();
        assert!(state.is_tracking());

        state.push('o');
        state.push('o');
        assert_eq!(state.get_typing_buffer(), "oo");
    }

    #[test]
    fn resume_previous_word_re_enables_tracking() {
        let mut state = InputState::new();
        state.push('t');
        state.push('e');
        state.push('s');
        state.push('t');
        state.new_word();
        state.mark_resumable();
        assert!(state.is_buffer_empty());

        assert!(state.try_resume_previous_word());
        assert!(state.is_tracking());
        assert_eq!(state.get_typing_buffer(), "test");
    }
}
