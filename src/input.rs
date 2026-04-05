use std::collections::BTreeMap;
use std::{collections::HashMap, fmt::Display, str::FromStr};

use druid::{Data, Target};
use log::debug;
use once_cell::sync::{Lazy, OnceCell};
use rdev::{Keyboard, KeyboardState};
use vi::TransformResult;

use crate::platform::{get_active_app_name, KeyModifier};
use crate::{
    config::CONFIG_MANAGER, hotkey::Hotkey, platform::is_in_text_selection, ui::UPDATE_UI,
    UI_EVENT_SINK,
};

// According to Google search, the longest possible Vietnamese word
// is "nghiêng", which is 7 letters long. Add a little buffer for
// tone and marks, I guess the longest possible buffer length would
// be around 10 to 12.
const MAX_POSSIBLE_WORD_LENGTH: usize = 10;
const MAX_DUPLICATE_LENGTH: usize = 4;
const TONE_DUPLICATE_PATTERNS: [&str; 17] = [
    "ss", "ff", "jj", "rr", "xx", "ww", "kk", "tt", "nn", "mm", "yy", "hh", "ii", "aaa", "eee",
    "ooo", "ddd",
];

pub static mut INPUT_STATE: Lazy<InputState> = Lazy::new(InputState::new);
pub static mut HOTKEY_MODIFIERS: KeyModifier = KeyModifier::MODIFIER_NONE;
pub static mut HOTKEY_MATCHING: bool = false;
pub static mut HOTKEY_MATCHING_CIRCUIT_BREAK: bool = false;

pub const PREDEFINED_CHARS: [char; 47] = [
    'a', '`', '1', '2', '3', '4', '5', '6', '7', '8', '9', '0', '-', '=', 'q', 'w', 'e', 'r', 't',
    'y', 'u', 'i', 'o', 'p', '[', ']', 's', 'd', 'f', 'g', 'h', 'j', 'k', 'l', ';', '\'', '\\',
    'z', 'x', 'c', 'v', 'b', 'n', 'm', ',', '.', '/',
];

pub const STOP_TRACKING_WORDS: [&str; 4] = [";", "'", "?", "/"];
/// In w-literal mode, replace standalone 'w' with placeholder bytes that the telex
/// engine ignores (falls through to `_ => Transformation::Ignored`), then restore them
/// after transformation. A 'w' is "standalone" when NOT preceded by a Horn/Breve-eligible
/// vowel — those cases (uw→ư, ow→ơ, aw→ă) should still be handled by telex normally.
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

fn mask_standalone_w(buffer: &str) -> String {
    // Characters that can accept Horn (w) modification: u, o and all their toned forms.
    // Characters that can accept Breve (w) modification: a and all its toned forms.
    const HORN_BREVE_ELIGIBLE: &str =
        "uoaUOA\u{01b0}\u{01a1}\u{0103}\
         \u{00fa}\u{00f3}\u{00e1}\u{00f9}\u{00f2}\u{00e0}\
         \u{1ee7}\u{1ecf}\u{1ea3}\u{0169}\u{00f5}\u{00e3}\u{1ecd}\u{1ea1}\
         \u{00da}\u{00d3}\u{00c1}\u{00d9}\u{00d2}\u{00c0}\
         \u{1ee6}\u{1ece}\u{1ea2}\u{0168}\u{00d5}\u{00c3}\u{1ecc}\u{1ea0}";
    let chars: Vec<char> = buffer.chars().collect();
    let mut result = String::with_capacity(buffer.len() + 4);
    for (i, &ch) in chars.iter().enumerate() {
        if ch == 'w' || ch == 'W' {
            let preceded_by_eligible = i > 0 && HORN_BREVE_ELIGIBLE.contains(chars[i - 1]);
            // Also pass through when this 'w' follows a 'w' that was itself
            // preceded by an eligible vowel (e.g. "aww", "uww", "oww").
            // This lets telex see the full "ww" sequence and undo the
            // Horn/Breve modification, producing the raw text.
            let preceded_by_w_after_eligible = i >= 2
                && (chars[i - 1] == 'w' || chars[i - 1] == 'W')
                && HORN_BREVE_ELIGIBLE.contains(chars[i - 2]);
            if preceded_by_eligible || preceded_by_w_after_eligible {
                result.push(ch); // let telex transform it: uw→ư, ow→ơ, aw→ă, or ww→undo
            } else {
                // Mask it — telex ignores \x01/\x02, we restore them after transform
                result.push(if ch == 'w' { '\x01' } else { '\x02' });
            }
        } else {
            result.push(ch);
        }
    }
    result
}

