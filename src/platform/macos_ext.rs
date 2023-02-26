use cocoa::appkit::{NSApp, NSApplication, NSButton, NSMenu, NSStatusBar, NSStatusItem};
use cocoa::base::{nil, YES};
use cocoa::foundation::{NSAutoreleasePool, NSString};
use core_graphics::{
    event::{CGEventTapProxy, CGKeyCode},
    sys,
};
use druid::{Data, Lens};

#[derive(Clone, PartialEq, Eq)]
struct Wrapper(*mut objc::runtime::Object);
impl Data for Wrapper {
    fn same(&self, _other: &Self) -> bool {
        true
    }
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
            let title = NSString::alloc(nil).init_str("VN");
            item.setTitle_(title);
            item.setMenu_(menu);

            Self {
                _pool: Wrapper(pool),
                menu: Wrapper(menu),
                item: Wrapper(item),
            }
        }
    }

    pub fn set_title(&mut self, title: &str) {
        unsafe {
            let title = NSString::alloc(nil).init_str(title);
            self.item.0.setTitle_(title);
        }
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

    pub struct CGEventTap<'tap_life> {
        pub mach_port: CFMachPort,
        pub callback_ref:
            Box<dyn Fn(CGEventTapProxy, CGEventType, &CGEvent) -> Option<CGEvent> + 'tap_life>,
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
