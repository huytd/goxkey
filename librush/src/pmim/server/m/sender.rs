use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use tokio::sync::mpsc::{Sender, error::SendError};

/// 根据连接状态发送消息
#[derive(Debug, Clone)]
pub struct MSender<T> {
    s: Sender<T>,
    /// 连接状态
    c: Arc<AtomicBool>,
}

impl<T> MSender<T> {
    pub fn new(s: Sender<T>) -> Self {
        Self {
            s,
            c: Arc::new(AtomicBool::new(false)),
        }
    }

    /// 设置连接状态
    pub fn 已连接(&self, c: bool) {
        self.c.store(c, Ordering::SeqCst);
    }

    /// 发送消息
    ///
    /// 如果连接断开, 直接丢弃消息
    pub async fn send(&self, m: T) -> Result<(), SendError<T>> {
        if self.c.load(Ordering::SeqCst) {
            self.s.send(m).await
        } else {
            // 丢弃
            Ok(())
        }
    }
}
