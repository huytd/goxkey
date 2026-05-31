//! 按键管理器
use pm_bin::log::debug;
use xkeysym::key;

use crate::ibus::IBusModifierState;

use super::super::m::{MSender, Ms, MsT};

#[derive(Debug, Clone, Copy, PartialEq)]
enum 输入状态 {
    /// 默认输入状态: 通过所有按键, 仅捕捉 `a` ~ `z` (进入拼音状态)
    默认,
    /// 拼音输入状态: 捕捉所有相关按键
    拼音,
}

/// 按键管理器
#[derive(Debug, Clone)]
pub struct Km {
    s: MSender<Ms>,

    状态: 输入状态,
    // 当前输入的拼音字符串
    t: String,

    // 启用输入反馈
    ef: bool,
    // 禁用按键捕获 (英文输入模式)
    禁用: bool,
    // 禁用 退格键
    禁用退格: bool,
}

// 按键 b'' as u32 定义
const KC1: u32 = b'`' as u32;
const KC2: u32 = b'~' as u32;
const KC3: u32 = b'!' as u32;
const KC4: u32 = b'@' as u32;
const KC5: u32 = b'#' as u32;
const KC6: u32 = b'$' as u32;
const KC7: u32 = b'%' as u32;
const KC8: u32 = b'^' as u32;
const KC9: u32 = b'&' as u32;
const KC10: u32 = b'*' as u32;
const KC11: u32 = b'(' as u32;
const KC12: u32 = b')' as u32;
const KC13: u32 = b'-' as u32;
const KC14: u32 = b'_' as u32;
const KC15: u32 = b'=' as u32;
const KC16: u32 = b'+' as u32;
const KC17: u32 = b'[' as u32;
const KC18: u32 = b'{' as u32;
const KC19: u32 = b']' as u32;
const KC20: u32 = b'}' as u32;
const KC21: u32 = b'\\' as u32;
const KC22: u32 = b'|' as u32;
const KC23: u32 = b';' as u32;
const KC24: u32 = b':' as u32;
const KC25: u32 = b'\'' as u32;
const KC26: u32 = b'"' as u32;
const KC27: u32 = b',' as u32;
const KC28: u32 = b'<' as u32;
const KC29: u32 = b'.' as u32;
const KC30: u32 = b'>' as u32;
const KC31: u32 = b'/' as u32;
const KC32: u32 = b'?' as u32;

impl Km {
    pub fn new(s: MSender<Ms>) -> Self {
        Self {
            s,
            状态: 输入状态::默认,
            t: "".to_string(),
            // 默认禁用输入反馈
            ef: false,
            禁用: false,
            禁用退格: false,
        }
    }

    /// 发送 M::T(PsMsgT(self.t))
    async fn send(&self) {
        // TODO 更好的错误处理
        // 忽略错误
        let _ = self.s.send(Ms::T(MsT(self.t.clone()))).await;
    }

    /// 重置输入状态
    async fn 清理(&mut self, 发送: bool) {
        self.状态 = 输入状态::默认;
        self.禁用 = false;
        self.禁用退格 = false;
        self.t = "".to_string();
        if 发送 {
            self.send().await;
        }
    }

    /// 输入反馈
    ///
    /// + f = 0: 禁用输入反馈 (默认)
    /// + f = 1: 启用输入反馈
    /// + f = 2: 重置 (已输入)
    /// + f = 3: 禁用按键捕获 (英文输入模式)
    /// + f = 4: 禁用 退格键
    /// + f = 5: 启用 退格键
    pub async fn 输入反馈(&mut self, f: i32) {
        match f {
            0 => {
                self.ef = false;
            }
            1 => {
                self.ef = true;
                self.禁用 = false;
            }
            2 => {
                self.清理(true).await;
            }
            3 => {
                self.清理(true).await;
                self.禁用 = true;
            }
            4 => {
                self.禁用退格 = true;
            }
            5 => {
                self.禁用退格 = false;
            }
            // 忽略其余取值
            _ => {}
        }
    }

