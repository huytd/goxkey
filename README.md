## Deprecation Notice

Development on this project is no longer active. If you're looking for an alternative, consider checking out **VKey** (https://github.com/ldhnam/VKey), which is a promising new project.

---

<p align="center">
	<img src="./icons/icon.png" width="90px">
</p>

---
<img width="432" alt="image" align="right" src="https://github.com/huytd/goxkey/assets/613943/2a65372b-e27f-4b7f-914c-7f2b5c8484fd">


[![Build and Tests](https://github.com/huytd/goxkey/actions/workflows/main.yml/badge.svg)](https://github.com/huytd/goxkey/actions/workflows/main.yml)

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
4. Run the bundle command:

   ```
   cargo bundle
   ```

After that, you'll find the `Gõ Key.app` file in the `target/debug/bundle` folder. Copy it to your `/Applications` folder.

5. **(Important!):** Before you run the app, make you you already allowed Accessibility access for the app. Follow the [guide in the Wiki](https://github.com/huytd/goxkey/wiki/H%C6%B0%E1%BB%9Bng-d%E1%BA%ABn-s%E1%BB%ADa-l%E1%BB%97i-kh%C3%B4ng-g%C3%B5-%C4%91%C6%B0%E1%BB%A3c-ti%E1%BA%BFng-Vi%E1%BB%87t-tr%C3%AAn-macOS) to do so.

Without this step, the app will crash and can't be use.

## Dependencies

- [core-foundation](https://crates.io/crates/core-foundation), [core-graphics](https://crates.io/crates/core-graphics): for event handling on macOS
- [vi-rs](https://github.com/zerox-dg/vi-rs): the Vietnamese Input Engine

## Fun fact

Do you know how to type gõkey in Telex?

Do this: `gox<cmd>key`

