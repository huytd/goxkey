mod composer;
mod keys;
mod respell;
mod support;

use std::error::Error;

use log::{debug, info};
use xkeysym::{KeyCode, Keysym};

use goxkey_core::TypingMethod;
use librush::ibus::{
    extract_text_from_ibus_value, get_ibus_addr, IBus, IBusEngine, IBusEngineBackend, IBusFactory,
    IBusModifierState, IBusPreeditFocusMode,
};
use zbus::{fdo, object_server::SignalEmitter, zvariant::Value, Error as ZbusError, ObjectServer};

use composer::{Action, Composer};
use support::ClientCache;

/// D-Bus side of one IBus engine: forwards calls to the [`Composer`] and
/// emits the actions it returns.
struct GoxkeyEngine {
    composer: Composer,
}

impl GoxkeyEngine {
    async fn emit(se: &SignalEmitter<'_>, actions: Vec<Action>) -> Result<(), ZbusError> {
        for action in actions {
            match action {
                Action::Delete(n) => {
                    GoxkeyEngine::delete_surrounding_text(se, -(n as i32), n as u32).await?
                }
                Action::Commit(text) => GoxkeyEngine::commit_text(se, text).await?,
                Action::Preedit(text) => {
                    let len = text.chars().count() as u32;
                    // Commit mode: the client keeps the preedit text if focus
                    // moves or the context is reset mid-word.
                    GoxkeyEngine::update_preedit_text(
                        se,
                        text,
                        len,
                        len > 0,
                        IBusPreeditFocusMode::Commit,
                    )
                    .await?
                }
            }
        }
        Ok(())
    }
}

impl IBusEngine for GoxkeyEngine {
    async fn process_key_event(
        &mut self,
        se: SignalEmitter<'_>,
        _server: &ObjectServer,
        keyval: Keysym,
        _keycode: KeyCode,
        state: IBusModifierState,
    ) -> fdo::Result<bool> {
        let outcome = self.composer.process_key(keyval, state);
        GoxkeyEngine::emit(&se, outcome.actions).await?;
        Ok(outcome.handled)
    }

    fn set_capabilities(&mut self, caps: u32) {
        self.composer.set_capabilities(caps);
    }

    fn set_content_type(&mut self, purpose: u32, hints: u32) {
        self.composer.set_content_type(purpose, hints);
    }

    async fn set_surrounding_text(
        &mut self,
        _se: SignalEmitter<'_>,
        _server: &ObjectServer,
        text: Value<'_>,
        cursor_pos: u32,
        anchor_pos: u32,
    ) -> fdo::Result<()> {
        if let Some(text) = extract_text_from_ibus_value(&text) {
            self.composer
                .set_surrounding_text(&text, cursor_pos, anchor_pos);
        }
        Ok(())
    }

    fn has_focus_id(&self) -> bool {
        true
    }

    async fn focus_in(
        &mut self,
        _se: SignalEmitter<'_>,
        _server: &ObjectServer,
    ) -> fdo::Result<()> {
        debug!("Focus in");
        self.composer.focus_in(None);
        Ok(())
    }

    async fn focus_in_id(
        &mut self,
        _se: SignalEmitter<'_>,
        _server: &ObjectServer,
        _object_path: String,
        client: String,
    ) -> fdo::Result<()> {
        debug!("Focus in: {:?}", client);
        self.composer.focus_in(Some(&client));
        Ok(())
    }

    async fn focus_out(
        &mut self,
        _se: SignalEmitter<'_>,
        _server: &ObjectServer,
    ) -> fdo::Result<()> {
        debug!("Focus out");
        self.composer.discard_word();
        Ok(())
    }

    async fn reset(&mut self, _se: SignalEmitter<'_>, _server: &ObjectServer) -> fdo::Result<()> {
        debug!("Reset");
        self.composer.discard_word();
        Ok(())
    }

    async fn enable(&mut self, _se: SignalEmitter<'_>, _server: &ObjectServer) -> fdo::Result<()> {
        debug!("Enable");
        self.composer.discard_word();
        Ok(())
    }

    async fn disable(&mut self, _se: SignalEmitter<'_>, _server: &ObjectServer) -> fdo::Result<()> {
        info!("Engine disabled");
        self.composer.discard_word();
        Ok(())
    }
}

#[derive(Clone)]
struct GoxkeyFactory {
    cache: ClientCache,
}

impl IBusFactory<GoxkeyEngine> for GoxkeyFactory {
    fn create_engine(&mut self, name: String) -> Result<GoxkeyEngine, String> {
        debug!("Creating engine: {:?}", name);
        let method = match name.as_str() {
            "goxkey-telex" => TypingMethod::Telex,
            "goxkey-vni" => TypingMethod::VNI,
            _ => return Err(format!("unknown engine: {}", name)),
        };
        Ok(GoxkeyEngine {
            composer: Composer::new(method, self.cache.clone()),
        })
    }
}

// Engines are only ever touched from the D-Bus dispatch loop, and calls are
// handled in order (see `spawn = false` in librush), so one thread is enough.
#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();
    info!("Starting goxkey-ibus...");

    let addr = get_ibus_addr()?;
    debug!("IBus address: {:?}", addr);

    let factory = GoxkeyFactory {
        cache: ClientCache::load(),
    };
    let ibus = IBus::<GoxkeyEngine, GoxkeyFactory>::new(
        addr,
        factory,
        "org.freedesktop.IBus.Goxkey".to_string(),
    )
    .await?;
    let _conn = ibus.conn();

    info!("goxkey-ibus engine registered and running.");

    std::future::pending::<()>().await;
    Ok(())
}