    pub async fn process_key_event(&mut self, keyval: u32, _keycode: u32, state: u32) -> bool {
        let state = IBusModifierState::new_with_raw_value(state);
        // 禁用按键捕捉
        if self.禁用 {
            return false;
        }

        let mut 捕捉 = false;
        let 按下 = state.is_keydown();

        match self.状态 {
            输入状态::默认 => {
                // 只处理按键按下
                if 按下 {
                    match keyval {
                        // `a` ~ `z`
                        key::a..=key::z => {
                            // 如果特殊按键同时按下 (Shift, Ctrl, Alt, Super 等)
                            // 忽略按键
                            if !(state.has_special_modifiers() || state.shift()) {
                                捕捉 = true;
                                // 进入拼音状态
                                self.状态 = 输入状态::拼音;
                                // 更新拼音字符串
                                self.t = format!("{}", char::from_u32(keyval).unwrap());
                            }
                        }
                        // 忽略其余所有按键
                        _ => {}
                    }
                }
            }
            输入状态::拼音 => match keyval {
                // 捕捉所有相关按键
                // `a` ~ `z`
                key::a..=key::z => {
                    捕捉 = true;
                    if 按下 {
                        // 禁用退格的同时, 也禁止输入新的拼音
                        if !self.禁用退格 {
                            // 更新拼音字符串
                            self.t = format!("{}{}", self.t, char::from_u32(keyval).unwrap());
                        }
                    }
                }
                // ESC: 强制退出
                key::Escape => {
                    捕捉 = true;
                    if 按下 {
                        self.清理(false).await;
                    }
                }
                // `0` ~ `9`, 空格, Enter
                key::_0..=key::_9 | key::space | key::Return => {
                    捕捉 = true;
                    // 如果启用了输入反馈, 忽略按键
                    if 按下 && (!self.ef) {
                        // 退出拼音模式
                        self.清理(false).await;

                        debug!("退出拼音模式");
                    }
                }
                // 退格 (backspace)
                key::BackSpace => {
                    捕捉 = true;
                    // 如果禁用了退格键, 忽略按键
                    if 按下 && (!self.禁用退格) {
                        // 删除最后一个拼音字符
                        if self.t.len() > 1 {
                            let (a, _) = self.t.split_at(self.t.len() - 1);
                            self.t = a.to_string();
                        } else {
                            // 删除完毕, 退出拼音模式
                            self.清理(false).await;
                        }
                        debug!("退格");
                    }
                }
                // `A` ~ `Z`
                key::A..=key::Z => {
                    捕捉 = true;
                    // 忽略按键
                }
                // 所有 标准 104 键盘主键盘区 可输入的字符
                KC1 | KC2 | KC3 | KC4 | KC5 | KC6 | KC7 | KC8 | KC9 | KC10 | KC11 | KC12 | KC13
                | KC14 | KC15 | KC16 | KC17 | KC18 | KC19 | KC20 | KC21 | KC22 | KC23 | KC24
                | KC25 | KC26 | KC27 | KC28 | KC29 | KC30 | KC31 | KC32 => {
                    捕捉 = true;
                    // 忽略按键
                }
                // 删除键
                key::Delete => {
                    捕捉 = true;
                    // 忽略按键
                }
                // 光标按键: 上下左右
                key::Left | key::Right | key::Up | key::Down => {
                    捕捉 = true;
                    // 忽略按键
                    debug!("光标: 上下左右");
                }

                // 忽略其余所有按键
                _ => {}
            },
        }

        self.send().await;
        捕捉
    }

    pub async fn focus_in(&mut self) {}

    pub async fn focus_out(&mut self) {
        self.清理(true).await;
    }

    pub async fn reset(&mut self) {
        self.清理(true).await;
    }

    pub async fn enable(&mut self) {}

    pub async fn disable(&mut self) {
        self.清理(true).await;
    }
}
