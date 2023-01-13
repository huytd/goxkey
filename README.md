# Bộgõ-rs

This is my attempt to build an input method editor using only Rust. It's not the first, and definitely not the last.

The goal is to create an input method editor that allows the user to type Vietnamese text on the computer. Using
either VNI or TELEX method. Other than that, no other feature are planned.

## Dependencies

- [core-foundation](https://crates.io/crates/core-foundation), [core-graphics](https://crates.io/crates/core-graphics): for event handling on macOS
- [vi-rs](https://github.com/zerox-dg/vi-rs): the Vietnamese Input Engine

## Development

Currently, only macOS is supported. Windows and Linux can be supported later but it's not
in the roadmap. If you're on these OSes, any help would be greatly appreciated!

This project will only focus on the input handling logic, and provide a frontend for the
input engine ([`vi-rs`](https://github.com/zerox-dg/vi-rs)).

The following diagram explains how `bogo-rs` communicates with other components like OS's input source and `vi-rs`:

```
INPUT LAYER
+------------------+            FRONTEND               ENGINE
| macOS            | [d,a,a,y]  +---------+ "daay"     +-------+
|  +- CGEventTap   | ---------> | bogo-rs | ---------> | vi-rs |
|                  |            +---------+            +-------+
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

The result string will be sent back to `bogo-rs`, and from here, we will perform an edit on the target application. The edit
is done using [the BACKSPACE technique](https://notes.huy.rocks/posts/go-tieng-viet-linux.html#k%C4%A9-thu%E1%BA%ADt-backspace). It's
unreliable but it has the benefit of not having the pre-edit line, worth it.

To get yourself familiar with IME, there are some good article on the topic:

- [Vietnamese Keyboard Engine with Prolog](https://followthe.trailing.space/To-the-Root-of-the-Tree-dc170bf0e8de44a6b812ca3e01025236?p=0dd31fe76ebd45dca5b4466c9441fa1c&pm=s), lewtds
- [Ước mơ bộ gõ kiểu Unikey trên Linux](https://followthe.trailing.space/To-the-Root-of-the-Tree-dc170bf0e8de44a6b812ca3e01025236?p=9b12cc2fcdbe43149b10eefc7db6b161&pm=s), lewtds
- [Vấn đề về IME trên Linux](https://viethung.space/blog/2020/07/21/Van-de-ve-IME-tren-Linux/), zerox-dg
- [Bỏ dấu trong tiếng Việt](https://viethung.space/blog/2020/07/14/Bo-dau-trong-tieng-Viet/), zerox-dg
- [Chuyện gõ tiếng Việt trên Linux](https://notes.huy.rocks/posts/go-tieng-viet-linux.html), huytd



