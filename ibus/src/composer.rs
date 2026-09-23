//! Typing logic for one IBus input context, independent of D-Bus.
//!
//! Every key produces a list of [`Action`]s for the engine to emit, plus
//! whether the key was consumed. Each word is composed in one of two modes:
//!
//! - **Surrounding**: the word is committed as it is typed and corrected with
//!   `DeleteSurroundingText`. Fast and invisible, but only safe when the
//!   client really applies the deletes.
//! - **Preedit**: the word is shown as preedit text and committed once when it
//!   ends. Works in every client.
//!
//! Surrounding mode is only used once the client has proved it works: after a
//! preedit word is committed, a later surrounding-text report must end with
//! that word. Reports can lag several keys behind (on Wayland they travel
//! app -> compositor -> IBus), so reports that don't match yet are ignored;
//! a client that never confirms simply stays in preedit. A report showing that
//! a delete was ignored switches the rest of that focus to preedit. Positive
//! verdicts are cached per client name.

use goxkey_core::{get_diff_parts, InputState, TypingMethod};
use librush::ibus::IBusModifierState;
use log::debug;
use xkeysym::Keysym;

use crate::keys::{
    ends_leading_number, is_input_char, is_navigation_key, is_reset_modifier_key, is_shift_key,
    is_word_separator_key, keysym_to_char, separator_commit_text,
};
use crate::respell::respell;
use crate::support::{ClientCache, Support};

// `IBusCapabilite` and `IBusInputPurpose` values from ibustypes.h.
const CAP_SURROUNDING_TEXT: u32 = 1 << 5;
const PURPOSE_PASSWORD: u32 = 8;
const PURPOSE_PIN: u32 = 9;
const PURPOSE_TERMINAL: u32 = 10;

/// Reports to wait for before dropping an unresolved delete probe. Reports can
/// lag a few keys behind, so allow several.
const PROBE_REPORTS: u8 = 6;
/// How far before the cursor to look for a probed edit, in characters beyond
/// the probe text itself. Covers keys typed after the edit.
const PROBE_SLACK: usize = 16;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    /// Delete this many characters before the cursor.
    Delete(usize),
    Commit(String),
    /// Show this preedit text; empty hides it.
    Preedit(String),
}

pub struct Outcome {
    pub actions: Vec<Action>,
    /// True when the key was consumed; false forwards it to the client.
    pub handled: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WordMode {
    Surrounding,
    Preedit,
}

/// What the text before the cursor ends with once a delete has been applied,
/// and what it ends with if the client ignored it.
struct DeleteProbe {
    applied: String,
    ignored: String,
    reports_left: u8,
}

pub struct Composer {
    input: InputState,
    /// The current word as the user sees it (committed or preedit).
    shown: String,
    mode: WordMode,
    caps: u32,
    purpose: u32,
    client: String,
    support: Support,
    cache: ClientCache,
    /// Text last committed from preedit, waiting for a report ending with it.
    pending_check: Option<String>,
    probe: Option<DeleteProbe>,
    /// Set when the client ignored a delete or showed a selection mid-word
    /// (e.g. inline autocomplete); later words in this focus use preedit.
    prefer_preedit: bool,
}

impl Composer {
    pub fn new(method: TypingMethod, cache: ClientCache) -> Self {
        let mut input = InputState::new();
        input.set_method_im(method);
        Self {
            input,
            shown: String::new(),
            mode: WordMode::Preedit,
            // Assume support until the client says otherwise.
            caps: CAP_SURROUNDING_TEXT,
            purpose: 0,
            client: String::new(),
            support: Support::Unknown,
            cache,
            pending_check: None,
            probe: None,
            prefer_preedit: false,
        }
    }

    pub fn set_capabilities(&mut self, caps: u32) {
        debug!("Capabilities: {:#x}", caps);
        self.caps = caps;
    }

    pub fn set_content_type(&mut self, purpose: u32, hints: u32) {
        debug!("Content type: purpose={} hints={:#x}", purpose, hints);
        self.purpose = purpose;
        self.discard_word();
    }

