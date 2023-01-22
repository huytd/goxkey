use druid::Data;
use once_cell::sync::Lazy;
use std::sync::Mutex;

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

#[derive(PartialEq, Eq, Data, Clone, Copy)]
pub enum TypingMethod {
    VNI,
    Telex,
}

pub struct InputState {
    pub buffer: String,
    pub display_buffer: String,
    pub method: TypingMethod,
    pub enabled: bool,
    pub should_track: bool,
}

impl InputState {
    pub fn new() -> Self {
        Self {
            buffer: String::new(),
            display_buffer: String::new(),
            method: TypingMethod::Telex,
            enabled: true,
            should_track: true,
        }
    }

    pub fn new_word(&mut self) {
        self.clear();
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
    }

    pub fn should_process(&self, c: &char) -> bool {
        self.enabled
            && match self.method {
                TypingMethod::VNI => c.is_numeric(),
                TypingMethod::Telex => {
                    ['a', 'e', 'o', 'd', 's', 't', 'j', 'f', 'x', 'r', 'w'].contains(c)
                }
            }
    }

    pub fn process_key(&self) -> String {
        let mut output = String::new();
        let transform_method = match self.method {
            TypingMethod::VNI => vi::vni::transform_buffer,
            TypingMethod::Telex => vi::telex::transform_buffer,
        };
        transform_method(self.buffer.chars(), &mut output);
        return output;
    }

    pub fn get_backspace_count(&self) -> usize {
        self.display_buffer.chars().count() - 1
    }

    pub fn replace(&mut self, buf: String) {
        self.display_buffer = buf;
    }

    pub fn push(&mut self, c: char) {
        if self.buffer.len() <= MAX_POSSIBLE_WORD_LENGTH {
            self.buffer.push(c);
            self.display_buffer.push(c);
            if self.should_stop_tracking() {
                self.stop_tracking();
            }
        }
    }

    pub fn pop(&mut self) {
        self.buffer.pop();
        self.display_buffer.pop();
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
            let buf = &self.buffer[len-MAX_DUPLICATE_LENGTH..];
            let first = buf.chars().nth(0).unwrap();
            return buf.chars().all(|c| c == first);
        }
        return false;
    }
}

pub static INPUT_STATE: Lazy<Mutex<InputState>> = Lazy::new(|| Mutex::new(InputState::new()));

// Detect whenever the user is attempting to restore the English word or not by
// checking for any toned vowels in the input, and see if the output has any vowels.
// This should be a temporary fix until the engine (vi-rs) implemented a proper one.
pub fn user_attempted_to_restore_a_word(input: &str, output: &str) -> bool {
    let input_has_toned_vowels = input
        .chars()
        .filter(|c| TONABLE_VOWELS.contains(c) && c > &'z')
        .count()
        > 0;
    let output_has_no_toned_vowels = output
        .chars()
        .filter(|c| TONABLE_VOWELS.contains(c) && c > &'z')
        .count()
        == 0;
    return input_has_toned_vowels && output_has_no_toned_vowels;
}
