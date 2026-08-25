//! `AtS`: 给 pmim-server 发送消息的任务
use pm_bin::log::{debug, error, info};
use std::env;
use std::error::Error;
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader, BufWriter},
    net::{UnixStream, unix::OwnedReadHalf},
    sync::mpsc,
    time::{Duration, sleep},
};

use super::super::m::{MSender, Mr, Ms, MsS};

/// 获取 pmim-server unix socket 文件路径
/// ${XDG_RUNTIME_DIR}/pmim/us
pub fn pmim_us() -> Result<String, Box<dyn Error>> {
    let xrd = env::var("XDG_RUNTIME_DIR")?;
    Ok(format!("{}/pmim/us", xrd))
}

async fn 接收消息(r: OwnedReadHalf, s: mpsc::Sender<Mr>) -> Result<(), Box<dyn Error>> {
    let mut r = BufReader::new(r).lines();
    // 读行
    while let Some(l) = r.next_line().await? {
        match Mr::from(l.clone()) {
            Some(m) => {
                // 忽略错误
                let _ = s.send(m).await;
            }
            None => {
                // 解析失败
                error!("消息解析失败: {}", l);
            }
        }
    }
    Ok(())
}

async fn 连接服务单次(
    ps: String,
    s: MSender<Ms>,
    r: &mut mpsc::Receiver<Ms>,
    sr: mpsc::Sender<Mr>,
) -> Result<(), Box<dyn Error>> {
    debug!("连接 {}", ps);

    let (rx, tx) = UnixStream::connect(ps).await?.into_split();
    // 连接成功
    s.已连接(true);
    // 启动接收任务
    tokio::spawn(async move {
        // 忽略错误
        let _ = 接收消息(rx, sr).await;
    });
    // 发送连接成功消息
    let s1 = s.clone();
    tokio::spawn(async move {
        let _ = s1.send(Ms::S(MsS("ok".to_string()))).await;
    });

    let mut w = BufWriter::new(tx);
    // 不停的发送消息
    loop {
        match r.recv().await {
            Some(m) => {
                // 消息字节数据
                let b = m.to_string();
                w.write_all(b.as_bytes()).await?;
                // 写入换行
                w.write_u8(b'\n').await?;
                // 写入完毕
                w.flush().await?;
            }
            None => {
                return Ok(());
            }
        }
    }
}

async fn 任务(
    ps: String,
    s: MSender<Ms>,
    mut r: mpsc::Receiver<Ms>,
    sr: mpsc::Sender<Mr>,
    flatpak: bool,
) {
    loop {
        let _ = 连接服务单次(ps.clone(), s.clone(), &mut r, sr.clone()).await;
        // 连接断开
        s.已连接(false);

        // 重新连接之前等待的时间 (秒)
        let mut w = 2;
        if flatpak {
            w = 1;
        }
        debug!("连接断开, {}s 后重试 .. .", w);
        sleep(Duration::from_secs(w)).await;
    }
}

/// 启动 `AtS` 任务
pub fn at_s(sr: mpsc::Sender<Mr>, flatpak: bool) -> Result<MSender<Ms>, Box<dyn Error>> {
    let ps = pmim_us()?;
    info!("{}", ps);

    // 发送消息的通道
    let (tx, rx) = mpsc::channel(256);
    let s = MSender::<Ms>::new(tx);

    let s1 = s.clone();
    tokio::spawn(async move {
        任务(ps, s1, rx, sr, flatpak).await;
    });

    Ok(s)
}