    /// `client` is the IBus client name when known (from `FocusInId`).
    pub fn focus_in(&mut self, client: Option<&str>) {
        self.input.sync_config();
        if let Some(client) = client {
            if client != self.client {
                self.client = client.to_string();
                self.support = self.cache.get(client).unwrap_or(Support::Unknown);
                debug!("Focus in {:?}: {:?}", client, self.support);
            }
        }
        self.prefer_preedit = false;
        self.pending_check = None;
        self.probe = None;
        self.discard_word();
    }

    /// Forget the current word without emitting anything. Used for focus
    /// changes and resets, where the client commits any preedit itself
    /// (preedit is sent with `IBusPreeditFocusMode::Commit`).
    pub fn discard_word(&mut self) {
        self.input.new_word();
        self.shown.clear();
    }

    pub fn set_surrounding_text(&mut self, text: &str, cursor: u32, anchor: u32) {
        let before: String = text.chars().take(cursor as usize).collect();

        if let Some(expected) = &self.pending_check {
            if before.ends_with(expected.as_str()) {
                self.pending_check = None;
                self.set_support(Support::Works);
            }
        }

        if let Some(probe) = &mut self.probe {
            let window = tail(&before, probe.ignored.chars().count() + PROBE_SLACK);
            if window.contains(&probe.ignored) {
                debug!("Client ignored delete_surrounding_text; using preedit for this focus");
                self.probe = None;
                self.prefer_preedit = true;
                // The rest of this word cannot be fixed in place.
                self.discard_word();
            } else if window.contains(&probe.applied) {
                self.probe = None;
            } else {
                probe.reports_left -= 1;
                if probe.reports_left == 0 {
                    self.probe = None;
                }
            }
        }

        if cursor != anchor && self.mode == WordMode::Surrounding && !self.shown.is_empty() {
            debug!("Selection while composing; using preedit for later words");
            self.prefer_preedit = true;
        }
    }

    pub fn process_key(&mut self, keyval: Keysym, state: IBusModifierState) -> Outcome {
        let mut actions = Vec::new();
        let handled = self.handle_key(keyval, state, &mut actions);
        Outcome { actions, handled }
    }

    fn handle_key(
        &mut self,
        keyval: Keysym,
        state: IBusModifierState,
        out: &mut Vec<Action>,
    ) -> bool {
        if state.is_keyup() || is_shift_key(keyval) || self.is_passthrough() {
            return false;
        }
        if !self.input.is_enabled() {
            return false;
        }

        // Shortcuts and modifier keys pressed alone end the word so shortcuts
        // act on final text.
        if state.has_special_modifiers() || is_reset_modifier_key(keyval) {
            let text = self.final_text();
            self.end_word(&text, None, out);
            return false;
        }

        // The cursor is about to move: keep the word as it is.
        if is_navigation_key(keyval) {
            self.end_word_as_shown(out);
            return false;
        }

        if keyval == Keysym::BackSpace {
            return self.backspace(out);
        }

        // Space is committed by us so it stays ordered with any pending
        // delete; other separators are forwarded so apps get the real key
        // (Enter sends a chat message, Tab moves focus, ...).
        if is_word_separator_key(keyval) {
            let sep = separator_commit_text(keyval);
            let text = match keyval {
                Keysym::space | Keysym::Tab if !self.input.is_buffer_empty() => {
                    self.input.get_macro_target()
                }
                _ => None,
            }
            .unwrap_or_else(|| self.final_text());
            self.end_word(&text, sep, out);
            return sep.is_some();
        }

        match keysym_to_char(keyval) {
            Some(c) if is_input_char(c) => self.letter(c, out),
            Some(_) => {
                // Punctuation and symbols end the word; the key is forwarded.
                let text = self.final_text();
                self.end_word(&text, None, out);
                false
            }
            None => {
                // F1-F12 and other keys without a character.
                self.end_word_as_shown(out);
                false
            }
        }
    }

