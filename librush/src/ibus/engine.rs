//! `ibus_engine` 表示一个输入法实现
use std::error::Error;
use std::future::Future;
use std::marker::Send;
use std::sync::atomic::{AtomicU64, Ordering};

use pm_bin::log::info;
use xkeysym::{KeyCode, Keysym};
use zbus::{
    Connection, ObjectServer, fdo, interface, object_server::SignalEmitter, zvariant::Value,
};

use super::{IBusModifierState, LookupTable, ibus_serde::make_ibus_text};

/// Implement this trait to implement an input method.
///
/// Your implementation can use the methods of the [`IBusEngineBackend`]
/// to display text to the user.
pub trait IBusEngine: Send + Sync {
    /// A key was pressed or released.
    ///
    /// `keyval` encodes the symbol of the key interpreted according to the current keyboard layout.
    ///
    /// `keycode` encodes the position of the key on the keyboard, which is independent of the
    /// keyboard layout.
    ///
    /// state encodes wether the key was pressed or released, and modifiers (shift, control...).
    ///
    /// Note that when `shift+a` is pressed, `keyval` will be `Keysym::A` (instead of `Keysym::a`).
    /// `state.shift()` will still be `true`. Same applies for `AltGr` in keyboard layouts which
    /// have it.
    fn process_key_event(
        &mut self,
        _se: SignalEmitter<'_>,
        _server: &ObjectServer,
        _keyval: Keysym,
        _keycode: KeyCode,
        _state: IBusModifierState,
    ) -> impl Future<Output = fdo::Result<bool>> + Send {
        async { Ok(false) }
    }

    /// 设置光标周围文本和位置
    fn set_surrounding_text(
        &mut self,
        _se: SignalEmitter<'_>,
        _server: &ObjectServer,
        _text: Value<'_>,
        _cursor_pos: u32,
        _anchor_pos: u32,
    ) -> impl Future<Output = fdo::Result<()>> + Send {
        async { Ok(()) }
    }

    /// 设置光标位置
    fn set_cursor_location(
        &mut self,
        _se: SignalEmitter<'_>,
        _server: &ObjectServer,
        _x: i32,
        _y: i32,
        _w: i32,
        _h: i32,
    ) -> impl Future<Output = fdo::Result<()>> + Send {
        async { Ok(()) }
    }

    /// 获得焦点
    fn focus_in(
        &mut self,
        _se: SignalEmitter<'_>,
        _server: &ObjectServer,
    ) -> impl Future<Output = fdo::Result<()>> + Send {
        async { Ok(()) }
    }

    /// 失去焦点
    fn focus_out(
        &mut self,
        _se: SignalEmitter<'_>,
        _server: &ObjectServer,
    ) -> impl Future<Output = fdo::Result<()>> + Send {
        async { Ok(()) }
    }

    /// 重置
    fn reset(
        &mut self,
        _se: SignalEmitter<'_>,
        _server: &ObjectServer,
    ) -> impl Future<Output = fdo::Result<()>> + Send {
        async { Ok(()) }
    }

    /// 启用
    fn enable(
        &mut self,
        _se: SignalEmitter<'_>,
        _server: &ObjectServer,
    ) -> impl Future<Output = fdo::Result<()>> + Send {
        async { Ok(()) }
    }

    /// 禁用
    fn disable(
        &mut self,
        _se: SignalEmitter<'_>,
        _server: &ObjectServer,
    ) -> impl Future<Output = fdo::Result<()>> + Send {
        async { Ok(()) }
    }

    /// (UI) Emitted when a candidate on a lookup table is clicked
    ///
    /// _index is the 0-based index of the clicked candiate *in the current page*, not in the full
    /// lookup table
    fn candidate_clicked(
        &mut self,
        _se: SignalEmitter<'_>,
        _server: &ObjectServer,
        _index: u32,
        _button: u32,
        _state: u32,
    ) -> impl Future<Output = fdo::Result<()>> + Send {
        async { Ok(()) }
    }

    /// (UI) Emitted when the "previous page" button is clicked on a lookup table
    fn page_up(
        &mut self,
        _se: SignalEmitter<'_>,
        _server: &ObjectServer,
    ) -> impl Future<Output = fdo::Result<()>> + Send {
        async { Ok(()) }
    }

    /// (UI) Emitted when the "next page" button is clicked on a lookup table
    fn page_down(
        &mut self,
        _se: SignalEmitter<'_>,
        _server: &ObjectServer,
    ) -> impl Future<Output = fdo::Result<()>> + Send {
        async { Ok(()) }
    }

