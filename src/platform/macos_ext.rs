use cocoa::appkit::{
    NSApp, NSApplication, NSButton, NSMenu, NSMenuItem, NSStatusBar, NSStatusItem,
};
use cocoa::base::{id, nil, BOOL, NO, YES};
use cocoa::foundation::{NSAutoreleasePool, NSString};
use core_foundation::dictionary::CFDictionaryRef;
use core_foundation::string::CFStringRef;
use core_graphics::{
    event::{CGEventTapProxy, CGKeyCode},
    sys,
};
use druid::{Data, Lens};
use libc::c_void;
use objc::{
    class,
    declare::ClassDecl,
    msg_send,
    runtime::{Class, Object, Sel},
    sel, sel_impl, Message,
};
use objc_foundation::{INSObject, NSObject};
use objc_id::Id;
use once_cell::sync::OnceCell;
use std::mem;

/// Global reference to the NSStatusItem pointer so we can update the tray
/// title directly from any thread (via dispatch_async to the main queue),
/// bypassing Druid's event loop which can lag ~1 s when the window is hidden.
static SYSTRAY_ITEM: OnceCell<usize> = OnceCell::new();

#[derive(Clone, PartialEq, Eq)]
struct Wrapper(*mut objc::runtime::Object);
impl Data for Wrapper {
    fn same(&self, _other: &Self) -> bool {
        true
    }
}

pub enum SystemTrayMenuItemKey {
    ShowUI,
    Enable,
    TypingMethodTelex,
    TypingMethodVNI,
    TypingMethodTelexVNI,
    Exit,
}

#[derive(Clone, Data, Lens, PartialEq, Eq)]
pub struct SystemTray {
    _pool: Wrapper,
    menu: Wrapper,
    item: Wrapper,
}

impl SystemTray {
    pub fn new() -> Self {
        unsafe {
            let pool = NSAutoreleasePool::new(nil);
            let menu = NSMenu::new(nil).autorelease();

            let app = NSApp();
            app.activateIgnoringOtherApps_(YES);
            let item = NSStatusBar::systemStatusBar(nil).statusItemWithLength_(-1.0);
            let button: id = msg_send![item, button];
            let image = create_badge_image("VN", true);
            let _: () = msg_send![button, setImage: image];
            item.setMenu_(menu);

            // Store the raw pointer globally so dispatch_set_systray_title
            // can update the title without going through Druid's event loop.
            let _ = SYSTRAY_ITEM.set(item as usize);

            let s = Self {
                _pool: Wrapper(pool),
                menu: Wrapper(menu),
                item: Wrapper(item),
            };
            s.init_menu_items();
            s
        }
    }

    pub fn set_title(&mut self, title: &str, is_vietnamese: bool) {
        unsafe {
            let button: id = msg_send![self.item.0, button];
            let image = create_badge_image(title, is_vietnamese);
            let _: () = msg_send![button, setImage: image];
            let empty = NSString::alloc(nil).init_str("");
            let _: () = msg_send![button, setTitle: empty];
            let _: () = msg_send![empty, release];
        }
    }

    pub fn init_menu_items(&self) {
        use crate::ui::locale::t;
        self.add_menu_item(t("menu.open_panel"), || ());
        self.add_menu_separator();
        self.add_menu_item(t("menu.disable_vietnamese"), || ());
        self.add_menu_separator();
        self.add_menu_item("Telex ✓", || ());
        self.add_menu_item("VNI", || ());
        self.add_menu_item("Telex+VNI", || ());
        self.add_menu_separator();
        self.add_menu_item(t("menu.quit"), || ());
    }

    pub fn add_menu_separator(&self) {
        unsafe {
            NSMenu::addItem_(self.menu.0, NSMenuItem::separatorItem(nil));
        }
    }

    pub fn add_menu_item<F>(&self, label: &str, cb: F)
    where
        F: Fn() + Send + 'static,
    {
        let cb_obj = Callback::from(Box::new(cb));

        unsafe {
            let no_key = NSString::alloc(nil).init_str("");
            let itemtitle = NSString::alloc(nil).init_str(label);
            let action = sel!(call);
            let item = NSMenuItem::alloc(nil)
                .initWithTitle_action_keyEquivalent_(itemtitle, action, no_key);
            let _: () = msg_send![item, setTarget: cb_obj];

            NSMenu::addItem_(self.menu.0, item);
        }
    }