    fn letter(&mut self, c: char, out: &mut Vec<Action>) -> bool {
        if !self.input.is_tracking() {
            return false;
        }
        if ends_leading_number(self.input.get_method(), self.input.get_typing_buffer(), c) {
            self.end_word_as_shown(out);
        }
        if self.input.is_buffer_empty() {
            self.mode = self.word_mode();
            debug!("New word in {:?} mode", self.mode);
        }

        self.input.push(c);
        if let Ok((transformed, _)) = self.input.transform_keys() {
            self.input.replace(transformed);
        }

        if self.input.should_stop_tracking() {
            // Doubled tone keys or overlong words are usually English: put the
            // typed keys back if the result is not Vietnamese.
            let text = self.final_text();
            self.end_word(&text, None, out);
            self.input.stop_tracking();
        } else {
            let display = self.input.get_displaying_word().to_string();
            self.show(&display, out);
        }
        true
    }

    /// Delete the last visible character and keep composing what is left.
    fn backspace(&mut self, out: &mut Vec<Action>) -> bool {
        if self.input.is_buffer_empty() {
            self.discard_word();
            return false;
        }

        let mut chars: Vec<char> = self.shown.chars().collect();
        chars.pop();
        let target: String = chars.into_iter().collect();

        let keys_round_trip = !target.is_empty() && {
            let raw = respell(&target, self.input.get_method());
            self.input.set_word(&raw, &target);
            matches!(self.input.transform_keys(), Ok((t, _)) if t == target)
        };

        let handled = match self.mode {
            // A single character before the cursor: let the client delete it
            // natively instead of relying on delete_surrounding_text.
            WordMode::Surrounding => {
                self.shown = target;
                false
            }
            WordMode::Preedit => {
                self.show(&target, out);
                true
            }
        };

        if !keys_round_trip {
            debug!("Backspace: cannot keep composing {:?}", self.shown);
            self.end_word_as_shown(out);
        }
        handled
    }

    fn is_passthrough(&self) -> bool {
        matches!(self.purpose, PURPOSE_PASSWORD | PURPOSE_PIN)
    }

    fn word_mode(&self) -> WordMode {
        let surrounding_ok = self.caps & CAP_SURROUNDING_TEXT != 0
            && self.purpose != PURPOSE_TERMINAL
            && self.support == Support::Works
            && !self.prefer_preedit;
        if surrounding_ok {
            WordMode::Surrounding
        } else {
            WordMode::Preedit
        }
    }

    fn set_support(&mut self, support: Support) {
        if self.support != support {
            debug!(
                "Client {:?}: {:?} -> {:?}",
                self.client, self.support, support
            );
        }
        self.support = support;
        self.cache.set(&self.client, support);
    }

    /// The word to leave on screen: the typed keys when the transformed word
    /// is not valid Vietnamese, otherwise the transformed word.
    fn final_text(&self) -> String {
        if !self.input.is_buffer_empty() && self.input.should_restore_word() {
            debug!("Restoring word");
            self.input.get_typing_buffer().to_string()
        } else {
            self.input.get_displaying_word().to_string()
        }
    }

    /// Update the word on screen to `target`.
    fn show(&mut self, target: &str, out: &mut Vec<Action>) {
        match self.mode {
            WordMode::Surrounding => self.edit_in_place(target, "", out),
            WordMode::Preedit => {
                if target != self.shown {
                    out.push(Action::Preedit(target.to_string()));
                }
            }
        }
        self.shown = target.to_string();
    }

    /// Emit the minimal delete + commit turning `shown` into `target`,
    /// followed by `trailing` in the same commit.
    fn edit_in_place(&mut self, target: &str, trailing: &str, out: &mut Vec<Action>) {
        let (deletes, suffix) = get_diff_parts(&self.shown, target);
        if deletes > 0 {
            out.push(Action::Delete(deletes));
            if self.probe.is_none() {
                self.probe = Some(DeleteProbe {
                    applied: target.to_string(),
                    ignored: format!("{}{}", self.shown, suffix),
                    reports_left: PROBE_REPORTS,
                });
            }
        }
        let text = format!("{suffix}{trailing}");
        if !text.is_empty() {
            out.push(Action::Commit(text));
        }
    }

