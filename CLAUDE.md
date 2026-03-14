## Project Overview

Gõkey is a Vietnamese input method editor (IME) for macOS, written in Rust. It
intercepts keyboard events via `CGEventTap`, accumulates typed characters into a
buffer, delegates transformation to the
[`vi-rs`](https://github.com/zerox-dg/vi-rs) crate, then replaces the typed
characters using the backspace technique.

## Commands

```sh
make setup    # Install git hooks (run once after cloning)
make run      # cargo r
make bundle   # cargo bundle (creates .app bundle, requires cargo-bundle)
cargo test    # Run all tests
cargo test <test_name>  # Run a single test by name
```

**Requirements:** `cargo-bundle` must be installed
(`cargo install cargo-bundle`). The app requires macOS Accessibility permission
granted before first run.

## Architecture

### Data Flow

```
macOS CGEventTap → event_handler() in main.rs → INPUT_STATE (global InputState)
    → vi-rs (transformation engine) → send_backspace + send_string → target app
```

App change events feed into `InputState` for auto-toggling Vietnamese per-app.

### Key Modules

- **`src/main.rs`** — Entry point. Sets up the Druid UI window, spawns the
  keyboard event listener thread, and contains `event_handler()` which is the
  core dispatch function for every keystroke.
- **`src/input.rs`** — `InputState`: the central state machine. Manages the
  typing buffer, calls `vi-rs` for transformation, handles macro expansion, word
  restoration (reverting invalid transformations), and app-specific auto-toggle.
- **`src/platform/macos.rs`** — All macOS-specific code: `CGEventTap` setup,
  synthetic key event generation, accessibility permission checks, active app
  detection, system tray.
- **`src/platform/mod.rs`** — Platform abstraction: `PressedKey`, `KeyModifier`
  bitflags, `EventTapType`, and the
  `send_string`/`send_backspace`/`run_event_listener` interface.
- **`src/config.rs`** — `ConfigStore`: reads/writes `~/.goxkey` in a simple
  key-value format. Stores hotkey, input method, macros, VN/EN app lists, and
  allowed words.
- **`src/hotkey.rs`** — Parses hotkey strings (e.g., `"super+shift+z"`) and
  matches them against current key + modifiers.
- **`src/ui/`** — Druid-based settings UI. `views.rs` defines the window layout;
  `data.rs` defines `UIDataAdapter` (Druid data binding); `widgets.rs` has
  custom widgets (`SegmentedControl`, `ToggleSwitch`, `HotkeyBadgesWidget`,
  `AppsListWidget`). The `UPDATE_UI` selector in `selectors.rs` synchronizes
  input state changes to the UI.

### Threading Model

- **Main thread**: Druid UI event loop.
- **Listener thread**: `run_event_listener()` runs the `CGEventTap` callback.
  Communicates back to UI via `EventSink` (stored in global `UI_EVENT_SINK`).
- Global state (`INPUT_STATE`, `UI_EVENT_SINK`) uses `unsafe` static access, as
  callbacks cannot carry context.

### Input Handling Logic (`main.rs` `event_handler`)

1. Check if key matches the toggle hotkey → enable/disable.
2. Non-character keys (arrows, function keys) → reset word tracking.
3. Space/Enter/Tab → finalize word, attempt macro replacement.
4. Backspace → pop character from buffer.
5. Regular character → push to buffer, call `do_transform_keys()` which runs
   `vi-rs` and uses backspace+retype to replace the word.
6. Word restoration: `should_restore_transformed_word()` determines when to
   revert a transformation (e.g., when the user types a non-Vietnamese
   sequence).