    /// (UI) Emmitted when the user scrolls up (with the mouse wheel) on a lookup table
    fn cursor_up(
        &mut self,
        _se: SignalEmitter<'_>,
        _server: &ObjectServer,
    ) -> impl Future<Output = fdo::Result<()>> + Send {
        async { Ok(()) }
    }

    /// (UI) Emmitted when the user scrolls down (with the mouse wheel) on a lookup table
    fn cursor_down(
        &mut self,
        _se: SignalEmitter<'_>,
        _server: &ObjectServer,
    ) -> impl Future<Output = fdo::Result<()>> + Send {
        async { Ok(()) }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, PartialOrd, Ord, Eq)]
pub enum IBusPreeditFocusMode {
    Clear,
    Commit,
}

impl From<IBusPreeditFocusMode> for u32 {
    fn from(value: IBusPreeditFocusMode) -> Self {
        match value {
            IBusPreeditFocusMode::Clear => 0,
            IBusPreeditFocusMode::Commit => 1,
        }
    }
}

/// Methods that the IBus daemon provides for inputs methods to use
pub trait IBusEngineBackend: IBusEngine + 'static {
    /// Type this text on behalf of the user
    fn commit_text(
        se: &SignalEmitter<'_>,
        text: String,
    ) -> impl std::future::Future<Output = zbus::Result<()>> + Send;

    /// Forward a keyboard event to the client application
    fn forward_key_event(
        se: &SignalEmitter<'_>,
        keyval: u32,
        keycode: u32,
        state: u32,
    ) -> impl std::future::Future<Output = zbus::Result<()>> + Send;

    /// Ask the client to delete surrounding text
    fn delete_surrounding_text(
        se: &SignalEmitter<'_>,
        offset_from_cursor: i32,
        nchars: u32,
    ) -> impl std::future::Future<Output = zbus::Result<()>> + Send;

    /// (UI) Show of hide this lookup table
    fn update_lookup_table(
        se: &SignalEmitter<'_>,
        table: &LookupTable,
        visible: bool,
    ) -> impl std::future::Future<Output = zbus::Result<()>> + Send;

    /// (UI) Sets the preedit text.
    ///
    /// The preedit text is a piece of text displayed in the place where text is to be written, but
    /// not written yet, in the sense that the underlying application is not aware of it.
    ///
    /// cursor_pos is a value from 0 to `text.len()` indicating where the cursor should be
    /// displayed.
    fn update_preedit_text(
        se: &SignalEmitter<'_>,
        text: String,
        cursor_pos: u32,
        visible: bool,
        mode: IBusPreeditFocusMode,
    ) -> impl std::future::Future<Output = zbus::Result<()>> + Send;

    /// (UI) Sets the auxiliary text
    ///
    /// The auxiliary text is a text shown in a floating textbox besides the place where text is
    /// to be written.
    fn update_auxiliary_text(
        se: &SignalEmitter<'_>,
        text: String,
        visible: bool,
    ) -> impl std::future::Future<Output = zbus::Result<()>> + Send;
}

impl<T: IBusEngine + 'static> IBusEngineBackend for T {
    async fn commit_text(se: &SignalEmitter<'_>, text: String) -> zbus::Result<()> {
        Engine::<Self>::commit_text(se, make_ibus_text(text)).await
    }

    async fn forward_key_event(
        se: &SignalEmitter<'_>,
        keyval: u32,
        keycode: u32,
        state: u32,
    ) -> zbus::Result<()> {
        Engine::<Self>::forward_key_event(se, keyval, keycode, state).await
    }

    async fn delete_surrounding_text(
        se: &SignalEmitter<'_>,
        offset_from_cursor: i32,
        nchars: u32,
    ) -> zbus::Result<()> {
        Engine::<Self>::delete_surrounding_text(se, offset_from_cursor, nchars).await
    }

    async fn update_lookup_table(
        se: &SignalEmitter<'_>,
        table: &LookupTable,
        visible: bool,
    ) -> zbus::Result<()> {
        Engine::<Self>::update_lookup_table(se, table.serialize(), visible).await
    }

    async fn update_preedit_text(
        se: &SignalEmitter<'_>,
        text: String,
        cursor_pos: u32,
        visible: bool,
        mode: IBusPreeditFocusMode,
    ) -> zbus::Result<()> {
        Engine::<Self>::update_preedit_text(
            se,
            make_ibus_text(text),
            cursor_pos,
            visible,
            mode.into(),
        )
        .await
    }

    async fn update_auxiliary_text(
        se: &SignalEmitter<'_>,
        text: String,
        visible: bool,
    ) -> zbus::Result<()> {
        Engine::<Self>::update_auxiliary_text(se, make_ibus_text(text), visible).await
    }
}

