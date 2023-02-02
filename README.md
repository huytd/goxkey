<p align="center">
   <picture>
     <source media="(prefers-color-scheme: dark)" srcset="https://user-images.githubusercontent.com/613943/213103585-33c79982-617d-4f81-a949-3335b465129a.png">
     <img src="https://user-images.githubusercontent.com/613943/213103583-4399e224-841f-462a-887d-6427f7af7376.png" width="90px">
   </picture>
</p>

---
<img width="320" alt="image" align="right" src="https://user-images.githubusercontent.com/613943/213217673-e58c873a-9219-4a33-8487-620a07210206.png">

**Goxkey** - A Vietnamese input method editor.

- Excellent performance (Gen Z translation: Blazing fast!)
- Written completely in Rust.
- Supported both Telex and VNI input method.
- Focused on typing experience and features that you will use.
- Developed by some of the best Vietnamese engineers.

<p style="clear: both"></p>

---


## About

This is my attempt to build an input method editor using only Rust. It's not the first, and definitely not the last.

The goal is to create an input method editor that enable users to type Vietnamese text on the computer using
either VNI or TELEX method. Other than that, no other features are planned.

## How to install

Currently, since we are still at an early development stage, no pre-built binaries are provided.

But the source code can be compiled easily:

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

After that, you'll find the `Gõ Key.app` file in the `target/debug/bundle` folder.

## Dependencies

- [core-foundation](https://crates.io/crates/core-foundation), [core-graphics](https://crates.io/crates/core-graphics): for event handling on macOS
- [vi-rs](https://github.com/zerox-dg/vi-rs): the Vietnamese Input Engine

## Development

Currently, only macOS is supported. Windows and Linux could also be supported as well but it's not our primary goal. If you're on these OSes, consider contributing! Any help would be greatly appreciated!

This project will only focus on the input handling logic, and provide a frontend for the
input engine ([`vi-rs`](https://github.com/zerox-dg/vi-rs)).

The following diagram explains how `goxkey` communicates with other components like OS's input source and `vi-rs`:

```
INPUT LAYER
+------------------+              FRONTEND                ENGINE
| macOS            | [d,d,a,a,y]  +---------+ "ddaay"     +-------+
|  +- CGEventTap   | -----------> | goxkey  | ----------> | vi-rs |
|                  |              +---------+             +-------+
| Linux   (TBD)    |               |  ^                    |
| Windows (TBD)    |               |  |              "đây" |
+------------------+               |  +--------------------+
                                   |
                                   | (send_key)
                                   v
                                Target
                                Application
```

On macOS, we run an instance of `CGEventTap` to listen for every `keydown` event. A callback function will be called
on every keystroke. In this callback, we have a buffer (`TYPING_BUF`) to keep track of the word that the user is typing.
This buffer will be reset whenever the user hit the `SPACE` or `ENTER` key.

The input engine (`vi-rs`) will receive this buffer and convert it to a correct word, for example: `vieetj` will be
transformed into `việt`.

The result string will be sent back to `goxkey`, and from there, it will perform an edit on the target application. The edit
is done using [the BACKSPACE technique](https://notes.huy.rocks/posts/go-tieng-viet-linux.html#k%C4%A9-thu%E1%BA%ADt-backspace). It's
unreliable but it has the benefit of not having the pre-edit line so it's worth it.

To get yourself familiar with IME, here are some good article on the topic:

- [Vietnamese Keyboard Engine with Prolog](https://followthe.trailing.space/To-the-Root-of-the-Tree-dc170bf0e8de44a6b812ca3e01025236?p=0dd31fe76ebd45dca5b4466c9441fa1c&pm=s), lewtds
- [Ước mơ bộ gõ kiểu Unikey trên Linux](https://followthe.trailing.space/To-the-Root-of-the-Tree-dc170bf0e8de44a6b812ca3e01025236?p=9b12cc2fcdbe43149b10eefc7db6b161&pm=s), lewtds
- [Vấn đề về IME trên Linux](https://viethung.space/blog/2020/07/21/Van-de-ve-IME-tren-Linux/), zerox-dg
- [Bỏ dấu trong tiếng Việt](https://viethung.space/blog/2020/07/14/Bo-dau-trong-tieng-Viet/), zerox-dg
- [Chuyện gõ tiếng Việt trên Linux](https://notes.huy.rocks/posts/go-tieng-viet-linux.html), huytd

## Local development setup

To setup the project locally, first, checkout the code and run the install script to have all the Git hooks configured:

```sh
$ git clone https://github.com/huytd/goxkey && cd goxkey
$ make setup
```

After this step, you can use the `make` commands to run or bundle the code as needed:

```sh
$ make run

# or

$ make bundle
```
