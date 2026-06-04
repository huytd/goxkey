use std::error::Error;
use std::time::Duration;

use log::{debug, info};
use xkeysym::{KeyCode, Keysym};

use goxkey_core::{TypingMethod, INPUT_STATE};
use librush::ibus::{
    get_ibus_addr, IBus, IBusEngine, IBusEngineBackend, IBusFactory, IBusModifierState,
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
    last_committed_len: usize,
    method: TypingMethod,
}

impl GoxkeyEngine {
    /// Commit the current transformed word in-place by first erasing
    /// whatever we previously committed (via delete_surrounding_text),
    /// then committing the new transformation.
    /// This avoids the preedit underline since every keystroke instantly
    /// commits — there is no preedit text, only committed text.
    async unsafe fn commit_in_place(
        &mut self,
        se: &SignalEmitter<'_>,
    ) -> Result<(), ZbusError> {
        let input = &mut *INPUT_STATE;
        if let Ok((transformed, _)) = input.transform_keys() {
            if transformed != input.get_displaying_word() {
                input.replace(transformed);
            }
        }
        let display = input.get_displaying_word().to_string();

        if self.last_committed_len > 0 {
            GoxkeyEngine::delete_surrounding_text(
                se,
                -(self.last_committed_len as i32),
                self.last_committed_len as u32,
            )
            .await?;
        }

        if !display.is_empty() {
            self.last_committed_len = display.chars().count();
            GoxkeyEngine::commit_text(se, display).await?;
        } else {
            self.last_committed_len = 0;
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
        if state.is_keyup() {
            return Ok(false);
        }

        unsafe {
            let input = &mut *INPUT_STATE;
            input.set_method_im(self.method);

            if keyval == Keysym::BackSpace {
                if input.is_enabled() && !input.is_buffer_empty() {
                    input.pop();
                    if input.is_buffer_empty() {
                        if self.last_committed_len > 0 {
                            GoxkeyEngine::delete_surrounding_text(
                                &se,
                                -(self.last_committed_len as i32),
                                self.last_committed_len as u32,
                            )
                            .await?;
                            self.last_committed_len = 0;
                        }
                    } else {
                        self.commit_in_place(&se).await?;
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
                if input.is_enabled() {
                    if !input.is_buffer_empty() {
                        if input.should_restore_word() {
                            debug!("Restoring word");
                            let raw = input.get_typing_buffer().to_string();
                            if self.last_committed_len > 0 {
                                GoxkeyEngine::delete_surrounding_text(
                                    &se,
                                    -(self.last_committed_len as i32),
                                    self.last_committed_len as u32,
                                )
                                .await?;
                            }
                            if !raw.is_empty() {
                                GoxkeyEngine::commit_text(&se, raw).await?;
                            }
                        }
                        input.new_word();
                        self.last_committed_len = 0;
                    } else if !input.is_tracking() {
                        input.new_word();
                    }
                }
                return Ok(false);
            }

            if keyval.is_cursor_key() {
                if input.is_enabled() {
                    if !input.is_buffer_empty() {
                        let raw = input.get_typing_buffer().to_string();
                        if self.last_committed_len > 0 {
                            GoxkeyEngine::delete_surrounding_text(
                                &se,
                                -(self.last_committed_len as i32),
                                self.last_committed_len as u32,
                            )
                            .await?;
                        }
                        GoxkeyEngine::commit_text(&se, raw).await?;
                        input.new_word();
                        self.last_committed_len = 0;
                    } else if !input.is_tracking() {
                        input.new_word();
                    }
                }
                return Ok(false);
            }

            if !input.is_enabled() {
                return Ok(false);
            }

            if state.has_special_modifiers() {
                if !input.is_buffer_empty() {
                    input.new_word();
                }
                self.last_committed_len = 0;
                return Ok(false);
            }

            if let Some(c) = keysym_to_char(keyval) {
                if is_input_char(c) {
                    if input.is_tracking() {
                        debug!("Pushing: {:?}", c);
                        input.push(c);
                        self.commit_in_place(&se).await?;

                        if input.should_stop_tracking() {
                            input.stop_tracking();
                            self.last_committed_len = 0;
                        }
                        return Ok(true);
                    }
                    return Ok(false);
                }

                if input.is_enabled() {
                    if !input.is_buffer_empty() {
                        if input.should_restore_word() {
                            debug!("Restoring word");
                            let raw = input.get_typing_buffer().to_string();
                            if self.last_committed_len > 0 {
                                GoxkeyEngine::delete_surrounding_text(
                                    &se,
                                    -(self.last_committed_len as i32),
                                    self.last_committed_len as u32,
                                )
                                .await?;
                            }
                            if !raw.is_empty() {
                                GoxkeyEngine::commit_text(&se, raw).await?;
                            }
                        }
                        input.new_word();
                        self.last_committed_len = 0;
                    } else if !input.is_tracking() {
                        input.new_word();
                    }
                }
            }

            Ok(false)
        }
    }

    async fn focus_out(&mut self, _se: SignalEmitter<'_>, _server: &ObjectServer) -> fdo::Result<()> {
        debug!("Focus out");
        unsafe {
            INPUT_STATE.new_word();
        }
        self.last_committed_len = 0;
        Ok(())
    }

    async fn reset(&mut self, _se: SignalEmitter<'_>, _server: &ObjectServer) -> fdo::Result<()> {
        debug!("Reset");
        unsafe {
            INPUT_STATE.new_word();
        }
        self.last_committed_len = 0;
        Ok(())
    }

    async fn disable(&mut self, _se: SignalEmitter<'_>, _server: &ObjectServer) -> fdo::Result<()> {
        info!("Engine disabled");
        unsafe {
            INPUT_STATE.new_word();
        }
        self.last_committed_len = 0;
        Ok(())
    }
}

#[derive(Debug, Clone)]
struct GoxkeyFactory;

impl IBusFactory<GoxkeyEngine> for GoxkeyFactory {
    fn create_engine(&mut self, name: String) -> Result<GoxkeyEngine, String> {
        debug!("Creating engine: {:?}", name);
        match name.as_str() {
            "goxkey-telex" => Ok(GoxkeyEngine {
                last_committed_len: 0,
                method: TypingMethod::Telex,
            }),
            "goxkey-vni" => Ok(GoxkeyEngine {
                last_committed_len: 0,
                method: TypingMethod::VNI,
            }),
            _ => Err(format!("unknown engine: {}", name)),
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
