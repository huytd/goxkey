//! `AtR`: 从 pmim-server 接收消息 (中转) 的任务
use tokio::sync::mpsc;
use zbus::object_server::SignalEmitter;

use super::super::super::engine::PmimEngine;
use super::super::m::{Mk, Mr};
use crate::ibus::IBusEngineBackend;

async fn 任务(mut r: mpsc::Receiver<Mr>) {
    // 按键管理器 消息发送端
    let mut k: Option<mpsc::Sender<Mk>> = None;
    // SignalEmitter
    let mut se: Option<SignalEmitter<'static>> = None;

    // 不停的接收消息
    loop {
        match r.recv().await {
            Some(m) => match m {
                // 提交文本 (CommitText)
                Mr::T(t) => {
                    if let Some(se) = &se {
                        // 忽略错误
                        let _ = PmimEngine::commit_text(se, t.0).await;
                    }
                    // 忽略
                }
                // 输入反馈
                Mr::F(f) => {
                    if let Some(k) = &k {
                        // 忽略错误
                        let _ = k.send(Mk::F(f.0)).await;
                    }
                    // 忽略
                }
                // 更新 SignalContext
                Mr::SE(x) => {
                    se = Some(x);
                }
                // 更新 按键管理器 消息发送端
                Mr::K(x) => {
                    k = Some(x);
                }
            },
            None => {
                break;
            }
        }
    }
}

/// 启动 `AtR` 任务
pub fn at_r() -> mpsc::Sender<Mr> {
    // 发送消息的通道
    let (tx, rx) = mpsc::channel(256);

    tokio::spawn(async move {
        任务(rx).await;
    });

    tx
}
