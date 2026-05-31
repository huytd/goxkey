//! 对 ibus 整体的抽象
use std::error::Error;
use std::marker::PhantomData;

use pm_bin::log::debug;
use zbus::Connection;

use super::{IBusEngine, IBusFactory};

use super::factory::{Factory, 注册factory};
use super::init::{请求名称, 连接ibus};

/// Abstract for the whole ibus
#[derive(Debug, Clone)]
pub struct IBus<T: IBusEngine, U: IBusFactory<T>> {
    _t: PhantomData<T>,
    _u: PhantomData<U>,

    c: Connection,
}

impl<T: IBusEngine + 'static, U: IBusFactory<T> + 'static> IBus<T, U> {
    /// connect to ibus and init
    ///
    /// only support 1 engine now.
    pub async fn new(addr: String, factory: U, name: String) -> Result<Self, Box<dyn Error>> {
        let c = 连接ibus(addr).await?;
        debug!("连接到 ibus 成功");

        {
            let f = Factory::new(c.clone(), factory);
            注册factory(&c, f).await?;
        }
        debug!("注册 factory 成功");

        请求名称(&c, name.clone()).await?;
        debug!("请求名称: {}", name);

        Ok(Self {
            _t: PhantomData,
            _u: PhantomData,
            c,
        })
    }

    /// get D-Bus connection
    pub fn conn(&self) -> Connection {
        self.c.clone()
    }
}
