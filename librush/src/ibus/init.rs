//! ibus 相关的初始化
use std::error::Error;

use pm_bin::log::debug;
use zbus::{Connection, connection::Builder, names::WellKnownName};

use super::IBusErr;

pub async fn 连接ibus(addr: String) -> Result<Connection, Box<dyn Error>> {
    let c = Builder::address(addr.as_str())?.build().await?;

    // ibus 初始化: 获取 unique_name
    // 源文件: `ibus/src/ibusbus.c`
    // 函数: `ibus_bus_new()` -> `ibus_bus_connect()` -> `ibus_bus_hello()`
    // -> `g_dbus_connection_get_unique_name()`
    let n = c
        .unique_name()
        .ok_or(IBusErr::new("can not get dbus unique_name".to_string()))?;
    debug!("unique_name: {}", n);

    Ok(c)
}

/// ibus 初始化: 请求名称
///
/// 源文件: `ibus/src/ibusbus.c`
/// 函数: `ibus_bus_request_name()`
pub async fn 请求名称(c: &Connection, 名称: String) -> Result<(), Box<dyn Error>> {
    let n = WellKnownName::try_from(名称)?;

    c.request_name(n).await?;
    Ok(())
}
