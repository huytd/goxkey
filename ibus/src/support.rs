//! Whether a client really applies `DeleteSurroundingText`, remembered per
//! client name so each app only has to be probed once.

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use log::{debug, warn};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Support {
    /// Not verified yet: compose in preedit until the client proves itself.
    Unknown,
    /// Surrounding-text reports match what we committed; deletes work.
    Works,
    /// The client does not report or does not apply deletes: always preedit.
    Broken,
}

impl Support {
    fn as_str(self) -> &'static str {
        match self {
            Support::Unknown => "unknown",
            Support::Works => "works",
            Support::Broken => "broken",
        }
    }

    fn parse(s: &str) -> Option<Self> {
        match s {
            "works" => Some(Support::Works),
            "broken" => Some(Support::Broken),
            _ => None,
        }
    }
}

/// Verdicts keyed by IBus client name (e.g. `gtk3-im:firefox`), shared by all
/// engines and saved as `<verdict>\t<client>` lines.
#[derive(Clone, Default)]
pub struct ClientCache {
    inner: Arc<Mutex<HashMap<String, Support>>>,
    path: Option<PathBuf>,
}

fn default_path() -> Option<PathBuf> {
    let base = std::env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".cache")))?;
    Some(base.join("goxkey").join("ibus-clients"))
}

impl ClientCache {
    /// Load the cache from the user's cache directory.
    pub fn load() -> Self {
        let path = default_path();
        let mut map = HashMap::new();
        if let Some(text) = path.as_ref().and_then(|p| fs::read_to_string(p).ok()) {
            for line in text.lines() {
                if let Some((verdict, client)) = line.split_once('\t') {
                    if let Some(v) = Support::parse(verdict) {
                        map.insert(client.to_string(), v);
                    }
                }
            }
        }
        debug!("Loaded {} client verdicts", map.len());
        Self {
            inner: Arc::new(Mutex::new(map)),
            path,
        }
    }

    /// An in-memory cache that is never written to disk.
    #[cfg(test)]
    pub fn in_memory() -> Self {
        Self::default()
    }

    pub fn get(&self, client: &str) -> Option<Support> {
        if client.is_empty() {
            return None;
        }
        self.inner.lock().unwrap().get(client).copied()
    }

    pub fn set(&self, client: &str, support: Support) {
        if client.is_empty() || support == Support::Unknown {
            return;
        }
        let mut map = self.inner.lock().unwrap();
        if map.insert(client.to_string(), support) == Some(support) {
            return;
        }
        debug!("Client {:?}: surrounding text {}", client, support.as_str());
        let Some(path) = &self.path else { return };
        let mut lines: Vec<String> = map
            .iter()
            .map(|(c, s)| format!("{}\t{}", s.as_str(), c))
            .collect();
        lines.sort();
        let result = path
            .parent()
            .map_or(Ok(()), fs::create_dir_all)
            .and_then(|_| fs::write(path, lines.join("\n") + "\n"));
        if let Err(e) = result {
            warn!("Failed to save client cache {:?}: {}", path, e);
        }
    }
}