pub fn get_key_from_char(c: char) -> rdev::Key {
    use rdev::Key::*;
    match &c {
        'a' => KeyA,
        '`' => BackQuote,
        '1' => Num1,
        '2' => Num2,
        '3' => Num3,
        '4' => Num4,
        '5' => Num5,
        '6' => Num6,
        '7' => Num7,
        '8' => Num8,
        '9' => Num9,
        '0' => Num0,
        '-' => Minus,
        '=' => Equal,
        'q' => KeyQ,
        'w' => KeyW,
        'e' => KeyE,
        'r' => KeyR,
        't' => KeyT,
        'y' => KeyY,
        'u' => KeyU,
        'i' => KeyI,
        'o' => KeyO,
        'p' => KeyP,
        '[' => LeftBracket,
        ']' => RightBracket,
        's' => KeyS,
        'd' => KeyD,
        'f' => KeyF,
        'g' => KeyG,
        'h' => KeyH,
        'j' => KeyJ,
        'k' => KeyK,
        'l' => KeyL,
        ';' => SemiColon,
        '\'' => Quote,
        '\\' => BackSlash,
        'z' => KeyZ,
        'x' => KeyX,
        'c' => KeyC,
        'v' => KeyV,
        'b' => KeyB,
        'n' => KeyN,
        'm' => KeyM,
        ',' => Comma,
        '.' => Dot,
        '/' => Slash,
        _ => Unknown(0),
    }
}

pub static mut KEYBOARD_LAYOUT_CHARACTER_MAP: OnceCell<HashMap<char, char>> = OnceCell::new();

fn build_keyboard_layout_map(map: &mut HashMap<char, char>) {
    map.clear();
    let mut kb = Keyboard::new().unwrap();
    for c in PREDEFINED_CHARS {
        let key = rdev::EventType::KeyPress(get_key_from_char(c));
        if let Some(s) = kb.add(&key) {
            let ch = s.chars().last().unwrap();
            map.insert(c, ch);
        }
    }
}

pub fn rebuild_keyboard_layout_map() {
    unsafe {
        if let Some(map) = KEYBOARD_LAYOUT_CHARACTER_MAP.get_mut() {
            debug!("Rebuild keyboard layout map...");
            build_keyboard_layout_map(map);
            debug!("Done");
        } else {
            debug!("Creating keyboard layout map...");
            let mut map = HashMap::new();
            build_keyboard_layout_map(&mut map);
            _ = KEYBOARD_LAYOUT_CHARACTER_MAP.set(map);
            debug!("Done");
        }
    }
}

#[allow(clippy::upper_case_acronyms)]
#[derive(PartialEq, Eq, Data, Clone, Copy)]
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

