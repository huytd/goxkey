//! ibus module for pmim
//!
//! <https://github.com/fm-elpac/pmim>
use std::error::Error;
use std::process::Command;

use pm_bin::log::{debug, info};
use tokio::{
    runtime::Runtime,
    time::{Duration, sleep},
};

use crate::ibus::IBus;
use crate::ibus::get_ibus_addr;

pub mod engine;
mod server;

use engine::PmimFactory;

pub fn main(flatpak: bool) -> Result<(), Box<dyn Error>> {
    debug!("init");

    let 地址 = get_ibus_addr()?;
    info!("ibus addr: {}", 地址);

    if flatpak {
        // 运行命令: `flatpak run io.github.fm_elpac.pmim_ibus`
        info!("run: flatpak run io.github.fm_elpac.pmim_ibus");

        Command::new("flatpak")
            .args(["run", "io.github.fm_elpac.pmim_ibus"])
            .spawn()?;
    }

    let rt = Runtime::new()?;
    rt.block_on(async {
        let s = server::初始化pmims(flatpak).await?;

        let 名称 = "org.fm_elpac.pmim";
        let _b = IBus::new(地址, PmimFactory::new(s), 名称.to_string()).await?;

        info!("初始化完毕");

        loop {
            sleep(Duration::from_secs(10)).await;
        }
        //Ok(())
    })
}