    pub fn get_menu_item_index_by_key(&self, key: SystemTrayMenuItemKey) -> i64 {
        match key {
            SystemTrayMenuItemKey::ShowUI => 0,
            SystemTrayMenuItemKey::Enable => 2,
            SystemTrayMenuItemKey::TypingMethodTelex => 4,
            SystemTrayMenuItemKey::TypingMethodVNI => 5,
            SystemTrayMenuItemKey::TypingMethodTelexVNI => 6,
            SystemTrayMenuItemKey::Exit => 8,
        }
    }

    pub fn set_menu_item_title(&self, key: SystemTrayMenuItemKey, label: &str) {
        unsafe {
            let item_title = NSString::alloc(nil).init_str(label);
            let index = self.get_menu_item_index_by_key(key);
            let menu_item: id = msg_send![self.menu.0, itemAtIndex: index];
            let _: () = msg_send![menu_item, setTitle: item_title];
            let _: () = msg_send![item_title, release];
        }
    }

    pub fn set_menu_item_callback<F>(&self, key: SystemTrayMenuItemKey, cb: F)
    where
        F: Fn() + Send + 'static,
    {
        let cb_obj = Callback::from(Box::new(cb));
        unsafe {
            let index = self.get_menu_item_index_by_key(key);
            let _: () = msg_send![self.menu.0.itemAtIndex_(index), setTarget: cb_obj];
        }
    }
}

/// Create an NSImage with a colored badge-style rounded rectangle and centered text.
/// `is_vietnamese` selects the color: green for Vietnamese, blue for English.
unsafe fn create_badge_image(title: &str, is_vietnamese: bool) -> id {
    use cocoa::foundation::{NSPoint, NSRect, NSSize};

    let (r, g, b) = if is_vietnamese {
        (26.0 / 255.0, 138.0 / 255.0, 110.0 / 255.0) // green
    } else {
        (58.0 / 255.0, 115.0 / 255.0, 199.0 / 255.0) // blue
    };
    let badge_color: id = msg_send![class!(NSColor), colorWithSRGBRed:r green:g blue:b alpha:1.0_f64];

    // Measure text to determine badge width
    let font: id = msg_send![class!(NSFont), systemFontOfSize: 9.5_f64 weight: 0.4_f64];
    let title_ns = NSString::alloc(nil).init_str(title);

    // Create attributed string with badge color for the text
    let font_key = NSString::alloc(nil).init_str("NSFont");
    let color_key = NSString::alloc(nil).init_str("NSColor");
    let keys: [id; 2] = [font_key, color_key];
    let vals: [id; 2] = [font, badge_color];
    let attrs: id = msg_send![class!(NSDictionary), dictionaryWithObjects:vals.as_ptr() forKeys:keys.as_ptr() count:2_u64];
    let attr_str: id = msg_send![class!(NSAttributedString), alloc];
    let attr_str: id = msg_send![attr_str, initWithString:title_ns attributes:attrs];
    let text_size: NSSize = msg_send![attr_str, size];

    let padding_h = 6.0_f64;
    let padding_v = 3.5_f64;
    let badge_w = (text_size.width + padding_h * 2.0).ceil();
    let badge_h = (text_size.height + padding_v * 2.0).ceil();
    let corner_radius = 4.0_f64;
    let border_width = 1.2_f64;

    let img_size = NSSize::new(badge_w, badge_h);
    let image: id = msg_send![class!(NSImage), alloc];
    let image: id = msg_send![image, initWithSize: img_size];

    let _: () = msg_send![image, lockFocus];

    // Draw rounded rect border in badge color
    let inset = border_width / 2.0;
    let rect = NSRect::new(
        NSPoint::new(inset, inset),
        NSSize::new(badge_w - border_width, badge_h - border_width),
    );
    let path: id = msg_send![class!(NSBezierPath), bezierPathWithRoundedRect:rect xRadius:corner_radius yRadius:corner_radius];
    let _: () = msg_send![badge_color, setStroke];
    let _: () = msg_send![path, setLineWidth: border_width];
    let _: () = msg_send![path, stroke];

    // Draw centered text
    let text_x = (badge_w - text_size.width) / 2.0;
    let text_y = (badge_h - text_size.height) / 2.0;
    let _: () = msg_send![attr_str, drawAtPoint: NSPoint::new(text_x, text_y)];

    let _: () = msg_send![image, unlockFocus];
    let _: () = msg_send![attr_str, release];

    image
}

