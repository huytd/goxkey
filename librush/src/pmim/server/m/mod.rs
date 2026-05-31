//! 内部互发消息 (根据接收者分类)
//!
//! + `Mk`: `Km` 按键管理器 <- IBusEngine
//! + `Ms`: pmim-server <- ibrus
//! + `Mr`: ibrus (SignalContext) <- pmim-server
//!
//! (`Ms`) 发送消息: ibrus -> pmim-server (unix socket)
//! + `S`: IBusEngine 状态转换消息
//! + `K`: 按键消息
//! + `C`: 光标位置消息
//! + `T`: 按键管理器 设置输入字符串
//!
//! (`Mr`) 接收消息: ibrus <- pmim-server (unix socket)
//! + `f`: 输入反馈
//! + `t`: 提交文本 (CommitText)

mod mk;
mod mr;
mod ms;
mod sender;

pub use mk::Mk;
pub use mr::Mr;
pub use ms::{Ms, MsC, MsK, MsS, MsT};
pub use sender::MSender;
