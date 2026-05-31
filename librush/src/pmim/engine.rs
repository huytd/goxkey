use xkeysym::{KeyCode, Keysym};
use zbus::{ObjectServer, fdo, object_server::SignalEmitter};

use super::server::Pmims;
use crate::ibus::{IBusEngine, IBusFactory, IBusModifierState};

#[derive(Debug, Clone)]
pub struct PmimEngine {
    s: Pmims,
}

impl PmimEngine {
    pub fn new(s: Pmims) -> Self {
        Self { s }
    }
}

impl IBusEngine for PmimEngine {
    async fn process_key_event(
        &mut self,
        se: SignalEmitter<'_>,
        server: &ObjectServer,
        keyval: Keysym,
        keycode: KeyCode,
        state: IBusModifierState,
    ) -> fdo::Result<bool> {
        self.s
            .process_key_event(se, server, keyval.into(), keycode.into(), state.raw_value())
            .await
    }

    async fn set_cursor_location(
        &mut self,
        se: SignalEmitter<'_>,
        server: &ObjectServer,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
    ) -> fdo::Result<()> {
        self.s.set_cursor_location(se, server, x, y, w, h).await
    }

    async fn focus_in(&mut self, se: SignalEmitter<'_>, server: &ObjectServer) -> fdo::Result<()> {
        self.s.focus_in(se, server).await
    }

    async fn focus_out(&mut self, se: SignalEmitter<'_>, server: &ObjectServer) -> fdo::Result<()> {
        self.s.focus_out(se, server).await
    }

    async fn reset(&mut self, se: SignalEmitter<'_>, server: &ObjectServer) -> fdo::Result<()> {
        self.s.reset(se, server).await
    }

    async fn enable(&mut self, se: SignalEmitter<'_>, server: &ObjectServer) -> fdo::Result<()> {
        self.s.enable(se, server).await
    }

    async fn disable(&mut self, se: SignalEmitter<'_>, server: &ObjectServer) -> fdo::Result<()> {
        self.s.disable(se, server).await
    }
}

#[derive(Debug, Clone)]
pub struct PmimFactory {
    s: Pmims,
}

impl PmimFactory {
    pub fn new(s: Pmims) -> Self {
        Self { s }
    }
}

impl IBusFactory<PmimEngine> for PmimFactory {
    fn create_engine(&mut self, name: String) -> Result<PmimEngine, String> {
        let 名称 = "pmim";

        if 名称 == name {
            Ok(PmimEngine::new(self.s.clone()))
        } else {
            Err(format!("unknown name: {}", name))
        }
    }
}