/// D-Bus interface: `org.freedesktop.IBus.Engine`
///
/// <https://ibus.github.io/docs/ibus-1.5/IBusEngine.html>
#[derive(Debug, Clone)]
pub(crate) struct Engine<T: IBusEngine + 'static> {
    e: T,

    _op: String,
}

// 源文件: `ibus/src/ibusengine.c`
//
// <node>
//   <interface name='org.freedesktop.IBus.Engine'>
//
//     <method name='ProcessKeyEvent'>
//       <arg direction='in'  type='u' name='keyval' />
//       <arg direction='in'  type='u' name='keycode' />
//       <arg direction='in'  type='u' name='state' />
//       <arg direction='out' type='b' />
//     </method>
//     <method name='SetCursorLocation'>
//       <arg direction='in'  type='i' name='x' />
//       <arg direction='in'  type='i' name='y' />
//       <arg direction='in'  type='i' name='w' />
//       <arg direction='in'  type='i' name='h' />
//     </method>
//     <method name='ProcessHandWritingEvent'>
//       <arg direction='in'  type='ad' name='coordinates' />
//     </method>
//     <method name='CancelHandWriting'>
//       <arg direction='in'  type='u' name='n_strokes' />
//     </method>
//     <method name='SetCapabilities'>
//       <arg direction='in'  type='u' name='caps' />
//     </method>
//     <method name='PropertyActivate'>
//       <arg direction='in'  type='s' name='name' />
//       <arg direction='in'  type='u' name='state' />
//     </method>
//     <method name='PropertyShow'>
//       <arg direction='in'  type='s' name='name' />
//     </method>
//     <method name='PropertyHide'>
//       <arg direction='in'  type='s' name='name' />
//     </method>
//     <method name='CandidateClicked'>
//       <arg direction='in'  type='u' name='index' />
//       <arg direction='in'  type='u' name='button' />
//       <arg direction='in'  type='u' name='state' />
//     </method>
//     <method name='FocusIn' />
//     <method name='FocusInId'>
//       <arg direction='in'  type='s' name='object_path' />
//       <arg direction='in'  type='s' name='client' />
//     </method>
//     <method name='FocusIn' />
//     <method name='FocusOut' />
//     <method name='FocusOutId'>
//       <arg direction='in'  type='s' name='object_path' />
//     </method>
//     <method name='Reset' />
//     <method name='Enable' />
//     <method name='Disable' />
//     <method name='PageUp' />
//     <method name='PageDown' />
//     <method name='CursorUp' />
//     <method name='CursorDown' />
//     <method name='SetSurroundingText'>
//       <arg direction='in'  type='v' name='text' />
//       <arg direction='in'  type='u' name='cursor_pos' />
//       <arg direction='in'  type='u' name='anchor_pos' />
//     </method>
//     <method name='PanelExtensionReceived'>
//       <arg direction='in'  type='v' name='event' />
//     </method>
//     <method name='PanelExtensionRegisterKeys'>
//       <arg direction='in'  type='v' name='data' />
//     </method>
//
//     <signal name='CommitText'>
//       <arg type='v' name='text' />
//     </signal>
//     <signal name='UpdatePreeditText'>
//       <arg type='v' name='text' />
//       <arg type='u' name='cursor_pos' />
//       <arg type='b' name='visible' />
//       <arg type='u' name='mode' />
//     </signal>
//     <signal name='UpdateAuxiliaryText'>
//       <arg type='v' name='text' />
//       <arg type='b' name='visible' />
//     </signal>
//     <signal name='UpdateLookupTable'>
//       <arg type='v' name='table' />
//       <arg type='b' name='visible' />
//     </signal>
//     <signal name='RegisterProperties'>
//       <arg type='v' name='props' />
//     </signal>
//     <signal name='UpdateProperty'>
//       <arg type='v' name='prop' />
//     </signal>
//     <signal name='ForwardKeyEvent'>
//       <arg type='u' name='keyval' />
//       <arg type='u' name='keycode' />
//       <arg type='u' name='state' />
//     </signal>
//     <signal name='PanelExtension'>
//       <arg type='v' name='data' />
//     </signal>
//
//     <property name='ContentType' type='(uu)' access='write' />
//     <property name='FocusId' type='(b)' access='read' />
//     <property name='ActiveSurroundingText' type='(b)' access='read' />
//   </interface>
// </node>
#[interface(name = "org.freedesktop.IBus.Engine")]
impl<T: IBusEngine + 'static> Engine<T> {
    async fn process_key_event(
        &mut self,
        #[zbus(signal_emitter)] se: SignalEmitter<'_>,
        #[zbus(object_server)] server: &ObjectServer,
        keyval: u32,
        keycode: u32,
        state: u32,
    ) -> fdo::Result<bool> {
        // Note: ibuskeysyms-update.pl indicates that IBUS_KEY_* constants are the same as XK_
        // constants provided by xkeysym
        self.e
            .process_key_event(
                se,
                server,
                keyval.into(),
                keycode.into(),
                IBusModifierState::new_with_raw_value(state),
            )
            .await
    }

    async fn set_cursor_location(
        &mut self,
        #[zbus(signal_emitter)] se: SignalEmitter<'_>,
        #[zbus(object_server)] server: &ObjectServer,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
    ) -> fdo::Result<()> {
        self.e.set_cursor_location(se, server, x, y, w, h).await
    }

    // 忽略
    fn process_hand_writing_event(&mut self, _coordinates: Vec<f64>) -> fdo::Result<()> {
        Ok(())
    }

    // 忽略
    fn cancel_hand_writing(&mut self, _n_strokes: u32) -> fdo::Result<()> {
        Ok(())
    }

    // 忽略
    fn set_capabilities(&mut self, _caps: u32) -> fdo::Result<()> {
        Ok(())
    }

    // 忽略 (用户界面相关)
    fn property_activate(&mut self, _name: String, _state: u32) -> fdo::Result<()> {
        Ok(())
    }

    // 忽略 (用户界面相关)
    fn property_show(&mut self, _name: String) -> fdo::Result<()> {
        Ok(())
    }

    // 忽略 (用户界面相关)
    fn property_hide(&mut self, _name: String) -> fdo::Result<()> {
        Ok(())
    }

    // (UI)
    async fn candidate_clicked(
        &mut self,
        #[zbus(signal_emitter)] se: SignalEmitter<'_>,
        #[zbus(object_server)] server: &ObjectServer,
        index: u32,
        button: u32,
        state: u32,
    ) -> fdo::Result<()> {
        self.e
            .candidate_clicked(se, server, index, button, state)
            .await
    }

    async fn focus_in(
        &mut self,
        #[zbus(signal_emitter)] se: SignalEmitter<'_>,
        #[zbus(object_server)] server: &ObjectServer,
    ) -> fdo::Result<()> {
        self.e.focus_in(se, server).await
    }

    fn focus_in_id(&mut self, _object_path: String, _client: String) -> fdo::Result<()> {
        // TODO
        Ok(())
    }

    async fn focus_out(
        &mut self,
        #[zbus(signal_emitter)] se: SignalEmitter<'_>,
        #[zbus(object_server)] server: &ObjectServer,
    ) -> fdo::Result<()> {
        self.e.focus_out(se, server).await
    }

    fn focus_out_id(&mut self, _object_path: String) -> fdo::Result<()> {
        // TODO
        Ok(())
    }

    async fn reset(
        &mut self,
        #[zbus(signal_emitter)] se: SignalEmitter<'_>,
        #[zbus(object_server)] server: &ObjectServer,
    ) -> fdo::Result<()> {
        self.e.reset(se, server).await
    }

    async fn enable(
        &mut self,
        #[zbus(signal_emitter)] se: SignalEmitter<'_>,
        #[zbus(object_server)] server: &ObjectServer,
    ) -> fdo::Result<()> {
        self.e.enable(se, server).await
    }

    async fn disable(
        &mut self,
        #[zbus(signal_emitter)] se: SignalEmitter<'_>,
        #[zbus(object_server)] server: &ObjectServer,
    ) -> fdo::Result<()> {
        self.e.disable(se, server).await
    }

    /// (UI) Emitted when the page-up button is pressed.
    async fn page_up(
        &mut self,
        #[zbus(signal_emitter)] se: SignalEmitter<'_>,
        #[zbus(object_server)] server: &ObjectServer,
    ) -> fdo::Result<()> {
        self.e.page_up(se, server).await
    }

    /// (UI) Emitted when the page-down button is pressed
    async fn page_down(
        &mut self,
        #[zbus(signal_emitter)] se: SignalEmitter<'_>,
        #[zbus(object_server)] server: &ObjectServer,
    ) -> fdo::Result<()> {
        self.e.page_down(se, server).await
    }

    /// (UI) Emitted when the up cursor button is pressed.
    async fn cursor_up(
        &mut self,
        #[zbus(signal_emitter)] se: SignalEmitter<'_>,
        #[zbus(object_server)] server: &ObjectServer,
    ) -> fdo::Result<()> {
        self.e.cursor_up(se, server).await
    }

    /// (UI) Emitted when the down cursor button is pressed
    async fn cursor_down(
        &mut self,
        #[zbus(signal_emitter)] se: SignalEmitter<'_>,
        #[zbus(object_server)] server: &ObjectServer,
    ) -> fdo::Result<()> {
        self.e.cursor_down(se, server).await
    }

    async fn set_surrounding_text(
        &mut self,
        #[zbus(signal_emitter)] se: SignalEmitter<'_>,
        #[zbus(object_server)] server: &ObjectServer,
        text: Value<'_>,
        cursor_pos: u32,
        anchor_pos: u32,
    ) -> fdo::Result<()> {
        self.e
            .set_surrounding_text(se, server, text, cursor_pos, anchor_pos)
            .await
    }

    // 忽略 (用户界面相关)
    fn panel_extension_received(&mut self, _event: Value) -> fdo::Result<()> {
        Ok(())
    }

    // 忽略 (用户界面相关)
    fn panel_extension_register_keys(&mut self, _data: Value) -> fdo::Result<()> {
        Ok(())
    }

    #[zbus(signal)]
    async fn commit_text(se: &SignalEmitter<'_>, text: Value<'_>) -> zbus::Result<()>;

    // (UI)
    #[zbus(signal)]
    async fn update_preedit_text(
        se: &SignalEmitter<'_>,
        text: Value<'_>,
        cursor_pos: u32,
        visible: bool,
        mode: u32,
    ) -> zbus::Result<()>;

    // (UI)
    #[zbus(signal)]
    async fn update_auxiliary_text(
        se: &SignalEmitter<'_>,
        text: Value<'_>,
        visible: bool,
    ) -> zbus::Result<()>;

    // (UI)
    #[zbus(signal)]
    async fn update_lookup_table(
        se: &SignalEmitter<'_>,
        table: Value<'_>,
        visible: bool,
    ) -> zbus::Result<()>;

    // 忽略 (用户界面相关)
    #[zbus(signal)]
    async fn register_properties(se: &SignalEmitter<'_>, props: Value<'_>) -> zbus::Result<()>;

    // 忽略 (用户界面相关)
    #[zbus(signal)]
    async fn update_property(se: &SignalEmitter<'_>, prop: Value<'_>) -> zbus::Result<()>;

    #[zbus(signal)]
    pub async fn forward_key_event(
        se: &SignalEmitter<'_>,
        keyval: u32,
        keycode: u32,
        state: u32,
    ) -> zbus::Result<()>;

    /// Ask the client to delete surrounding text.
    ///
    /// `offset_from_cursor` is the distance from the cursor (negative = before cursor),
    /// `nchars` is how many characters to delete.
    #[zbus(signal)]
    pub async fn delete_surrounding_text(
        se: &SignalEmitter<'_>,
        offset_from_cursor: i32,
        nchars: u32,
    ) -> zbus::Result<()>;

    // 忽略 (用户界面相关)
    #[zbus(signal)]
    async fn panel_extension(se: &SignalEmitter<'_>, data: Value<'_>) -> zbus::Result<()>;

    #[zbus(property)]
    fn content_type(&self) -> (u32, u32) {
        // TODO
        (0, 0)
    }

    #[zbus(property)]
    fn set_content_type(&mut self, _t: (u32, u32)) -> fdo::Result<()> {
        // TODO
        Ok(())
    }

    #[zbus(property)]
    fn focus_id(&self) -> bool {
        // TODO
        false
    }

    #[zbus(property)]
    fn active_surrounding_text(&self) -> bool {
        true
    }
}

impl<T: IBusEngine + 'static> Engine<T> {
    /// create engine (include ibus init)
    pub async fn new(c: &Connection, e: T) -> Result<String, Box<dyn Error>> {
        // 源文件: `ibus/src/ibusfactory.c`
        // 函数: `ibus_factory_real_create_engine()`
        let id = ENGINE_ID.fetch_add(1, Ordering::SeqCst);
        let object_path = format!("/org/freedesktop/IBus/Engine/{}", id);

        let o = Engine {
            e,
            _op: object_path.clone(),
        };

        c.object_server().at(object_path.clone(), o).await?;

        info!("创建 engine 成功: {}", object_path);
        Ok(object_path)
    }
}

static ENGINE_ID: AtomicU64 = AtomicU64::new(1);
