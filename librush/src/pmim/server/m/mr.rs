use tokio::sync::mpsc;
use zbus::object_server::SignalEmitter;

use super::Mk;

/// 消息: ibrus (SignalEmitter) <- pmim-server
#[derive(Debug, Clone)]
pub enum Mr {
    /// `t`: 提交文本 (CommitText)
    T(MrT),
    /// `f`: 输入反馈
    F(MrF),
    /// SignalEmitter
    SE(SignalEmitter<'static>),
    /// 按键管理器 消息发送端
    K(mpsc::Sender<Mk>),
}

const MR_T: &'static str = "t";
const MR_F: &'static str = "f";

impl Mr {
    /// 从字符串解析消息
    ///
    /// 解析失败返回 None
    pub fn from(s: String) -> Option<Mr> {
        // 寻找空格
        match s.find(' ') {
            Some(i) => {
                let (n, v) = s.split_at(i);
                match n {
                    MR_T => MrT::from(v).map(|m| Mr::T(m)),
                    MR_F => MrF::from(v).map(|m| Mr::F(m)),
                    // 未知消息
                    _ => None,
                }
            }
            // 解析失败
            None => None,
        }
    }
}

/// 消息 `t`: 提交文本 (CommitText)
#[derive(Debug, Clone, PartialEq)]
pub struct MrT(pub String);

impl MrT {
    /// 从字符串获取数据, 解析失败返回 None
    pub fn from(s: &str) -> Option<Self> {
        match serde_json::from_str::<String>(s) {
            Ok(t) => Some(Self(t)),
            _ => None,
        }
    }
}

/// 消息 `f`: 输入反馈
#[derive(Debug, Clone, PartialEq)]
pub struct MrF(pub i32);

impl MrF {
    /// 从字符串获取数据, 解析失败返回 None
    pub fn from(s: &str) -> Option<Self> {
        match serde_json::from_str::<i32>(s) {
            Ok(i) => Some(Self(i)),
            _ => None,
        }
    }
}