    /// Finish the word as `text`, followed by `trailing`, and start a new one.
    fn end_word(&mut self, text: &str, trailing: Option<&str>, out: &mut Vec<Action>) {
        let trailing = trailing.unwrap_or("");
        match self.mode {
            WordMode::Surrounding => self.edit_in_place(text, trailing, out),
            WordMode::Preedit => {
                if !self.shown.is_empty() {
                    out.push(Action::Preedit(String::new()));
                }
                let committed = format!("{text}{trailing}");
                if !committed.is_empty() {
                    out.push(Action::Commit(committed.clone()));
                }
                // Only words followed by a space we committed ourselves are
                // predictable enough to check against the next report.
                let verifiable = !text.is_empty() && !trailing.is_empty();
                if verifiable
                    && self.support == Support::Unknown
                    && self.caps & CAP_SURROUNDING_TEXT != 0
                {
                    self.pending_check = Some(committed);
                }
            }
        }
        self.discard_word();
    }

    fn end_word_as_shown(&mut self, out: &mut Vec<Action>) {
        let shown = self.shown.clone();
        self.end_word(&shown, None, out);
    }
}

/// The last `n` characters of `s`.
fn tail(s: &str, n: usize) -> &str {
    match s.char_indices().rev().nth(n.saturating_sub(1)) {
        Some((i, _)) if n > 0 => &s[i..],
        _ if n == 0 => "",
        _ => s,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CAP_PREEDIT_TEXT: u32 = 1;

    /// A client that applies actions to its text, like a GTK text field.
    struct FakeClient {
        text: String,
        preedit: String,
        applies_deletes: bool,
        sends_reports: bool,
        deletes_seen: usize,
    }

    impl FakeClient {
        fn new(applies_deletes: bool, sends_reports: bool) -> Self {
            Self {
                text: String::new(),
                preedit: String::new(),
                applies_deletes,
                sends_reports,
                deletes_seen: 0,
            }
        }

        fn visible(&self) -> String {
            format!("{}{}", self.text, self.preedit)
        }

        fn apply(&mut self, actions: Vec<Action>) {
            for action in actions {
                match action {
                    Action::Delete(n) => {
                        self.deletes_seen += 1;
                        if self.applies_deletes {
                            for _ in 0..n {
                                self.text.pop();
                            }
                        }
                    }
                    Action::Commit(s) => self.text.push_str(&s),
                    Action::Preedit(s) => self.preedit = s,
                }
            }
        }

        fn forward(&mut self, key: char) {
            match key {
                '<' => {
                    self.text.pop();
                }
                c => self.text.push(c),
            }
        }
    }

    fn keysym_for(c: char) -> Keysym {
        match c {
            '<' => Keysym::BackSpace,
            '\n' => Keysym::Return,
            ' ' => Keysym::space,
            c => Keysym::from_char(c),
        }
    }

    /// Type `keys` ('<' is Backspace) and return what the client shows.
    fn type_into(composer: &mut Composer, client: &mut FakeClient, keys: &str) -> String {
        for key in keys.chars() {
            if client.sends_reports {
                let len = client.text.chars().count() as u32;
                composer.set_surrounding_text(&client.text, len, len);
            }
            let outcome =
                composer.process_key(keysym_for(key), IBusModifierState::new_with_raw_value(0));
            client.apply(outcome.actions);
            if !outcome.handled {
                client.forward(key);
            }
        }
        client.visible()
    }

    fn composer(method: TypingMethod, caps: u32) -> Composer {
        let mut c = Composer::new(method, ClientCache::in_memory());
        c.set_capabilities(caps);
        c.focus_in(Some("test-client"));
        c
    }

    fn telex(keys: &str) -> String {
        let mut c = composer(TypingMethod::Telex, CAP_PREEDIT_TEXT);
        type_into(&mut c, &mut FakeClient::new(false, false), keys)
    }

    #[test]
    fn no_surrounding_capability_uses_preedit_only() {
        let mut c = composer(TypingMethod::Telex, CAP_PREEDIT_TEXT);
        let mut client = FakeClient::new(false, false);
        assert_eq!(
            type_into(&mut c, &mut client, "tieengs vieetj "),
            "tiếng việt "
        );
        assert_eq!(client.deletes_seen, 0);
        assert_eq!(c.support, Support::Unknown);
    }

    #[test]
    fn verified_client_switches_to_surrounding() {
        let mut c = composer(TypingMethod::Telex, CAP_PREEDIT_TEXT | CAP_SURROUNDING_TEXT);
        let mut client = FakeClient::new(true, true);
        assert_eq!(type_into(&mut c, &mut client, "tieengs "), "tiếng ");
        assert_eq!(client.deletes_seen, 0, "first word is composed in preedit");
        assert_eq!(type_into(&mut c, &mut client, "vieetj "), "tiếng việt ");
        assert_eq!(c.support, Support::Works);
        assert!(client.deletes_seen > 0, "second word is edited in place");
        assert_eq!(c.cache.get("test-client"), Some(Support::Works));
    }

    #[test]
    fn client_ignoring_deletes_garbles_at_most_one_word() {
        let mut c = composer(TypingMethod::Telex, CAP_PREEDIT_TEXT | CAP_SURROUNDING_TEXT);
        let mut client = FakeClient::new(false, true);
        type_into(&mut c, &mut client, "tieengs ");
        // The report sent with the next key confirms the first word.
        type_into(&mut c, &mut client, "v");
        assert_eq!(c.support, Support::Works);
        type_into(&mut c, &mut client, "ieetj ");
        assert!(c.prefer_preedit);
        let before = client.visible();
        assert_eq!(
            type_into(&mut c, &mut client, "nguowif "),
            format!("{before}người ")
        );
    }

    #[test]
    fn client_that_never_confirms_stays_in_preedit() {
        let mut c = composer(TypingMethod::Telex, CAP_PREEDIT_TEXT | CAP_SURROUNDING_TEXT);
        // Reports arrive but never contain our text (e.g. a stale buffer).
        let mut client = FakeClient::new(true, false);
        for word in ["mootj ", "hai ", "ba ", "boons "] {
            type_into(&mut c, &mut client, word);
            c.set_surrounding_text("", 0, 0);
        }
        assert_eq!(c.support, Support::Unknown);
        assert_eq!(client.visible(), "một hai ba bốn ");
        assert_eq!(client.deletes_seen, 0);
    }

    #[test]
    fn late_reports_still_verify() {
        let mut c = composer(TypingMethod::Telex, CAP_PREEDIT_TEXT | CAP_SURROUNDING_TEXT);
        let mut client = FakeClient::new(true, false);
        type_into(&mut c, &mut client, "mootj ");
        // Reports from before the commit arrive first (Wayland latency).
        c.set_surrounding_text("", 0, 0);
        c.set_surrounding_text("m", 1, 1);
        assert_eq!(c.support, Support::Unknown);
        c.set_surrounding_text("một ", 4, 4);
        assert_eq!(c.support, Support::Works);
    }

    #[test]
    fn ignored_delete_detected_from_late_report() {
        let cache = ClientCache::in_memory();
        cache.set("app", Support::Works);
        let mut c = Composer::new(TypingMethod::Telex, cache);
        c.focus_in(Some("app"));
        // No automatic reports: the report arrives after more keys.
        let mut client = FakeClient::new(false, false);
        type_into(&mut c, &mut client, "tieen");
        assert!(!c.prefer_preedit);
        c.set_surrounding_text(
            &client.text,
            client.text.chars().count() as u32,
            client.text.chars().count() as u32,
        );
        assert!(c.prefer_preedit);
    }

    #[test]
    fn tail_takes_last_chars() {
        assert_eq!(tail("tiếng việt", 4), "việt");
        assert_eq!(tail("ab", 5), "ab");
        assert_eq!(tail("ab", 0), "");
    }

    #[test]
    fn cached_verdict_applies_to_new_context() {
        let cache = ClientCache::in_memory();
        cache.set("firefox", Support::Works);
        let mut c = Composer::new(TypingMethod::Telex, cache);
        c.focus_in(Some("firefox"));
        let mut client = FakeClient::new(true, true);
        assert_eq!(type_into(&mut c, &mut client, "vieetj "), "việt ");
        assert!(client.deletes_seen > 0);
    }

    #[test]
    fn terminal_always_uses_preedit() {
        let cache = ClientCache::in_memory();
        cache.set("term", Support::Works);
        let mut c = Composer::new(TypingMethod::Telex, cache);
        c.focus_in(Some("term"));
        c.set_content_type(PURPOSE_TERMINAL, 0);
        let mut client = FakeClient::new(false, true);
        assert_eq!(type_into(&mut c, &mut client, "vieetj "), "việt ");
        assert_eq!(client.deletes_seen, 0);
    }

    #[test]
    fn password_fields_pass_keys_through() {
        let mut c = composer(TypingMethod::Telex, CAP_PREEDIT_TEXT);
        c.set_content_type(PURPOSE_PASSWORD, 0);
        assert_eq!(
            type_into(&mut c, &mut FakeClient::new(true, false), "aas"),
            "aas"
        );
    }

    #[test]
    fn selection_mid_word_prefers_preedit_afterwards() {
        let cache = ClientCache::in_memory();
        cache.set("browser", Support::Works);
        let mut c = Composer::new(TypingMethod::Telex, cache);
        c.focus_in(Some("browser"));
        let mut client = FakeClient::new(true, false);
        type_into(&mut c, &mut client, "vi");
        c.set_surrounding_text("vietnamnet", 2, 10);
        type_into(&mut c, &mut client, "eetj ");
        let deletes = client.deletes_seen;
        assert_eq!(type_into(&mut c, &mut client, "tieengs "), "việt tiếng ");
        assert_eq!(client.deletes_seen, deletes);
    }

    #[test]
    fn enter_and_punctuation_commit_the_word() {
        assert_eq!(telex("vieetj\n"), "việt\n");
        assert_eq!(telex("vieetj."), "việt.");
        assert_eq!(telex("vieetj, nam"), "việt, nam");
    }

    #[test]
    fn backspace_deletes_last_visible_character() {
        assert_eq!(telex("vieetj<"), "việ");
        assert_eq!(telex("vieetj<<"), "vi");
        // Composition continues on what is left.
        assert_eq!(telex("tieengs<<ng "), "tiếng ");
        assert_eq!(telex("vieet<j "), "việ ");
        assert_eq!(telex("a<<b"), "b");
    }

    #[test]
    fn backspace_in_surrounding_mode_is_forwarded() {
        let cache = ClientCache::in_memory();
        cache.set("app", Support::Works);
        let mut c = Composer::new(TypingMethod::Telex, cache);
        c.focus_in(Some("app"));
        let mut client = FakeClient::new(true, true);
        assert_eq!(type_into(&mut c, &mut client, "vieetj<<eej "), "việ ");
    }

    #[test]
    fn english_words_are_restored() {
        assert_eq!(telex("boss "), "boss ");
        assert_eq!(telex("Office "), "Office ");
        assert_eq!(telex("class "), "class ");
        assert_eq!(telex("text "), "text ");
        assert_eq!(telex("window "), "window ");
    }

    #[test]
    fn vietnamese_words() {
        let cases = [
            ("vieetj ", "việt "),
            ("tuwr ", "tử "),
            ("nuowcs ", "nước "),
            ("nguowif ", "người "),
            ("giuwax ", "giữa "),
            ("ddaays ", "đấy "),
        ];
        for (keys, expected) in cases {
            assert_eq!(telex(keys), expected, "keys: {keys:?}");
        }
    }

    #[test]
    fn vni_keeps_leading_digits() {
        let mut c = composer(TypingMethod::VNI, CAP_PREEDIT_TEXT);
        let mut client = FakeClient::new(false, false);
        assert_eq!(type_into(&mut c, &mut client, "10am viet65 "), "10am việt ");
    }
}
