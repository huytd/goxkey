use std::error::Error;
use std::time::Duration;

use log::{debug, info};
use xkeysym::{KeyCode, Keysym};

use goxkey_core::INPUT_STATE;
use librush::ibus::{
    get_ibus_addr, IBus, IBusEngine, IBusEngineBackend, IBusFactory, IBusModifierState,
    IBusPreeditFocusMode,
};
use zbus::{fdo, object_server::SignalEmitter, ObjectServer, Error as ZbusError};

fn keysym_to_char(keysym: Keysym) -> Option<char> {
    let val: u32 = keysym.into();
    if val <= 0x10FFFF {
        char::from_u32(val)
    } else if (0x01000100..=0x0110FFFF).contains(&val) {
        char::from_u32(val & 0x00FFFFFF)
    } else {
        None
    }
}

fn is_input_char(c: char) -> bool {
    c.is_alphabetic() || c.is_ascii_digit()
}

#[derive(Debug, Clone)]
struct GoxkeyEngine {
    last_preedit: String,
}

impl GoxkeyEngine {
    async unsafe fn update_and_show_preedit(
        &mut self,
        se: &SignalEmitter<'_>,
    ) -> Result<(), ZbusError> {
        let input = &mut *INPUT_STATE;
        if let Ok((transformed, _)) = input.transform_keys() {
            if transformed != input.get_displaying_word() {
                input.replace(transformed);
            }
        }
        let display = input.get_displaying_word();
        if display == self.last_preedit {
            return Ok(());
        }
        self.last_preedit = display.to_string();
        let cursor_pos = display.chars().count() as u32;
        GoxkeyEngine::update_preedit_text(
            se,
            display.to_string(),
            cursor_pos,
            true,
            IBusPreeditFocusMode::Clear,
        )
        .await
    }

    async unsafe fn hide_preedit(
        &self,
        se: &SignalEmitter<'_>,
    ) -> Result<(), ZbusError> {
        GoxkeyEngine::update_preedit_text(
            se,
            String::new(),
            0,
            false,
            IBusPreeditFocusMode::Clear,
        )
        .await
    }

    async unsafe fn commit_and_clear(
        &mut self,
        se: &SignalEmitter<'_>,
    ) -> Result<(), ZbusError> {
        let input = &mut *INPUT_STATE;
        let to_commit = if input.should_restore_word() {
            debug!("Restoring word");
            input.get_typing_buffer().to_string()
        } else {
            input.get_displaying_word().to_string()
        };
        if !to_commit.is_empty() {
            self.hide_preedit(se).await?;
            debug!("Committing: {:?}", to_commit);
            GoxkeyEngine::commit_text(se, to_commit).await?;
        }
        input.new_word();
        self.last_preedit.clear();
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
        if state.is_keyup() {
            return Ok(false);
        }

        unsafe {
            let input = &mut *INPUT_STATE;

            if keyval == Keysym::BackSpace {
                if input.is_enabled() && !input.is_buffer_empty() {
                    input.pop();
                    if input.is_buffer_empty() {
                        self.last_preedit.clear();
                        self.hide_preedit(&se).await?;
                    } else {
                        self.update_and_show_preedit(&se).await?;
                    }
                    debug!("Backspace -> buffer: {:?}", input.get_typing_buffer());
                    return Ok(true);
                }
                return Ok(false);
            }

            if keyval == Keysym::space
                || keyval == Keysym::Return
                || keyval == Keysym::Tab
                || keyval == Keysym::Escape
            {
                if input.is_enabled() && !input.is_buffer_empty() {
                    self.commit_and_clear(&se).await?;
                }
                input.new_word();
                return Ok(false);
            }

            if keyval.is_cursor_key() {
                if input.is_enabled() && !input.is_buffer_empty() {
                    let raw = input.get_typing_buffer().to_string();
                    self.hide_preedit(&se).await?;
                    GoxkeyEngine::commit_text(&se, raw).await?;
                    input.new_word();
                    self.last_preedit.clear();
                }
                return Ok(false);
            }

            if !input.is_enabled() {
                return Ok(false);
            }

            if state.has_special_modifiers() {
                return Ok(false);
            }

            if let Some(c) = keysym_to_char(keyval) {
                if is_input_char(c) {
                    if input.is_tracking() {
                        debug!("Pushing: {:?}", c);
                        input.push(c);

                        if input.should_stop_tracking() {
                            if let Ok((transformed, _)) = input.transform_keys() {
                                if !transformed.is_empty() {
                                    debug!("Committing (stop tracking): {:?}", transformed);
                                    GoxkeyEngine::commit_text(&se, transformed).await?;
                                }
                            }
                            input.stop_tracking();
                        } else {
                            self.update_and_show_preedit(&se).await?;
                        }
                        return Ok(true);
                    }
                    return Ok(false);
                }

                if input.is_enabled() && !input.is_buffer_empty() {
                    self.commit_and_clear(&se).await?;
                }
            }

            Ok(false)
        }
    }

    async fn focus_out(&mut self, se: SignalEmitter<'_>, _server: &ObjectServer) -> fdo::Result<()> {
        debug!("Focus out");
        unsafe {
            self.hide_preedit(&se).await.unwrap_or_default();
            INPUT_STATE.new_word();
        }
        Ok(())
    }

    async fn reset(&mut self, se: SignalEmitter<'_>, _server: &ObjectServer) -> fdo::Result<()> {
        debug!("Reset");
        unsafe {
            self.hide_preedit(&se).await.unwrap_or_default();
            INPUT_STATE.new_word();
        }
        Ok(())
    }

    async fn disable(&mut self, se: SignalEmitter<'_>, _server: &ObjectServer) -> fdo::Result<()> {
        info!("Engine disabled");
        unsafe {
            self.hide_preedit(&se).await.unwrap_or_default();
            INPUT_STATE.new_word();
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
struct GoxkeyFactory;

impl IBusFactory<GoxkeyEngine> for GoxkeyFactory {
    fn create_engine(&mut self, name: String) -> Result<GoxkeyEngine, String> {
        debug!("Creating engine: {:?}", name);
        if name == "goxkey" {
            Ok(GoxkeyEngine {
                last_preedit: String::new(),
            })
        } else {
            Err(format!("unknown engine: {}", name))
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();
    info!("Starting goxkey-ibus...");

    let addr = get_ibus_addr()?;
    debug!("IBus address: {:?}", addr);

    let factory = GoxkeyFactory;
    let ibus = IBus::<GoxkeyEngine, GoxkeyFactory>::new(
        addr,
        factory,
        "org.freedesktop.IBus.Goxkey".to_string(),
    )
    .await?;
    let _conn = ibus.conn();

    info!("goxkey-ibus engine registered and running.");

    loop {
        tokio::time::sleep(Duration::from_secs(u64::MAX)).await;
    }
}
