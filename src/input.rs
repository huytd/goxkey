use druid::Data;
use once_cell::sync::Lazy;
use std::sync::Mutex;

// According to Google search, the longest possible Vietnamese word
// is "nghiêng", which is 7 letters long. Add a little buffer for
// tone and marks, I guess the longest possible buffer length would
// be around 10 to 12.
const MAX_POSSIBLE_WORD_LENGTH: usize = 10;

#[derive(PartialEq, Eq, Data, Clone, Copy)]
pub enum TypingMethod {
    VNI,
    Telex,
}

pub struct InputState {
    pub buffer: String,
    pub method: TypingMethod,
    pub enabled: bool,
}

impl InputState {
    pub fn new() -> Self {
        Self {
            buffer: String::new(),
            method: TypingMethod::Telex,
            enabled: true,
        }
    }

    pub fn toggle_vietnamese(&mut self) {
        self.enabled = !self.enabled;
        self.clear();
    }

    pub fn set_method(&mut self, method: TypingMethod) {
        self.method = method;
        self.clear();
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

    pub fn replace(&mut self, buf: String) {
        self.buffer = buf;
    }

    pub fn push(&mut self, c: char) {
        if self.buffer.len() <= MAX_POSSIBLE_WORD_LENGTH {
            self.buffer.push(c);
        }
    }

    pub fn pop(&mut self) {
        self.buffer.pop();
    }

    pub fn clear(&mut self) {
        self.buffer.clear();
    }
}

pub static INPUT_STATE: Lazy<Mutex<InputState>> = Lazy::new(|| Mutex::new(InputState::new()));
