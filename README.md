# Attention
The code in this fork is created entirely by Gemini (3 Flash).

# Status
Compiles, runs to UI but IME stack is broken. Yet to be implemented.
- [x] IME stack rewrite/implement for Windows
Somewhat works. Athough not without errors. Currently fixing.
- [ ] Tray icon
- [ ] Further implementation due to dev's abandonment of the Windows' version plan

<p align="center">
	<img src="./icons/icon.png" width="90px">
</p>


<img width="1585" height="797" alt="screenshots" src="https://github.com/user-attachments/assets/42930e8a-77fb-4493-aa90-3c0bb9f1ab40" />


**Gõkey** - A Vietnamese input method editor.

- :zap: Excellent performance (Gen Z translation: Blazing fast!)
- :crab: Written completely in Rust.
- :keyboard: Supported both Telex and VNI input method.
- :sparkles: Focused on typing experience and features that you will use.

## Why another Vietnamese IME?

> technical curiosity

## About

This is my attempt to build an input method editor using only Rust. It's not the first, and definitely not the last.

The goal is to create an input method editor that enable users to type Vietnamese text on the computer using
either VNI or TELEX method. Other than that, no other features are planned.

## How to install

There are 2 options to download GõKey at this moment: Build from source or Download the Nightly build.

### Option 1: Download the Nightly Build

Nightly build is the prebuilt binary that automatically bundled everytime we merged the code to the `main` branch.

You can download it at the Release page here: https://github.com/huytd/goxkey/releases/tag/nightly-build

### Option 2: Build from source

The source code can be compiled easily:

1. Get the latest stable version of the Rust compiler ([see here](https://rustup.rs/))
2. Install the [cargo-bundle](https://github.com/burtonageo/cargo-bundle) extension, this is necessary for bundling macOS apps
3. Checkout the source code of the **gõkey** project
   ```
   git clone https://github.com/huytd/goxkey && cd goxkey
   ```
4. Run the build command:

   ```
   cargo build
   ```

Go to 'build' and you'll see goxkey.exe. Run it.

## Dependencies
- [core-foundation](https://crates.io/crates/core-foundation), [core-graphics](https://crates.io/crates/core-graphics): for event handling on macOS
- [vi-rs](https://github.com/zerox-dg/vi-rs): the Vietnamese Input Engine

