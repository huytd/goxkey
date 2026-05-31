use std::env;
use std::error::Error;
use std::fs;
use std::path::PathBuf;

use super::IBusErr;

/// get the D-Bus (unix socket) address of ibus
pub fn get_ibus_addr() -> Result<String, Box<dyn Error>> {
    // this logic is copied from ibus_get_address in ibusshare.c
    if let Ok(address) = env::var("IBUS_ADDRESS") {
        return Ok(address);
    }
    // we must find the file containing the ibus address
    // this logic is copied from ibus_get_socket_path in ibus_share.c
    let ibus_address_file = match env::var("IBUS_ADDRESS_FILE") {
        Ok(path) => PathBuf::from(path),
        Err(_) => {
            // we must compute the path ourselves
            // On wayland:
            // ~/.config/ibus/bus/*-unix-wayland-0
            // On X:
            // ~/.config/ibus/bus/*-unix-0
            let config_home: PathBuf = match env::var("XDG_CONFIG_HOME") {
                Ok(home) => PathBuf::from(home),
                Err(_) => PathBuf::from(env::var("HOME")?).join(".config"),
            };
            let x_display;
            let wayland_display;
            let (hostname, display) = match env::var("WAYLAND_DISPLAY") {
                Ok(display) => {
                    wayland_display = display;
                    ("unix", wayland_display.as_str())
                }
                Err(_) => {
                    x_display = env::var("DISPLAY")?;
                    let mut parts = x_display.split(&[':', '.']);
                    let hostname = parts.next().ok_or("DISPLAY env var is empty")?;
                    let display = parts.next().ok_or("DISPLAY env var has no colon")?;
                    (
                        if hostname.is_empty() {
                            "unix"
                        } else {
                            hostname
                        },
                        display,
                    )
                }
            };
            // logic copied from ibus_get_local_machine_id
            let machine_id_file_content = std::fs::read_to_string("/var/lib/dbus/machine-id")
                .or(std::fs::read_to_string("/etc/machine-id"))
                .map_err(|e| IBusErr::new(format!("reading /etc/machine-id: {e}")))?;
            let machine_id = machine_id_file_content.trim();
            config_home
                .join("ibus/bus")
                .join(format!("{machine_id}-{hostname}-{display}"))
        }
    };

    // 读取配置文件, 获取里面的地址
    // 忽略注释 (`#`)
    let 地址 = fs::read_to_string(&ibus_address_file)?
        .lines()
        .filter(|i| !i.starts_with("#"))
        .find_map(|i| i.strip_prefix("IBUS_ADDRESS=").map(|i| i.to_string()))
        .ok_or(IBusErr::new(format!(
            "can not find ibus addr in: {}",
            ibus_address_file.display()
        )))?;

    Ok(地址.to_string())
}
