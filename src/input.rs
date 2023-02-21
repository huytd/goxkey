use std::{collections::HashMap, fmt::Display, str::FromStr};

use druid::{Data, Target};
use log::debug;
use once_cell::sync::{Lazy, OnceCell};
use rdev::{Keyboard, KeyboardState};

use crate::{
    config::{CONFIG_MANAGER, HOTKEY_CONFIG_KEY, TYPING_METHOD_CONFIG_KEY},
    hotkey::Hotkey,
    ui::UPDATE_UI,
    UI_EVENT_SINK,
};

// According to Google search, the longest possible Vietnamese word
// is "nghiêng", which is 7 letters long. Add a little buffer for
// tone and marks, I guess the longest possible buffer length would
// be around 10 to 12.
const MAX_POSSIBLE_WORD_LENGTH: usize = 10;

const MAX_DUPLICATE_LENGTH: usize = 4;

const TONABLE_VOWELS: [char; 144] = [
    'a', 'à', 'ả', 'ã', 'á', 'ạ', 'ă', 'ằ', 'ẳ', 'ẵ', 'ắ', 'ặ', 'â', 'ầ', 'ẩ', 'ẫ', 'ấ', 'ậ', 'A',
    'À', 'Ả', 'Ã', 'Á', 'Ạ', 'Ă', 'Ằ', 'Ẳ', 'Ẵ', 'Ắ', 'Ặ', 'Â', 'Ầ', 'Ẩ', 'Ẫ', 'Ấ', 'Ậ', 'e', 'è',
    'ẻ', 'ẽ', 'é', 'ẹ', 'ê', 'ề', 'ể', 'ễ', 'ế', 'ệ', 'E', 'È', 'Ẻ', 'Ẽ', 'É', 'Ẹ', 'Ê', 'Ề', 'Ể',
    'Ễ', 'Ế', 'Ệ', 'i', 'ì', 'ỉ', 'ĩ', 'í', 'ị', 'I', 'Ì', 'Ỉ', 'Ĩ', 'Í', 'Ị', 'o', 'ò', 'ỏ', 'õ',
    'ó', 'ọ', 'ô', 'ồ', 'ổ', 'ỗ', 'ố', 'ộ', 'ơ', 'ờ', 'ở', 'ỡ', 'ớ', 'ợ', 'O', 'Ò', 'Ỏ', 'Õ', 'Ó',
    'Ọ', 'Ô', 'Ồ', 'Ổ', 'Ỗ', 'Ố', 'Ộ', 'Ơ', 'Ờ', 'Ở', 'Ỡ', 'Ớ', 'Ợ', 'u', 'ù', 'ủ', 'ũ', 'ú', 'ụ',
    'ư', 'ừ', 'ử', 'ữ', 'ứ', 'ự', 'U', 'Ù', 'Ủ', 'Ũ', 'Ú', 'Ụ', 'Ư', 'Ừ', 'Ử', 'Ữ', 'Ứ', 'Ự', 'y',
    'ỳ', 'ỷ', 'ỹ', 'ý', 'ỵ', 'Y', 'Ỳ', 'Ỷ', 'Ỹ', 'Ý', 'Ỵ',
];

pub static mut INPUT_STATE: Lazy<InputState> = Lazy::new(|| InputState::new());

pub const PREDEFINED_CHARS: [char; 47] = [
    'a', '`', '1', '2', '3', '4', '5', '6', '7', '8', '9', '0', '-', '=', 'q', 'w', 'e', 'r', 't',
    'y', 'u', 'i', 'o', 'p', '[', ']', 's', 'd', 'f', 'g', 'h', 'j', 'k', 'l', ';', '\'', '\\',
    'z', 'x', 'c', 'v', 'b', 'n', 'm', ',', '.', '/',
];

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
}

impl InputState {
    pub fn new() -> Self {
        let config = CONFIG_MANAGER.lock().unwrap();
        Self {
            buffer: String::new(),
            display_buffer: String::new(),
            method: TypingMethod::from_str(&config.read(TYPING_METHOD_CONFIG_KEY)).unwrap(),
            hotkey: Hotkey::from_str(&config.read(HOTKEY_CONFIG_KEY)),
            enabled: true,
            should_track: true,
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

    pub fn stop_tracking(&mut self) {
        self.clear();
        self.should_track = false;
    }

    pub fn toggle_vietnamese(&mut self) {
        self.enabled = !self.enabled;
        self.new_word();
    }

    pub fn set_method(&mut self, method: TypingMethod) {
        self.method = method;
        self.new_word();
        CONFIG_MANAGER
            .lock()
            .unwrap()
            .write(TYPING_METHOD_CONFIG_KEY, &method.to_string());
        if let Some(event_sink) = UI_EVENT_SINK.get() {
            _ = event_sink.submit_command(UPDATE_UI, (), Target::Auto);
        }
    }

    pub fn get_method(&self) -> TypingMethod {
        self.method
    }

    pub fn set_hotkey(&mut self, key_sequence: &str) {
        self.hotkey = Hotkey::from_str(key_sequence);
        CONFIG_MANAGER
            .lock()
            .unwrap()
            .write(HOTKEY_CONFIG_KEY, key_sequence);
        if let Some(event_sink) = UI_EVENT_SINK.get() {
            _ = event_sink.submit_command(UPDATE_UI, (), Target::Auto);
        }
    }

    pub fn get_hotkey(&self) -> &Hotkey {
        return &self.hotkey;
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

    pub fn transform_keys(&self) -> String {
        let mut output = String::new();
        let transform_method = match self.method {
            TypingMethod::VNI => vi::vni::transform_buffer,
            TypingMethod::Telex => vi::telex::transform_buffer,
        };
        transform_method(self.buffer.chars(), &mut output);
        return output;
    }

    pub fn should_send_keyboard_event(&self, word: &str) -> bool {
        !self.buffer.eq(word)
    }

    pub fn get_backspace_count(&self, is_delete: bool) -> usize {
        let dp_len = self.display_buffer.chars().count();
        if is_delete && dp_len >= 1 {
            dp_len
        } else {
            dp_len - 1
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
            if self.should_stop_tracking() {
                self.stop_tracking();
                debug!("! Stop tracking");
            }
        }
    }

    pub fn pop(&mut self) {
        self.buffer.pop();
        if self.buffer.is_empty() {
            self.new_word();
        }
    }

    pub fn clear(&mut self) {
        self.buffer.clear();
        self.display_buffer.clear();
    }

    // a set of rules that will trigger a hard stop for tracking
    // maybe these weird stuff should not be here, but let's
    // implement it anyway. we'll figure out where to put these
    // later on.
    pub fn should_stop_tracking(&mut self) -> bool {
        let len = self.buffer.len();
        if len >= MAX_DUPLICATE_LENGTH {
            let buf = &self.buffer[len - MAX_DUPLICATE_LENGTH..];
            let first = buf.chars().nth(0).unwrap();
            return buf.chars().all(|c| c == first);
        }
        return false;
    }
}
