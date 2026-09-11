use std::error::Error;
use std::time::Duration;

use log::{debug, info};
use xkeysym::{KeyCode, Keysym};

use goxkey_core::{get_diff_parts, TypingMethod, INPUT_STATE};
use librush::ibus::{
    get_ibus_addr, IBus, IBusEngine, IBusEngineBackend, IBusFactory, IBusModifierState,
};
use zbus::{
    fdo, object_server::SignalEmitter, zvariant::Value, Error as ZbusError, ObjectServer,
};

fn keysym_to_char(keysym: Keysym) -> Option<char> {
    keysym.key_char()
}

fn is_input_char(c: char) -> bool {
    c.is_alphabetic() || c.is_ascii_digit()
}

fn is_shift_key(keysym: Keysym) -> bool {
    matches!(
        keysym,
        Keysym::Shift_L | Keysym::Shift_R | Keysym::Caps_Lock | Keysym::Shift_Lock
    )
}

fn is_reset_modifier_key(keysym: Keysym) -> bool {
    matches!(
        keysym,
        Keysym::Control_L
            | Keysym::Control_R
            | Keysym::Alt_L
            | Keysym::Alt_R
            | Keysym::Super_L
            | Keysym::Super_R
            | Keysym::Meta_L
            | Keysym::Meta_R
            | Keysym::Hyper_L
            | Keysym::Hyper_R
            | Keysym::Mode_switch
            | Keysym::ISO_Level3_Shift
            | Keysym::ISO_Level5_Shift
    )
}

fn is_navigation_key(keysym: Keysym) -> bool {
    keysym.is_cursor_key()
        || matches!(
            keysym,
            Keysym::Delete
                | Keysym::Insert
                | Keysym::KP_Delete
                | Keysym::KP_Insert
                | Keysym::KP_Begin
        )
}

fn is_word_separator_key(keysym: Keysym) -> bool {
    matches!(
        keysym,
        Keysym::space
            | Keysym::Return
            | Keysym::Tab
            | Keysym::Escape
            | Keysym::KP_Enter
            | Keysym::KP_Space
            | Keysym::KP_Tab
            | Keysym::Linefeed
            | Keysym::Clear
    )
}

/// Text to insert ourselves for a separator key, if any.
///
/// Only Space is committed via `commit_text`. Forwarding Space races with a
/// pending `delete_surrounding_text` from the previous keystroke, which can
/// eat the space. Return/Enter/Tab/Escape must be forwarded so apps still
/// receive the real key (e.g. chat send on Enter, focus change on Tab).
fn separator_commit_text(keysym: Keysym) -> Option<&'static str> {
    match keysym {
        Keysym::space | Keysym::KP_Space => Some(" "),
        _ => None,
    }
}

#[derive(Debug, Clone)]
struct GoxkeyEngine {
    last_committed: String,
    method: TypingMethod,
}

impl GoxkeyEngine {
    /// Commit the current transformed word in-place using get_diff_parts
    /// to delete only the changed suffix via delete_surrounding_text,
    /// then committing the new suffix.
    ///
    /// For pure appends, backspace_count is 0 so no delete_surrounding_text call
    /// is made at all. This prevents racing with client buffers and eliminates
    /// spurious forward deletion in Wayland/Mutter when editing text.
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

        let (backspace_count, suffix) = get_diff_parts(&self.last_committed, &display);

        if backspace_count > 0 {
            GoxkeyEngine::delete_surrounding_text(
                se,
                -(backspace_count as i32),
                backspace_count as u32,
            )
            .await?;
        }

        if !suffix.is_empty() {
            GoxkeyEngine::commit_text(se, suffix.to_string()).await?;
        }

        self.last_committed = display;
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

        // Shift / CapsLock keys alone: do not reset tracking or buffer
        if is_shift_key(keyval) {
            return Ok(false);
        }

