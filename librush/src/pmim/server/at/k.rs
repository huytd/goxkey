//! `AtK`: Km 按键管理器 运行的任务
use tokio::sync::mpsc;

use super::super::m::{MSender, Mk, Ms};
use super::km::Km;

async fn 任务(mut r: mpsc::Receiver<Mk>, s: MSender<Ms>) {
    let mut km = Km::new(s);

    loop {
        match r.recv().await {
            Some(m) => match m {
                Mk::ProcessKeyEvent((keyval, keycode, state, ret)) => {
                    let 结果 = km.process_key_event(keyval, keycode, state).await;
                    // 忽略错误
                    let _ = ret.send(结果);
                }
                Mk::FocusIn => {
                    km.focus_in().await;
                }
                Mk::FocusOut => {
                    km.focus_out().await;
                }
                Mk::Reset => {
                    km.reset().await;
                }
                Mk::Enable => {
                    km.enable().await;
                }
                Mk::Disable => {
                    km.disable().await;
                }
                Mk::F(f) => {
                    km.输入反馈(f).await;
                }
            },
            None => {
                break;
            }
        }
    }
}

/// 启动 `AtK` 任务
pub fn at_k(s: MSender<Ms>) -> mpsc::Sender<Mk> {
    let (tx, rx) = mpsc::channel::<Mk>(16);

    tokio::spawn(async move {
        任务(rx, s).await;
    });

    tx
}
