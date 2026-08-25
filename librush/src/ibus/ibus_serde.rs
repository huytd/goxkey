use std::collections::HashMap;

use arbitrary_int::u11;
use bitbybit::bitfield;
use zbus::zvariant::{Structure, Value};

// 源文件: `ibus/src/ibustext.c`
// IBusText
//
// variant  struct {
//   string "IBusText"
//   array []
//   string "测试"
//   variant  struct {
//     string "IBusAttrList"
//     array []
//     array []
//   }
// }
//
// (
//   <(
//     'IBusText',
//     @a{sv} {},
//     '测试',
//     <(
//       'IBusAttrList',
//       @a{sv} {},
//       @av []
//     )>
//   )>,
// )

/// `IBusText`: serialize data as ibus format
pub fn make_ibus_text(text: String) -> Value<'static> {
    // 构造内部的 variant  struct
    // (sa{sv}av)
    let st1 = Structure::from((
        "IBusAttrList",
        HashMap::<String, Value<'static>>::new(),
        Vec::<Value<'static>>::new(),
    ));

    // 构造外部的 variant  struct
    // (sa{sv}sv)
    let st2 = Structure::from((
        "IBusText",
        HashMap::<String, Value<'static>>::new(),
        text,
        Value::new(st1),
    ));

    Value::new(st2)
}

// 源文件: `ibus/src/ibustypes.h`
#[bitfield(u32)]
pub struct IBusModifierState {
    /// `IBUS_SHIFT_MASK`: Shift  is activated.
    #[bit(0, rw)]
    shift: bool,
    /// `IBUS_LOCK_MASK`: Cap Lock is locked.
    #[bit(1, rw)]
    lock: bool,
    /// `IBUS_CONTROL_MASK`: Control key is activated.
    #[bit(2, rw)]
    control: bool,
    /// `IBUS_MOD1_MASK`: Modifier 1 (Usually Alt_L (0x40),  Alt_R (0x6c),  Meta_L (0xcd)) activated.
    #[bit(3, rw)]
    mod1: bool,
    /// `IBUS_MOD2_MASK`: Modifier 2 (Usually Num_Lock (0x4d)) activated.
    #[bit(4, rw)]
    mod2: bool,
    /// `IBUS_MOD3_MASK`: Modifier 3 activated.
    #[bit(5, rw)]
    mod3: bool,
    /// `IBUS_MOD4_MASK`: Modifier 4 (Usually Super_L (0xce),  Hyper_L (0xcf)) activated.
    #[bit(6, rw)]
    mod4: bool,
    /// `IBUS_MOD5_MASK`: Modifier 5 (ISO_Level3_Shift (0x5c),  Mode_switch (0xcb)) activated.
    #[bit(7, rw)]
    mod5: bool,
    /// `IBUS_BUTTON1_MASK`: Mouse button 1 (left) is activated.
    #[bit(8, rw)]
    button1: bool,
    /// `IBUS_BUTTON2_MASK`: Mouse button 2 (middle) is activated.
    #[bit(9, rw)]
    button2: bool,
    /// `IBUS_BUTTON3_MASK`: Mouse button 3 (right) is activated.
    #[bit(10, rw)]
    button3: bool,
    /// `IBUS_BUTTON4_MASK`: Mouse button 4 (scroll up) is activated.
    #[bit(11, rw)]
    button4: bool,
    /// `IBUS_BUTTON5_MASK`: Mouse button 5 (scroll down) is activated.
    #[bit(12, rw)]
    button5: bool,
    #[bits(13..=23, rw)]
    unused1: u11,
    /// `IBUS_HANDLED_MASK`: Handled mask indicates the event has been handled by ibus.
    #[bit(24, rw)]
    handled: bool,
    /// `IBUS_FORWARD_MASK`: Forward mask indicates the event has been forward from ibus.
    #[bit(25, rw)]
    forward: bool,
    /// `IBUS_SUPER_MASK`: Super (Usually Win) key is activated.
    #[bit(26, rw)]
    super_: bool,
    /// `IBUS_HYPER_MASK`: Hyper key is activated.
    #[bit(27, rw)]
    hyper: bool,
    /// `IBUS_META_MASK`: Meta key is activated.
    #[bit(28, rw)]
    meta: bool,
    #[bit(29, rw)]
    unused2: bool,
    /// `IBUS_RELEASE_MASK`: Key is released.
    #[bit(30, rw)]
    release: bool,
    #[bit(31, rw)]
    unused3: bool,
}

impl IBusModifierState {
    /// True when modifiers are pressed which indicate that this keypress is keybinding and would
    /// typically not be used to type text.
    ///
    /// Modifiers considered:
    /// - control
    /// - mod1
    /// - mod4
    /// - super
    /// - hyper
    pub fn has_special_modifiers(self) -> bool {
        self.control() || self.mod1() || self.mod4() || self.super_() || self.meta() || self.hyper()
    }

    // True when this keyboard event is caused by releasing a key, not pressing it
    pub fn is_keyup(self) -> bool {
        self.release()
    }

    // True when this keyboard event is caused by pressing a key, not releasing it
    pub fn is_keydown(self) -> bool {
        !self.is_keyup()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn ibus_text_zvariant_signature() {
        let v = make_ibus_text("test".into());
        assert_eq!(v.value_signature(), "(sa{sv}sv)");
    }
}