        unsafe {
            let input = &mut *INPUT_STATE;
            input.set_method_im(self.method);

            // Special modifiers active (Ctrl, Alt, Super, Meta, Hyper) OR modifier keys pressed alone:
            // reset word tracking so shortcuts work and tracking state is cleanly reset.
            if state.has_special_modifiers() || is_reset_modifier_key(keyval) {
                if input.is_enabled() && !input.is_buffer_empty() && input.should_restore_word() {
                    let raw = input.get_typing_buffer().to_string();
                    let (backspace_count, suffix) = get_diff_parts(&self.last_committed, &raw);
                    if backspace_count > 0 {
                        _ = GoxkeyEngine::delete_surrounding_text(
                            &se,
                            -(backspace_count as i32),
                            backspace_count as u32,
                        )
                        .await;
                    }
                    if !suffix.is_empty() {
                        _ = GoxkeyEngine::commit_text(&se, suffix.to_string()).await;
                    }
                }
                input.new_word();
                self.last_committed.clear();
                return Ok(false);
            }

            // Arrow/Cursor keys or navigation (Home, End, PageUp, PageDown, Delete, Insert):
            // The word is already committed on screen. Finalize tracking and let the cursor move.
            if is_navigation_key(keyval) {
                input.new_word();
                self.last_committed.clear();
                return Ok(false);
            }

            // Backspace key
            if keyval == Keysym::BackSpace {
                if input.is_enabled() && !input.is_buffer_empty() {
                    input.pop();
                    if input.is_buffer_empty() {
                        let len = self.last_committed.chars().count();
                        if len > 0 {
                            GoxkeyEngine::delete_surrounding_text(
                                &se,
                                -(len as i32),
                                len as u32,
                            )
                            .await?;
                        }
                        self.last_committed.clear();
                    } else {
                        self.commit_in_place(&se).await?;
                    }
                    debug!("Backspace -> buffer: {:?}", input.get_typing_buffer());
                    return Ok(true);
                }
                self.last_committed.clear();
                input.new_word();
                return Ok(false);
            }

            // Word separators (Space, Return, Tab, Escape, etc.)
            //
            // Space is inserted via commit_text and consumed so it stays
            // ordered with any pending delete_surrounding_text. Other
            // separators are forwarded (return false) so clients still get
            // the real key event — Enter must reach chat boxes to send, etc.
            if is_word_separator_key(keyval) {
                if input.is_enabled() {
                    let sep = separator_commit_text(keyval);

                    if (keyval == Keysym::space || keyval == Keysym::Tab) && !input.is_buffer_empty()
                    {
                        if let Some(target) = input.get_macro_target() {
                            let len = self.last_committed.chars().count();
                            if len > 0 {
                                GoxkeyEngine::delete_surrounding_text(
                                    &se,
                                    -(len as i32),
                                    len as u32,
                                )
                                .await?;
                            }
                            // Fold space into the same commit to save a D-Bus round-trip.
                            let text = match sep {
                                Some(s) => format!("{target}{s}"),
                                None => target,
                            };
                            GoxkeyEngine::commit_text(&se, text).await?;
                            input.new_word();
                            self.last_committed.clear();
                            return Ok(sep.is_some());
                        }
                    }

                    if !input.is_buffer_empty() && input.should_restore_word() {
                        debug!("Restoring word");
                        let raw = input.get_typing_buffer().to_string();
                        let (backspace_count, suffix) = get_diff_parts(&self.last_committed, &raw);
                        if backspace_count > 0 {
                            GoxkeyEngine::delete_surrounding_text(
                                &se,
                                -(backspace_count as i32),
                                backspace_count as u32,
                            )
                            .await?;
                        }
                        let text = match sep {
                            Some(s) => format!("{suffix}{s}"),
                            None => suffix.to_string(),
                        };
                        if !text.is_empty() {
                            GoxkeyEngine::commit_text(&se, text).await?;
                        }
                        input.new_word();
                        self.last_committed.clear();
                        return Ok(sep.is_some());
                    }

                    input.new_word();
                    self.last_committed.clear();

                    if let Some(s) = sep {
                        GoxkeyEngine::commit_text(&se, s.to_string()).await?;
                        return Ok(true);
                    }
                }
                return Ok(false);
            }

            if !input.is_enabled() {
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
                            self.last_committed.clear();
                        }
                        return Ok(true);
                    }
                    return Ok(false);
                }

                // Non-input character (e.g. punctuation, symbols like ., ! ? / ; [ ] etc.)
                if !input.is_buffer_empty() {
                    if input.should_restore_word() {
                        debug!("Restoring word");
                        let raw = input.get_typing_buffer().to_string();
                        let (backspace_count, suffix) = get_diff_parts(&self.last_committed, &raw);
                        if backspace_count > 0 {
                            GoxkeyEngine::delete_surrounding_text(
                                &se,
                                -(backspace_count as i32),
                                backspace_count as u32,
                            )
                            .await?;
                        }
                        if !suffix.is_empty() {
                            GoxkeyEngine::commit_text(&se, suffix.to_string()).await?;
                        }
                    }
                }
                input.new_word();
                self.last_committed.clear();
                return Ok(false);
            }

            // Keysym with no character representation (F1-F12, etc.)
            input.new_word();
            self.last_committed.clear();
            Ok(false)
        }
    }

    async fn set_surrounding_text(
        &mut self,
        _se: SignalEmitter<'_>,
        _server: &ObjectServer,
        _text: Value<'_>,
        _cursor_pos: u32,
        _anchor_pos: u32,
    ) -> fdo::Result<()> {
        Ok(())
    }

    async fn set_cursor_location(
        &mut self,
        _se: SignalEmitter<'_>,
        _server: &ObjectServer,
        _x: i32,
        _y: i32,
        _w: i32,
        _h: i32,
    ) -> fdo::Result<()> {
        Ok(())
    }

    async fn focus_in(&mut self, _se: SignalEmitter<'_>, _server: &ObjectServer) -> fdo::Result<()> {
        debug!("Focus in");
        unsafe {
            let input = &mut *INPUT_STATE;
            input.new_word();
        }
        self.last_committed.clear();
        Ok(())
    }

    async fn focus_out(&mut self, _se: SignalEmitter<'_>, _server: &ObjectServer) -> fdo::Result<()> {
        debug!("Focus out");
        unsafe {
            let input = &mut *INPUT_STATE;
            input.new_word();
        }
        self.last_committed.clear();
        Ok(())
    }

    async fn reset(&mut self, _se: SignalEmitter<'_>, _server: &ObjectServer) -> fdo::Result<()> {
        debug!("Reset");
        unsafe {
            let input = &mut *INPUT_STATE;
            input.new_word();
        }
        self.last_committed.clear();
        Ok(())
    }

    async fn enable(&mut self, _se: SignalEmitter<'_>, _server: &ObjectServer) -> fdo::Result<()> {
        debug!("Enable");
        unsafe {
            let input = &mut *INPUT_STATE;
            input.new_word();
        }
        self.last_committed.clear();
        Ok(())
    }

    async fn disable(&mut self, _se: SignalEmitter<'_>, _server: &ObjectServer) -> fdo::Result<()> {
        info!("Engine disabled");
        unsafe {
            let input = &mut *INPUT_STATE;
            input.new_word();
        }
        self.last_committed.clear();
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
                last_committed: String::new(),
                method: TypingMethod::Telex,
            }),
            "goxkey-vni" => Ok(GoxkeyEngine {
                last_committed: String::new(),
                method: TypingMethod::VNI,
            }),
            "goxkey-telexvni" => Ok(GoxkeyEngine {
                last_committed: String::new(),
                method: TypingMethod::TelexVNI,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keysym_to_char() {
        assert_eq!(keysym_to_char(Keysym::a), Some('a'));
        assert_eq!(keysym_to_char(Keysym::A), Some('A'));
        assert_eq!(keysym_to_char(Keysym::Control_L), None);
        assert_eq!(keysym_to_char(Keysym::Shift_L), None);
        assert_eq!(keysym_to_char(Keysym::Alt_L), None);
        assert_eq!(keysym_to_char(Keysym::Super_L), None);
        assert_eq!(keysym_to_char(Keysym::Left), None);
        assert_eq!(keysym_to_char(Keysym::BackSpace), Some('\u{08}'));
        assert_eq!(keysym_to_char(Keysym::space), Some(' '));
    }

    #[test]
    fn test_keysym_categories() {
        assert!(is_shift_key(Keysym::Shift_L));
        assert!(is_shift_key(Keysym::Shift_R));
        assert!(is_shift_key(Keysym::Caps_Lock));

        assert!(is_reset_modifier_key(Keysym::Control_L));
        assert!(is_reset_modifier_key(Keysym::Control_R));
        assert!(is_reset_modifier_key(Keysym::Alt_L));
        assert!(is_reset_modifier_key(Keysym::Alt_R));
        assert!(is_reset_modifier_key(Keysym::Super_L));
        assert!(is_reset_modifier_key(Keysym::Super_R));

        assert!(is_navigation_key(Keysym::Left));
        assert!(is_navigation_key(Keysym::Right));
        assert!(is_navigation_key(Keysym::Up));
        assert!(is_navigation_key(Keysym::Down));
        assert!(is_navigation_key(Keysym::Home));
        assert!(is_navigation_key(Keysym::End));
        assert!(is_navigation_key(Keysym::Page_Up));
        assert!(is_navigation_key(Keysym::Page_Down));
        assert!(is_navigation_key(Keysym::Delete));
        assert!(is_navigation_key(Keysym::Insert));

        assert!(is_word_separator_key(Keysym::space));
        assert!(is_word_separator_key(Keysym::Return));
        assert!(is_word_separator_key(Keysym::Tab));
        assert!(is_word_separator_key(Keysym::Escape));
        assert!(is_word_separator_key(Keysym::KP_Enter));
    }

    #[test]
    fn test_separator_commit_text() {
        assert_eq!(separator_commit_text(Keysym::space), Some(" "));
        assert_eq!(separator_commit_text(Keysym::KP_Space), Some(" "));
        // Action keys must be forwarded, not inserted as text.
        assert_eq!(separator_commit_text(Keysym::Tab), None);
        assert_eq!(separator_commit_text(Keysym::KP_Tab), None);
        assert_eq!(separator_commit_text(Keysym::Return), None);
        assert_eq!(separator_commit_text(Keysym::KP_Enter), None);
        assert_eq!(separator_commit_text(Keysym::Linefeed), None);
        assert_eq!(separator_commit_text(Keysym::Escape), None);
        assert_eq!(separator_commit_text(Keysym::Clear), None);
    }

    #[test]
    fn test_in_place_diff_appends_without_deletion() {
        // Appending characters one by one should have backspace_count == 0
        let (bs, sfx) = get_diff_parts("", "t");
        assert_eq!(bs, 0);
        assert_eq!(sfx, "t");

        let (bs, sfx) = get_diff_parts("t", "ti");
        assert_eq!(bs, 0);
        assert_eq!(sfx, "i");

        let (bs, sfx) = get_diff_parts("ti", "tie");
        assert_eq!(bs, 0);
        assert_eq!(sfx, "e");

        let (bs, sfx) = get_diff_parts("tie", "tien");
        assert_eq!(bs, 0);
        assert_eq!(sfx, "n");

        let (bs, sfx) = get_diff_parts("tien", "tieng");
        assert_eq!(bs, 0);
        assert_eq!(sfx, "g");
    }

    #[test]
    fn test_in_place_diff_transformation_minimal_delete() {
        // When 'e' is pressed on "tie" -> "tiê": only delete 1 ('e'), commit "ê"
        let (bs, sfx) = get_diff_parts("tie", "tiê");
        assert_eq!(bs, 1);
        assert_eq!(sfx, "ê");

        // When 's' is pressed on "tiêng" -> "tiếng": only delete 3 ("êng"), commit "ếng"
        let (bs, sfx) = get_diff_parts("tiêng", "tiếng");
        assert_eq!(bs, 3);
        assert_eq!(sfx, "ếng");

        // Prefix "ti" was preserved; backspace_count (3) <= old length (5)
        assert!(bs <= "tiêng".chars().count());
    }

    #[test]
    fn test_in_place_diff_editing_in_middle() {
        // Editing "tro" -> "trời"
        let (bs, sfx) = get_diff_parts("troi", "tròi");
        assert_eq!(bs, 2);
        assert_eq!(sfx, "òi");

        // Editing "viet" -> "việt": common prefix is "vi" (2 chars),
        // so only 2 backspaces ("et") are needed, committing "ệt"
        let (bs, sfx) = get_diff_parts("viet", "việt");
        assert_eq!(bs, 2);
        assert_eq!(sfx, "ệt");
    }

    #[test]
    fn test_words_ong_anh_viet_nam_tu_te_ong_nuoc() {
        let cases = [
            ("vieetj", "việt"),
            ("nam", "nam"),
            ("tuwr", "tử"),
            ("tees", "tế"),
            ("oongs", "ống"),
            ("nuowcs", "nước"),
            ("ongs", "óng"),
            ("anhs", "ánh"),
        ];

        for (input_seq, expected) in cases {
            let mut state = goxkey_core::InputState::new();
            for c in input_seq.chars() {
                state.push(c);
            }
            let (out, _) = state.transform_keys().unwrap();
            assert_eq!(out, expected, "Failed for input sequence: {}", input_seq);
        }
    }
}
