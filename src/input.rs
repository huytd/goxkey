use std::collections::BTreeMap;
use std::{collections::HashMap, fmt::Display, str::FromStr};

use druid::{Data, Target};
use log::debug;
use once_cell::sync::{Lazy, OnceCell};
use rdev::{Keyboard, KeyboardState};

use crate::platform::get_active_app_name;
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
const TONE_DUPLICATE_PATTERNS: [&str; 16] = [
    "ss", "ff", "jj", "rr", "xx", "ww", "kk", "tt", "nn", "mm", "yy", "hh", "ii", "aaa", "eee",
    "ooo",
];

pub static mut INPUT_STATE: Lazy<InputState> = Lazy::new(InputState::new);

pub const PREDEFINED_CHARS: [char; 47] = [
    'a', '`', '1', '2', '3', '4', '5', '6', '7', '8', '9', '0', '-', '=', 'q', 'w', 'e', 'r', 't',
    'y', 'u', 'i', 'o', 'p', '[', ']', 's', 'd', 'f', 'g', 'h', 'j', 'k', 'l', ';', '\'', '\\',
    'z', 'x', 'c', 'v', 'b', 'n', 'm', ',', '.', '/',
];

pub const STOP_TRACKING_WORDS: [&str; 4] = [";", "'", "?", "/"];

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
            // debug!("Rebuild keyboard layout map...");
            build_keyboard_layout_map(map);
            // debug!("Done");
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
}

impl FromStr for TypingMethod {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s.to_ascii_lowercase().as_str() {
            "vni" => TypingMethod::VNI,
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
    active_app: String,
    is_macro_enabled: bool,
    macro_table: BTreeMap<String, String>,
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
            active_app: String::new(),
            is_macro_enabled: true,
            macro_table: config.get_macro_table().clone(),
        }
    }

    pub fn update_active_app(&mut self) {
        self.active_app = get_active_app_name();
        let config = CONFIG_MANAGER.lock().unwrap();
        // Only switch the input mode if we found the app in the config
        if config.is_vietnamese_app(&self.active_app) {
            self.enabled = true;
        }
        if config.is_english_app(&self.active_app) {
            self.enabled = false;
        }
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
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
        self.should_track = true;
    }

    pub fn get_macro_target(&self) -> Option<&String> {
        if !self.is_macro_enabled {
            return None;
        }
        self.macro_table.get(&self.display_buffer)
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
        let mut config = CONFIG_MANAGER.lock().unwrap();
        if self.enabled {
            config.add_vietnamese_app(&self.active_app);
        } else {
            config.add_english_app(&self.active_app);
        }
        self.new_word();
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

    pub fn is_macro_enabled(&self) -> bool {
        self.is_macro_enabled
    }

    pub fn toggle_macro_enabled(&mut self) {
        self.is_macro_enabled = !self.is_macro_enabled
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

    pub fn should_transform_keys(&self, c: &char) -> bool {
        self.enabled
            && match self.method {
                TypingMethod::VNI => c.is_numeric(),
                TypingMethod::Telex => {
                    ['a', 'e', 'o', 'd', 's', 't', 'j', 'f', 'x', 'r', 'w', 'z'].contains(c)
                }
            }
    }

    pub fn transform_keys(&self) -> Result<String, ()> {
        let transform_method = match self.method {
            TypingMethod::VNI => vi::vni::transform_buffer,
            TypingMethod::Telex => vi::telex::transform_buffer,
        };
        let result = std::panic::catch_unwind(|| {
            let mut output = String::new();
            transform_method(self.buffer.chars(), &mut output);
            output
        });
        if let Ok(output) = result {
            return Ok(output);
        }
        Err(())
    }

    pub fn should_send_keyboard_event(&self, word: &str) -> bool {
        !self.buffer.eq(word)
    }

    pub fn should_dismiss_selection_if_needed(&self) -> bool {
        return self.active_app.contains("Firefox");
    }

    pub fn get_backspace_count(&self, is_delete: bool) -> usize {
        let dp_len = self.display_buffer.chars().count();
        let backspace_count = if is_delete && dp_len >= 1 {
            dp_len
        } else {
            dp_len - 1
        };

        // Add an extra backspace to compensate the initial text selection deletion.
        // This is useful in applications like chrome, where the URL bar uses text selection
        // for autocompletion, causing the first backspace to delete the selection instead of
        // the character behind the cursor.
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
        self.display_buffer.pop();
        self.buffer = self.display_buffer.clone();
        if self.buffer.is_empty() {
            self.new_word();
        }
    }

    pub fn clear(&mut self) {
        self.previous_word = self.buffer.to_owned();
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

    // a set of rules that will trigger a hard stop for tracking
    // maybe these weird stuff should not be here, but let's
    // implement it anyway. we'll figure out where to put these
    // later on.
    pub fn should_stop_tracking(&mut self) -> bool {
        let len = self.buffer.len();
        if len > MAX_POSSIBLE_WORD_LENGTH {
            return true;
        }
        // detect attempts to restore a word
        // by doubling tone marks like ss, rr, ff, jj, xx
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
}