/// Update the system tray title immediately by dispatching to the main queue.
/// This bypasses Druid's event loop, which can be slow when the window is hidden.
/// Safe to call from any thread.
pub fn dispatch_set_systray_title(title: &str, is_vietnamese: bool) {
    let Some(&item_ptr) = SYSTRAY_ITEM.get() else {
        return;
    };
    let title_owned = title.to_owned();

    struct Context {
        item: usize,
        title: String,
        is_vietnamese: bool,
    }

    unsafe extern "C" fn work(ctx: *mut c_void) {
        let ctx = Box::from_raw(ctx as *mut Context);
        let item = ctx.item as id;
        let button: id = msg_send![item, button];
        let image = create_badge_image(&ctx.title, ctx.is_vietnamese);
        let _: () = msg_send![button, setImage: image];
        let empty = NSString::alloc(nil).init_str("");
        let _: () = msg_send![button, setTitle: empty];
        let _: () = msg_send![empty, release];
    }

    let ctx = Box::new(Context {
        item: item_ptr,
        title: title_owned,
        is_vietnamese: is_vietnamese,
    });
    let ctx_ptr = Box::into_raw(ctx) as *mut c_void;

    unsafe {
        dispatch_async_f(&_dispatch_main_q, ctx_ptr, work);
    }
}

pub type Handle = CGEventTapProxy;

#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
    pub(crate) fn CGEventTapPostEvent(proxy: CGEventTapProxy, event: sys::CGEventRef);
    pub(crate) fn CGEventCreateKeyboardEvent(
        source: sys::CGEventSourceRef,
        keycode: CGKeyCode,
        keydown: bool,
    ) -> sys::CGEventRef;
    pub(crate) fn CGEventKeyboardSetUnicodeString(
        event: sys::CGEventRef,
        length: libc::c_ulong,
        string: *const u16,
    );
}

pub mod new_tap {
    use std::{
        mem::{self, ManuallyDrop},
        ptr,
    };

    use core_foundation::{
        base::TCFType,
        mach_port::{CFMachPort, CFMachPortRef},
    };
    use core_graphics::{
        event::{
            CGEvent, CGEventMask, CGEventTapCallBackFn, CGEventTapLocation, CGEventTapOptions,
            CGEventTapPlacement, CGEventTapProxy, CGEventType,
        },
        sys,
    };
    use foreign_types::ForeignType;
    use libc::c_void;

    type CGEventTapCallBackInternal = unsafe extern "C" fn(
        proxy: CGEventTapProxy,
        etype: CGEventType,
        event: sys::CGEventRef,
        user_info: *const c_void,
    ) -> sys::CGEventRef;

    #[link(name = "CoreGraphics", kind = "framework")]
    extern "C" {
        fn CGEventTapCreate(
            tap: CGEventTapLocation,
            place: CGEventTapPlacement,
            options: CGEventTapOptions,
            eventsOfInterest: CGEventMask,
            callback: CGEventTapCallBackInternal,
            userInfo: *const c_void,
        ) -> CFMachPortRef;
        fn CGEventTapEnable(tap: CFMachPortRef, enable: bool);
    }

    #[no_mangle]
    unsafe extern "C" fn cg_new_tap_callback_internal(
        _proxy: CGEventTapProxy,
        _etype: CGEventType,
        _event: sys::CGEventRef,
        _user_info: *const c_void,
    ) -> sys::CGEventRef {
        let callback = _user_info as *mut CGEventTapCallBackFn;
        let event = CGEvent::from_ptr(_event);
        let new_event = (*callback)(_proxy, _etype, &event);
        match new_event {
            Some(new_event) => ManuallyDrop::new(new_event).as_ptr(),
            None => {
                mem::forget(event);
                ptr::null_mut() as sys::CGEventRef
            }
        }
    }

    /* Generate an event mask for a single type of event. */
    macro_rules! CGEventMaskBit {
        ($eventType:expr) => {
            1 << $eventType as CGEventMask
        };
    }

