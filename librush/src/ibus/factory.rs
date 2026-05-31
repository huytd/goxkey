//! factory: 用于创建 engine
use std::error::Error;
use std::marker::PhantomData;

use pm_bin::log::debug;
use zbus::{Connection, fdo, interface, zvariant::ObjectPath};

use super::IBusEngine;
use super::engine::Engine;

/// Implement this trait to create engine
pub trait IBusFactory<T: IBusEngine>: Send + Sync {
    /// create the engine (input method)
    fn create_engine(&mut self, name: String) -> Result<T, String>;
}

#[derive(Debug, Clone)]
pub struct Factory<T: IBusEngine, U: IBusFactory<T>> {
    _t: PhantomData<T>,

    c: Connection,

    f: U,
}

impl<T: IBusEngine, U: IBusFactory<T>> Factory<T, U> {
    pub fn new(c: Connection, f: U) -> Self {
        Self {
            _t: PhantomData,
            c,
            f,
        }
    }
}

// <node>
//   <interface name='org.freedesktop.IBus.Factory'>
//     <method name='CreateEngine'>
//       <arg direction='in'  type='s' name='name' />
//       <arg direction='out' type='o' />
//     </method>
//   </interface>
// </node>
#[interface(name = "org.freedesktop.IBus.Factory")]
impl<T: IBusEngine + 'static, U: IBusFactory<T> + 'static> Factory<T, U> {
    #[zbus(name = "CreateEngine")]
    async fn create_engine(&mut self, name: String) -> fdo::Result<ObjectPath<'_>> {
        debug!("CreateEngine");

        let e = self
            .f
            .create_engine(name.clone())
            .map_err(|s| fdo::Error::Failed(s))?;

        let p = Engine::new(&self.c, e)
            .await
            .map_err(|e| fdo::Error::Failed(format!("{:?}", e)))?;
        let o = ObjectPath::try_from(p).map_err(|e| fdo::Error::Failed(format!("{:?}", e)))?;
        Ok(o)
    }
}

// `ibus/src/ibusshare.h`
const IBUS_PATH_FACTORY: &'static str = "/org/freedesktop/IBus/Factory";

/// ibus 初始化: 注册 engine factory
///
/// 源文件: `ibus/src/ibusfactory.c`
/// 函数: `ibus_factory_new()` -> `ibus_factory_class_init()`
/// -> `ibus_service_class_add_interfaces()` -> `g_dbus_node_info_new_for_xml()`
pub async fn 注册factory<T: IBusEngine + 'static, U: IBusFactory<T> + 'static>(
    c: &Connection,
    f: Factory<T, U>,
) -> Result<(), Box<dyn Error>> {
    c.object_server().at(IBUS_PATH_FACTORY, f).await?;
    Ok(())
}
