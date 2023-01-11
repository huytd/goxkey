mod ffi;

use std::{sync::mpsc::channel, thread};
use ffi::{register_event_tap, EventType};
use crate::ffi::{CFRunLoopRun, send_string, send_backspace, KEY_A, KEY_D, KEY_F};

fn main() {
    let (tx, rx) = channel();

    thread::spawn(move || {
        register_event_tap(&tx);
        println!("KEL is running...");
        CFRunLoopRun();
    });

    let mut queue: Vec<u16> = vec![];

    // safety net: terminate the app if more than 100 events sent
    let mut event_count = 0;

    while event_count < 100 {
        let event = rx.recv().unwrap();
        match event.etype {
            EventType::KeyDown => {
                event_count += 1;

                if event.code == 0x31 {
                    queue.clear();
                } else if event.code == 0x33 {
                    queue.pop();
                } else {
                    queue.push(event.code);

                    if queue.len() >= 2 {
                        if &queue[queue.len() - 2..] == &[KEY_A, KEY_A] {
                            send_backspace();
                            send_backspace();
                            send_string("â");
                        }
                        if &queue[queue.len() - 2..] == &[KEY_D, KEY_D] {
                            send_backspace();
                            send_backspace();
                            send_string("đ");
                        }
                        if &queue[queue.len() - 2..] == &[KEY_A, KEY_F] {
                            send_backspace();
                            send_backspace();
                            send_string("à");
                        }
                    }
                }
                println!("{:?}", queue);
            },
            _ => {}
        }
    }
}