    type CallbackType<'tap_life> =
        Box<dyn Fn(CGEventTapProxy, CGEventType, &CGEvent) -> Option<CGEvent> + 'tap_life>;
    pub struct CGEventTap<'tap_life> {
        pub mach_port: CFMachPort,
        pub callback_ref: CallbackType<'tap_life>,
    }

    impl<'tap_life> CGEventTap<'tap_life> {
        pub fn new<F: Fn(CGEventTapProxy, CGEventType, &CGEvent) -> Option<CGEvent> + 'tap_life>(
            tap: CGEventTapLocation,
            place: CGEventTapPlacement,
            options: CGEventTapOptions,
            events_of_interest: std::vec::Vec<CGEventType>,
            callback: F,
        ) -> Result<CGEventTap<'tap_life>, ()> {
            let event_mask: CGEventMask = events_of_interest
                .iter()
                .fold(CGEventType::Null as CGEventMask, |mask, &etype| {
                    mask | CGEventMaskBit!(etype)
                });
            let cb = Box::new(Box::new(callback) as CGEventTapCallBackFn);
            let cbr = Box::into_raw(cb);
            unsafe {
                let event_tap_ref = CGEventTapCreate(
                    tap,
                    place,
                    options,
                    event_mask,
                    cg_new_tap_callback_internal,
                    cbr as *const c_void,
                );

                if !event_tap_ref.is_null() {
                    Ok(Self {
                        mach_port: (CFMachPort::wrap_under_create_rule(event_tap_ref)),
                        callback_ref: Box::from_raw(cbr),
                    })
                } else {
                    _ = Box::from_raw(cbr);
                    Err(())
                }
            }
        }

        pub fn enable(&self) {
            unsafe { CGEventTapEnable(self.mach_port.as_concrete_TypeRef(), true) }
        }
    }
}

pub(crate) enum Callback {}
unsafe impl Message for Callback {}

pub(crate) struct CallbackState {
    cb: Box<dyn Fn()>,
}

impl Callback {
    pub(crate) fn from(cb: Box<dyn Fn()>) -> Id<Self> {
        let cbs = CallbackState { cb };
        let bcbs = Box::new(cbs);

        let ptr = Box::into_raw(bcbs);
        let ptr = ptr as *mut c_void as usize;
        let mut oid = <Callback as INSObject>::new();
        (*oid).setptr(ptr);
        oid
    }

    pub(crate) fn setptr(&mut self, uptr: usize) {
        unsafe {
            let obj = &mut *(self as *mut _ as *mut ::objc::runtime::Object);
            obj.set_ivar("_cbptr", uptr);
        }
    }
}

impl INSObject for Callback {
    fn class() -> &'static Class {
        let cname = "Callback";

        let mut klass = Class::get(cname);
        if klass.is_none() {
            let superclass = NSObject::class();
            let mut decl = ClassDecl::new(cname, superclass).unwrap();
            decl.add_ivar::<usize>("_cbptr");

            extern "C" fn sysbar_callback_call(this: &Object, _cmd: Sel) {
                unsafe {
                    let pval: usize = *this.get_ivar("_cbptr");
                    let ptr = pval as *mut c_void;
                    let ptr = ptr as *mut CallbackState;
                    let bcbs: Box<CallbackState> = Box::from_raw(ptr);
                    {
                        (*bcbs.cb)();
                    }
                    mem::forget(bcbs);
                }
            }

            unsafe {
                decl.add_method(
                    sel!(call),
                    sysbar_callback_call as extern "C" fn(&Object, Sel),
                );
            }

            decl.register();
            klass = Class::get(cname);
        }
        klass.unwrap()
    }
}

#[link(name = "ApplicationServices", kind = "framework")]
extern "C" {
    pub fn AXIsProcessTrustedWithOptions(options: CFDictionaryRef) -> bool;
    pub static kAXTrustedCheckOptionPrompt: CFStringRef;
}

#[link(name = "AppKit", kind = "framework")]
extern "C" {
    pub static NSWorkspaceDidActivateApplicationNotification: CFStringRef;
}

// dispatch_get_main_queue() is a C macro expanding to (&_dispatch_main_q)
extern "C" {
    static _dispatch_main_q: c_void;
    fn dispatch_async_f(
        queue: *const c_void,
        context: *mut c_void,
        work: unsafe extern "C" fn(*mut c_void),
    );
}

/// Open an app file picker deferred to the next run loop iteration.
/// This avoids re-entering druid's RefCell borrow during event handling.
pub fn defer_open_app_file_picker(callback: Box<dyn FnOnce(Option<String>) + Send>) {
    unsafe extern "C" fn work(ctx: *mut c_void) {
        let callback = Box::from_raw(ctx as *mut Box<dyn FnOnce(Option<String>) + Send>);
        let name = open_app_file_picker();
        callback(name);
    }

    let boxed: Box<Box<dyn FnOnce(Option<String>) + Send>> = Box::new(callback);
    let ctx_ptr = Box::into_raw(boxed) as *mut c_void;

    unsafe {
        dispatch_async_f(&_dispatch_main_q, ctx_ptr, work);
    }
}

