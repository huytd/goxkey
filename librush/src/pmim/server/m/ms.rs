use serde_json::Value;

use crate::ibus::IBusModifierState;

const MS_S: &'static str = "S";
const MS_K: &'static str = "K";
const MS_C: &'static str = "C";
const MS_T: &'static str = "T";

/// 消息: pmim-server <- ibrus
#[derive(Debug, Clone, PartialEq)]
pub enum Ms {
    /// `S`: IBusEngine 状态转换消息
    S(MsS),
    /// `K`: 按键消息
    K(MsK),
    /// `C`: 光标位置消息
    C(MsC),
    /// `T`: 按键管理器 设置输入字符串
    T(MsT),
}

impl ToString for Ms {
    fn to_string(&self) -> String {
        match self {
            Ms::S(m) => m.to_string(),
            Ms::K(m) => m.to_string(),
            Ms::C(m) => m.to_string(),
            Ms::T(m) => m.to_string(),
        }
    }
}

trait MsToString {
    fn value(&self) -> Value;

    fn name(&self) -> &'static str;

    /// 消息序列化
    fn to_string(&self) -> String {
        format!("{} {}", self.name().to_string(), self.value().to_string())
    }
}

/// 消息 `S`: IBusEngine 状态转换消息
#[derive(Debug, Clone, PartialEq)]
pub struct MsS(pub String);

impl MsToString for MsS {
    fn name(&self) -> &'static str {
        MS_S
    }

    fn value(&self) -> Value {
        Value::from(self.0.as_str())
    }
}

/// 消息 `K`: 按键消息
///
/// `process_key_event(keyval, keycode, state)` + is_keydown
#[derive(Debug, Clone, PartialEq)]
pub struct MsK(pub Vec<u32>);

impl MsK {
    pub fn new(keyval: u32, keycode: u32, state: u32) -> Self {
        let kd = if IBusModifierState::new_with_raw_value(state).is_keydown() {
            1
        } else {
            0
        };
        Self(vec![keyval, keycode, state, kd])
    }
}

impl MsToString for MsK {
    fn name(&self) -> &'static str {
        MS_K
    }

    fn value(&self) -> Value {
        Value::from(self.0.clone())
    }
}

/// 消息 `C`: 光标位置消息
///
/// `set_cursor_location(x, y, w, h)`
#[derive(Debug, Clone, PartialEq)]
pub struct MsC(pub Vec<i32>);

impl MsC {
    pub fn new(x: i32, y: i32, w: i32, h: i32) -> Self {
        Self(vec![x, y, w, h])
    }
}

impl MsToString for MsC {
    fn name(&self) -> &'static str {
        MS_C
    }

    fn value(&self) -> Value {
        Value::from(self.0.clone())
    }
}

/// 消息 `T`: 按键管理器 设置输入字符串
#[derive(Debug, Clone, PartialEq)]
pub struct MsT(pub String);

impl MsToString for MsT {
    fn name(&self) -> &'static str {
        MS_T
    }

    fn value(&self) -> Value {
        Value::from(self.0.as_str())
    }
}
