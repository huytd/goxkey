use std::sync::{Arc, Mutex};
use std::sync::mpsc::{self, Receiver, Sender};
use std::time::Duration;

// ── SmartState ────────────────────────────────────────────────────────────────

/// Tracks the raw ASCII word being typed and the committed Vietnamese context.
#[derive(Default)]
pub struct SmartState {
    /// Raw ASCII letters accumulating for the current word (e.g. "tieng")
    pub current_word: String,
    /// Accented Vietnamese sentence committed so far (e.g. "đây là")
    /// Passed to the model as context for each new prediction.
    pub committed_context: String,
}

impl SmartState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push_char(&mut self, c: char) {
        self.current_word.push(c);
    }

    pub fn pop_char(&mut self) {
        self.current_word.pop();
    }

    /// Commit a predicted word into the sentence context.
    /// Returns the raw word byte length — caller uses this to send the right
    /// number of fake backspaces.
    pub fn commit_word(&mut self, predicted: &str) -> usize {
        let raw_len = self.current_word.chars().count();
        if !self.committed_context.is_empty() {
            self.committed_context.push(' ');
        }
        self.committed_context.push_str(predicted);
        self.current_word.clear();
        raw_len
    }

    /// Fallback: commit the raw word unchanged (e.g. on timeout/error).
    pub fn commit_raw(&mut self) -> usize {
        let raw = self.current_word.clone();
        self.commit_word(&raw)
    }

    /// Reset everything — call on Enter, Escape, focus loss, mode switch.
    pub fn reset(&mut self) {
        self.current_word.clear();
        self.committed_context.clear();
    }

    pub fn has_word(&self) -> bool {
        !self.current_word.is_empty()
    }

    /// Returns true for inputs that are clearly not Vietnamese words.
    pub fn should_skip_prediction(&self) -> bool {
        self.current_word.len() > 20
            || self.current_word.chars().any(|c| !c.is_ascii_alphabetic())
    }
}

// ── Worker channel ────────────────────────────────────────────────────────────

struct PredictRequest {
    raw_word: String,
    context: String,
}

pub struct SmartWorker {
    request_tx: Sender<PredictRequest>,
    result_rx: Receiver<Option<String>>,
}

impl SmartWorker {
    /// Spawn the inference thread that owns the SmartEngine.
    pub fn spawn(engine: crate::smart_predict::SmartEngine) -> Self {
        let (req_tx, req_rx) = mpsc::channel::<PredictRequest>();
        let (res_tx, res_rx) = mpsc::channel::<Option<String>>();

        std::thread::Builder::new()
            .name("smart-predict".into())
            .spawn(move || {
                for req in req_rx {
                    let result = engine.predict(&req.raw_word, &req.context);
                    let _ = res_tx.send(result);
                }
            })
            .expect("failed to spawn smart-predict thread");

        SmartWorker {
            request_tx: req_tx,
            result_rx: res_rx,
        }
    }

    /// Send a prediction request and block up to `timeout` for the result.
    /// Returns None on timeout or inference error.
    pub fn predict_blocking(
        &self,
        raw_word: &str,
        context: &str,
        timeout: Duration,
    ) -> Option<String> {
        self.request_tx
            .send(PredictRequest {
                raw_word: raw_word.to_string(),
                context: context.to_string(),
            })
            .ok()?;
        self.result_rx.recv_timeout(timeout).ok().flatten()
    }
}

// ── Global worker slot ────────────────────────────────────────────────────────

/// Shared between the key-event thread and the background loader thread.
pub type WorkerSlot = Arc<Mutex<Option<SmartWorker>>>;

pub fn new_worker_slot() -> WorkerSlot {
    Arc::new(Mutex::new(None))
}