pub fn open_app_file_picker() -> Option<String> {
    unsafe {
        let panel: id = msg_send![class!(NSOpenPanel), openPanel];
        let _: () = msg_send![panel, setCanChooseFiles: YES];
        let _: () = msg_send![panel, setCanChooseDirectories: NO];
        let _: () = msg_send![panel, setAllowsMultipleSelection: NO as BOOL];

        // Allow only .app bundles
        let app_ext = NSString::alloc(nil).init_str("app");
        let types_array: id = msg_send![class!(NSArray), arrayWithObject: app_ext];
        let _: () = msg_send![panel, setAllowedFileTypes: types_array];

        // Start in /Applications
        let apps_path = NSString::alloc(nil).init_str("/Applications");
        let dir_url: id = msg_send![class!(NSURL), fileURLWithPath: apps_path];
        let _: () = msg_send![panel, setDirectoryURL: dir_url];

        let response: i64 = msg_send![panel, runModal];
        if response == 1 {
            // NSModalResponseOK = 1
            let url: id = msg_send![panel, URL];
            let path: id = msg_send![url, path];

            let utf8: *const std::ffi::c_char = msg_send![path, UTF8String];
            if !utf8.is_null() {
                return Some(
                    std::ffi::CStr::from_ptr(utf8)
                        .to_string_lossy()
                        .into_owned(),
                );
            }
        }
        None
    }
}

pub fn open_text_file_picker() -> Option<String> {
    unsafe {
        let panel: id = msg_send![class!(NSOpenPanel), openPanel];
        let _: () = msg_send![panel, setCanChooseFiles: YES];
        let _: () = msg_send![panel, setCanChooseDirectories: NO];
        let _: () = msg_send![panel, setAllowsMultipleSelection: NO as BOOL];

        let response: i64 = msg_send![panel, runModal];
        if response == 1 {
            let url: id = msg_send![panel, URL];
            let path: id = msg_send![url, path];
            let utf8: *const std::ffi::c_char = msg_send![path, UTF8String];
            if !utf8.is_null() {
                return Some(
                    std::ffi::CStr::from_ptr(utf8)
                        .to_string_lossy()
                        .into_owned(),
                );
            }
        }
        None
    }
}

pub fn save_text_file_picker() -> Option<String> {
    unsafe {
        let panel: id = msg_send![class!(NSSavePanel), savePanel];
        let suggested_name = NSString::alloc(nil).init_str("expansions.txt");
        let _: () = msg_send![panel, setNameFieldStringValue: suggested_name];

        let response: i64 = msg_send![panel, runModal];
        if response == 1 {
            let url: id = msg_send![panel, URL];
            let path: id = msg_send![url, path];
            let utf8: *const std::ffi::c_char = msg_send![path, UTF8String];
            if !utf8.is_null() {
                return Some(
                    std::ffi::CStr::from_ptr(utf8)
                        .to_string_lossy()
                        .into_owned(),
                );
            }
        }
        None
    }
}

pub fn defer_open_text_file_picker(callback: Box<dyn FnOnce(Option<String>) + Send>) {
    unsafe extern "C" fn work(ctx: *mut c_void) {
        let callback = Box::from_raw(ctx as *mut Box<dyn FnOnce(Option<String>) + Send>);
        let path = open_text_file_picker();
        callback(path);
    }

    let boxed: Box<Box<dyn FnOnce(Option<String>) + Send>> = Box::new(callback);
    let ctx_ptr = Box::into_raw(boxed) as *mut c_void;

    unsafe {
        dispatch_async_f(&_dispatch_main_q, ctx_ptr, work);
    }
}

pub fn defer_save_text_file_picker(callback: Box<dyn FnOnce(Option<String>) + Send>) {
    unsafe extern "C" fn work(ctx: *mut c_void) {
        let callback = Box::from_raw(ctx as *mut Box<dyn FnOnce(Option<String>) + Send>);
        let path = save_text_file_picker();
        callback(path);
    }

    let boxed: Box<Box<dyn FnOnce(Option<String>) + Send>> = Box::new(callback);
    let ctx_ptr = Box::into_raw(boxed) as *mut c_void;

    unsafe {
        dispatch_async_f(&_dispatch_main_q, ctx_ptr, work);
    }
}

pub fn add_app_change_callback<F>(cb: F)
where
    F: Fn() + Send + 'static,
{
    unsafe {
        let shared_workspace: id = msg_send![class!(NSWorkspace), sharedWorkspace];
        let notification_center: id = msg_send![shared_workspace, notificationCenter];
        let cb_obj = Callback::from(Box::new(cb));

        let _: id = msg_send![notification_center,
            addObserver:cb_obj
            selector:sel!(call)
            name:NSWorkspaceDidActivateApplicationNotification
            object:nil
        ];
    }
}