/// Compute the minimal edit needed to transform what is currently displayed (`old`)
/// into the desired output (`new`) by finding their longest common prefix.
///
/// Returns `(backspace_count, suffix)` where:
/// - `backspace_count` is the number of backspaces to send (to erase only the
///   diverging tail of `old`)
/// - `suffix` is the slice of `new` that must be typed after those backspaces
///
/// Both counts are in **Unicode scalar values** (chars), not bytes, because
/// each backspace deletes one displayed character regardless of its byte width.
/// The returned `suffix` is a byte slice of `new` starting at the first
/// diverging char — no allocation, no `.collect()`.
///
/// # Example
/// ```
/// // old = "mô"  (on screen after typing "moo")
/// // new = "mộ"  (engine output after pressing 'j' for nặng tone)
/// // common prefix = "m"  → only "ô" needs deleting, only "ộ" needs typing
/// let (bs, suffix) = get_diff_parts("mô", "mộ");
/// assert_eq!(bs, 1);
/// assert_eq!(suffix, "ộ");
/// ```
pub fn get_diff_parts<'a>(old: &str, new: &'a str) -> (usize, &'a str) {
    // Walk both strings char-by-char simultaneously.
    // We track the byte offset into `new` so we can return a zero-copy suffix slice.
    let mut old_chars = old.chars();
    let mut new_chars = new.char_indices();

    // Number of chars that are identical from the start.
    let mut common = 0usize;
    // Byte offset in `new` where divergence begins (used for the suffix slice).
    let mut diverge_byte = new.len(); // default: full match, empty suffix

    loop {
        match (old_chars.next(), new_chars.next()) {
            (Some(a), Some((byte_pos, b))) if a == b => {
                common += 1;
                diverge_byte = byte_pos + b.len_utf8();
            }
            (_, Some((byte_pos, _))) => {
                // Diverged — note byte position of the first differing char in `new`.
                diverge_byte = byte_pos;
                break;
            }
            (_, None) => {
                // `new` is a prefix of (or equal to) `old` — no suffix to type.
                diverge_byte = new.len();
                break;
            }
        }
    }

    // old_tail_len = number of chars in old that are NOT part of the common prefix.
    let old_len = old.chars().count();
    let backspace_count = old_len.saturating_sub(common);
    let suffix = &new[diverge_byte..];

    (backspace_count, suffix)
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

    pub fn update_active_app(&mut self) -> Option<()> {
        let current_active_app = get_active_app_name();
        // Only check if switch app
        if current_active_app == self.active_app {
            return None;
        }
        self.active_app = current_active_app;
        let config = CONFIG_MANAGER.lock().unwrap();
        // Only switch the input mode if we found the app in the config
        if config.is_vietnamese_app(&self.active_app) {
            self.enabled = true;
        }
        if config.is_english_app(&self.active_app) {
            self.enabled = false;
        }
        Some(())
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

    /// Mark that the previous word can be resumed if the user presses
    /// backspace immediately (i.e. the word was ended by space/tab/enter).
    pub fn mark_resumable(&mut self) {
        self.can_resume_previous_word = true;
    }

    /// Try to restore the previous word's buffers so editing can continue.
    /// Returns true if the word was resumed, false otherwise.
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
        // Exact match
        if let Some(target) = self.macro_table.get(&self.display_buffer) {
            return Some(target.clone());
        }
        // Auto-capitalize: try lowercase lookup, then apply cap pattern
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
        if let Some(event_sink) = UI_EVENT_SINK.get() {
            _ = event_sink.submit_command(UPDATE_UI, (), Target::Auto);
        }
    }

    pub fn get_method(&self) -> TypingMethod {
        self.method
    }

    pub fn set_hotkey(&mut self, key_sequence: &str) {
        self.hotkey = Hotkey::from_str(key_sequence);
        CONFIG_MANAGER.lock().unwrap().set_hotkey(key_sequence);
        if let Some(event_sink) = UI_EVENT_SINK.get() {
            _ = event_sink.submit_command(UPDATE_UI, (), Target::Auto);
        }
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

    pub fn should_transform_keys(&self, c: &char) -> bool {
        self.enabled
    }

    pub fn transform_keys(&self) -> Result<(String, TransformResult), ()> {
        // In w-literal mode (Telex only), replace standalone 'w' with a placeholder
        // before feeding to the telex engine, then restore it in the output.
        // A 'w' is considered standalone if NOT preceded by a Horn/Breve-eligible vowel
        // (u, o for Horn; a for Breve). This preserves uw→ư, ow→ơ, aw→ă etc.
        let effective_buffer = if self.is_w_literal_enabled
            && matches!(self.method, TypingMethod::Telex | TypingMethod::TelexVNI)
        {
            mask_standalone_w(&self.buffer)
        } else {
            self.buffer.clone()
        };

        if self.method == TypingMethod::TelexVNI {
            // Try both methods; prefer VNI when the buffer contains digits
            // (VNI's key differentiator), otherwise fall back to Telex.
            let buffer = effective_buffer;
            let result = std::panic::catch_unwind(move || {
                let has_digits = buffer.chars().any(|c| c.is_ascii_digit());
                if has_digits {
                    let mut output = String::new();
                    let transform_result = vi::vni::transform_buffer(buffer.chars(), &mut output);
                    (output, transform_result)
                } else {
                    let mut output = String::new();
                    let transform_result = vi::telex::transform_buffer(buffer.chars(), &mut output);
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
                TypingMethod::Telex | TypingMethod::TelexVNI => vi::telex::transform_buffer(buffer.chars(), &mut output),
            };
            // Restore masked standalone w's back to literal 'w'/'W'
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

    pub fn get_backspace_count(&self, is_delete: bool) -> usize {
        let dp_len = self.display_buffer.chars().count();
        let backspace_count = if is_delete && dp_len >= 1 {
            dp_len
        } else {
            dp_len - 1
        };

        if is_in_text_selection() {
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
        STOP_TRACKING_WORDS.contains(&self.previous_word.as_str())
    }

    pub fn should_stop_tracking(&mut self) -> bool {
        let len = self.buffer.len();
        if len > MAX_POSSIBLE_WORD_LENGTH {
            return true;
        }
        let buf = &self.buffer;
        if TONE_DUPLICATE_PATTERNS
            .iter()
            .find(|p| buf.to_ascii_lowercase().contains(*p))
            .is_some()
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
}

#[cfg(test)]
mod diff_tests {
    use super::get_diff_parts;

    // ── Basic tone application ────────────────────────────────────────────────

    /// "mô" → "mộ": only the vowel+tone char is replaced, "m" stays.
    #[test]
    fn tone_on_vowel_preserves_consonant_prefix() {
        let (bs, sfx) = get_diff_parts("mô", "mộ");
        assert_eq!(bs, 1, "should delete only 'ô'");
        assert_eq!(sfx, "ộ");
    }

    /// "mo" → "mô": typing 'o' again applies the circumflex.
    #[test]
    fn circumflex_application() {
        let (bs, sfx) = get_diff_parts("mo", "mô");
        assert_eq!(bs, 1);
        assert_eq!(sfx, "ô");
    }

    /// "tieng" → "tiếng": "ti" preserved, vowel+tone suffix replaced.
    #[test]
    fn multi_char_prefix_preserved() {
        let (bs, sfx) = get_diff_parts("tieng", "tiếng");
        assert_eq!(bs, 3); // "eng" deleted
        assert_eq!(sfx, "ếng");
    }

    /// "nguyen" → "nguyên": "nguy" is common.
    #[test]
    fn longer_common_prefix() {
        let (bs, sfx) = get_diff_parts("nguyen", "nguyên");
        assert_eq!(bs, 2); // "en" deleted
        assert_eq!(sfx, "ên");
    }

    // ── No-op / identical strings ─────────────────────────────────────────────

    /// Identical strings → 0 backspaces, empty suffix.
    #[test]
    fn identical_strings_no_op() {
        let (bs, sfx) = get_diff_parts("mộ", "mộ");
        assert_eq!(bs, 0);
        assert_eq!(sfx, "");
    }

    // ── Empty edge cases ──────────────────────────────────────────────────────

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

    // ── Prefix / suffix relationships ─────────────────────────────────────────

    /// new is a strict prefix of old: delete tail, type nothing.
    #[test]
    fn new_is_prefix_of_old() {
        let (bs, sfx) = get_diff_parts("mộng", "mộ");
        assert_eq!(bs, 2); // delete "ng"
        assert_eq!(sfx, "");
    }

    /// old is a strict prefix of new: 0 backspaces, append tail.
    #[test]
    fn old_is_prefix_of_new() {
        let (bs, sfx) = get_diff_parts("mộ", "mộng");
        assert_eq!(bs, 0);
        assert_eq!(sfx, "ng");
    }

    // ── Completely different strings ──────────────────────────────────────────

    #[test]
    fn no_common_prefix() {
        let (bs, sfx) = get_diff_parts("abc", "xyz");
        assert_eq!(bs, 3);
        assert_eq!(sfx, "xyz");
    }

    // ── Multi-byte / Unicode correctness ─────────────────────────────────────

    /// Each Vietnamese toned vowel is 1 char, possibly 3 bytes.
    /// backspace_count must be in chars, not bytes.
    #[test]
    fn char_count_not_byte_count() {
        let (bs, sfx) = get_diff_parts("ộ", "ô");
        assert_eq!(bs, 1, "one char deleted, not three bytes");
        assert_eq!(sfx, "ô");
    }

    #[test]
    fn all_multibyte_no_common_prefix() {
        let (bs, sfx) = get_diff_parts("ộ", "ể");
        assert_eq!(bs, 1);
        assert_eq!(sfx, "ể");
    }

    // ── Realistic Telex sequences ─────────────────────────────────────────────

    /// "moo" (buffer) → "mô" (engine output).
    #[test]
    fn telex_moo_to_mo_hat() {
        let (bs, sfx) = get_diff_parts("moo", "mô");
        assert_eq!(bs, 2);
        assert_eq!(sfx, "ô");
    }

    /// "cas" → "cá": "c" preserved.
    #[test]
    fn telex_cas_to_ca_sac() {
        let (bs, sfx) = get_diff_parts("cas", "cá");
        assert_eq!(bs, 2);
        assert_eq!(sfx, "á");
    }

    /// "viet" → "việt"
    #[test]
    fn telex_viet_transform() {
        let (bs, sfx) = get_diff_parts("viet", "việt");
        assert_eq!(bs, 2); // common = "vi"
        assert_eq!(sfx, "ệt");
    }

    /// Tone cycling: "tiến" → "tiền" (sắc → huyền), "ti" preserved.
    #[test]
    fn tone_cycling_preserves_prefix() {
        let (bs, sfx) = get_diff_parts("tiến", "tiền");
        assert_eq!(bs, 2);
        assert_eq!(sfx, "ền");
    }

    // ── Suffix slice is a zero-copy view into `new` ───────────────────────────

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
        // 'w' not preceded by eligible vowel → masked
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
        // aw→ă, uw→ư, ow→ơ should pass through
        assert_eq!(mask_standalone_w("aw"), "aw");
        assert_eq!(mask_standalone_w("uw"), "uw");
        assert_eq!(mask_standalone_w("ow"), "ow");
    }

    #[test]
    fn ww_after_eligible_vowel_not_masked() {
        // "aww" → both w's passed through so telex sees "ww" and undoes breve
        assert_eq!(mask_standalone_w("aww"), "aww");
        assert_eq!(mask_standalone_w("uww"), "uww");
        assert_eq!(mask_standalone_w("oww"), "oww");
        assert_eq!(mask_standalone_w("raww"), "raww");
    }

    #[test]
    fn standalone_ww_both_masked() {
        // "ww" with no eligible vowel before → both masked
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
        // Simulates: type "raww" → stop_tracking → backspace to empty → new_word
        let mut state = InputState::new();
        state.push('r');
        state.push('a');
        state.push('w');
        state.push('w');
        state.stop_tracking(); // triggered by "ww" pattern
        assert!(!state.is_tracking());
        assert!(state.is_buffer_empty());

        // Backspaces clear the screen (handled by OS), buffer already empty.
        // Calling new_word() re-enables tracking for the next keystrokes.
        state.new_word();
        assert!(state.is_tracking());

        // New characters should be tracked
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
        // Simulate end-of-word (space) → new_word + mark_resumable
        state.new_word();
        state.mark_resumable();
        assert!(state.is_buffer_empty());

        // Resume should restore the previous word
        assert!(state.try_resume_previous_word());
        assert!(state.is_tracking());
        assert_eq!(state.get_typing_buffer(), "test");
    }

}
