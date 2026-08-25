use tokio::sync::oneshot::Sender;

/// 消息: `Km` 按键管理器 <- IBusEngine
#[derive(Debug)]
pub enum Mk {
    /// `process_key_event(keyval, keycode, state) -> bool`
    ProcessKeyEvent((u32, u32, u32, Sender<bool>)),
    /// `focus_in()`
    FocusIn,
    /// `focus_out()`
    FocusOut,
    /// `reset()`
    Reset,
    /// `enable()`
    Enable,
    /// `disable()`
    Disable,
    /// 输入反馈 `f()`
    F(i32),
}
