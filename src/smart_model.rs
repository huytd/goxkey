use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use once_cell::sync::Lazy;

use crate::smart::{new_worker_slot, WorkerSlot};

// ── Model constants ───────────────────────────────────────────────────────────

pub const MODEL_FILENAME: &str = "Qwen2.5-1.5B-Instruct-Q4_K_M.gguf";
const MODEL_URL: &str =
    "https://huggingface.co/Qwen/Qwen2.5-1.5B-Instruct-GGUF/resolve/main/\
     qwen2.5-1.5b-instruct-q4_k_m.gguf";

// ── Load status ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum LoadStatus {
    Idle,
    Downloading { pct: u8 },
    Loading,
    Ready,
    Failed(String),
}

/// Global status — polled by the UI to update the system tray title.
pub static SMART_LOAD_STATUS: Lazy<Arc<Mutex<LoadStatus>>> =
    Lazy::new(|| Arc::new(Mutex::new(LoadStatus::Idle)));

/// Global worker slot — set once loading completes.
pub static SMART_WORKER: Lazy<WorkerSlot> = Lazy::new(new_worker_slot);

// ── Path helpers ──────────────────────────────────────────────────────────────

/// ~/.config/goxkey/models/<filename>  (falls back to ~/.goxkey/models/ on macOS
/// if XDG config dir is unavailable — matches the rest of goxkey's config style)
pub fn model_path() -> PathBuf {
    // goxkey stores its config in ~/.goxkey (see config.rs: get_home_dir().join(".goxkey"))
    // We mirror that convention for model storage.
    let base = dirs::home_dir()
        .expect("Cannot find home directory")
        .join(".goxkey")
        .join("models");
    base.join(MODEL_FILENAME)
}

pub fn is_model_downloaded() -> bool {
    model_path().exists()
}

// ── Download ──────────────────────────────────────────────────────────────────

fn download_model(on_progress: impl Fn(u64, u64)) -> anyhow::Result<()> {
    let dest = model_path();
    std::fs::create_dir_all(dest.parent().unwrap())?;

    let response = reqwest::blocking::Client::new()
        .get(MODEL_URL)
        .timeout(std::time::Duration::from_secs(3600))
        .send()?;

    anyhow::ensure!(
        response.status().is_success(),
        "HTTP {} downloading model",
        response.status()
    );

    let total = response.content_length().unwrap_or(986_000_000);
    let mut downloaded: u64 = 0;

    // Write to a .tmp file then rename atomically to avoid leaving partial files
    let tmp = dest.with_extension("gguf.tmp");
    let mut file = std::fs::File::create(&tmp)?;
    let mut stream = response;
    let mut buf = vec![0u8; 128 * 1024];

    loop {
        let n = stream.read(&mut buf)?;
        if n == 0 { break; }
        file.write_all(&buf[..n])?;
        downloaded += n as u64;
        on_progress(downloaded, total);
    }

    drop(file);
    std::fs::rename(&tmp, &dest)?;
    Ok(())
}

// ── Background loader ─────────────────────────────────────────────────────────

/// Kick off model download (if needed) and load on a background thread.
/// Progress is written into SMART_LOAD_STATUS; when done, SMART_WORKER is set.
/// Call once on first Smart mode activation.
pub fn ensure_model_loaded(n_threads: u32) {
    // If already loading/loaded, do nothing
    {
        let status = SMART_LOAD_STATUS.lock().unwrap();
        match *status {
            LoadStatus::Idle | LoadStatus::Failed(_) => {}
            _ => return, // already in progress or done
        }
    }

    let status_ref = Arc::clone(&SMART_LOAD_STATUS);
    let worker_ref = Arc::clone(&SMART_WORKER);

    std::thread::Builder::new()
        .name("smart-loader".into())
        .spawn(move || {
            // Step 1: download if needed
            if !is_model_downloaded() {
                let status_clone = Arc::clone(&status_ref);
                let result = download_model(move |dl, total| {
                    let pct = ((dl * 100) / total.max(1)) as u8;
                    *status_clone.lock().unwrap() = LoadStatus::Downloading { pct };
                });
                if let Err(e) = result {
                    *status_ref.lock().unwrap() =
                        LoadStatus::Failed(format!("Download failed: {e}"));
                    return;
                }
            }

            // Step 2: load model into memory (~1–3s)
            *status_ref.lock().unwrap() = LoadStatus::Loading;

            match crate::smart_predict::SmartEngine::load(&model_path(), n_threads) {
                Ok(engine) => {
                    let worker = crate::smart::SmartWorker::spawn(engine);
                    *worker_ref.lock().unwrap() = Some(worker);
                    *status_ref.lock().unwrap() = LoadStatus::Ready;
                }
                Err(e) => {
                    *status_ref.lock().unwrap() =
                        LoadStatus::Failed(format!("Load failed: {e}"));
                }
            }
        })
        .expect("failed to spawn smart-loader thread");
}
