// Windows platform implementation for GoxKey
use std::env;
use std::path::PathBuf;
use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use std::os::raw::c_void;
use std::ptr;
use std::cell::RefCell;

use winapi::um::winuser::*;
use winapi::um::winreg::*;
use winapi::um::winnt::{KEY_WRITE, KEY_READ, REG_SZ};
use winapi::shared::winerror::ERROR_SUCCESS;
use winapi::shared::windef::HHOOK;
use winapi::shared::minwindef::{LPARAM, WPARAM, LRESULT, FALSE};

use super::{CallbackFn, EventTapType, KeyModifier, PressedKey, KEY_DELETE, KEY_ENTER, KEY_ESCAPE, KEY_SPACE, KEY_TAB};
use druid::Data;

pub const SYMBOL_ALT: &str = "Alt";
pub const SYMBOL_CTRL: &str = "Ctrl";
pub const SYMBOL_SHIFT: &str = "Shift";
pub const SYMBOL_SUPER: &str = "Win";

pub type Handle = isize;

thread_local! {
    static HOOK_STATE: RefCell<HookState> = RefCell::new(HookState {
        hook: ptr::null_mut(),
        callback: None,
    });
}

struct HookState {
    hook: HHOOK,
    callback: Option<*const CallbackFn>,
}

pub fn get_home_dir() -> Option<PathBuf> {
    env::var("USERPROFILE")
        .ok()
        .map(PathBuf::from)
        .or_else(|| {
            env::var("HOMEDRIVE").ok().and_then(|home_drive| {
                env::var("HOMEPATH").ok().map(|home_path| {
                    PathBuf::from(format!("{}{}", home_drive, home_path))
                })
            })
        })
}

/// Send a string of text using Windows SendInput API
pub fn send_string(_handle: Handle, string: &str) -> Result<(), ()> {
    unsafe {
        for ch in string.chars() {
            let mut input: INPUT = std::mem::zeroed();
            input.type_ = INPUT_KEYBOARD;
            
            let ki = &mut input.u.ki_mut();
            ki.wVk = 0;
            ki.wScan = ch as u16;
            ki.dwFlags = KEYEVENTF_UNICODE;
            ki.time = 0;
            ki.dwExtraInfo = 0;
            
            if SendInput(1, &mut input, std::mem::size_of::<INPUT>() as i32) == 0 {
                return Err(());
            }
        }
    }
    Ok(())
}

/// Send backspace key press using SendInput API
pub fn send_backspace(_handle: Handle, count: usize) -> Result<(), ()> {
    unsafe {
        for _ in 0..count {
            let mut input_down: INPUT = std::mem::zeroed();
            input_down.type_ = INPUT_KEYBOARD;
            input_down.u.ki_mut().wVk = VK_BACK as u16;
            input_down.u.ki_mut().dwFlags = 0;
            
            let mut input_up: INPUT = std::mem::zeroed();
            input_up.type_ = INPUT_KEYBOARD;
            input_up.u.ki_mut().wVk = VK_BACK as u16;
            input_up.u.ki_mut().dwFlags = KEYEVENTF_KEYUP;
            
            if SendInput(1, &mut input_down, std::mem::size_of::<INPUT>() as i32) == 0 {
                return Err(());
            }
            if SendInput(1, &mut input_up, std::mem::size_of::<INPUT>() as i32) == 0 {
                return Err(());
            }
        }
    }
    Ok(())
}

/// Low-level keyboard hook procedure
unsafe extern "system" fn keyboard_hook_proc(
    n_code: i32,
    w_param: WPARAM,
    l_param: LPARAM,
) -> LRESULT {
    if n_code >= 0 {
        let p_kbd = l_param as *const KBDLLHOOKSTRUCT;
        if !p_kbd.is_null() {
            let kbd_struct = *p_kbd;
            
            // Ignore injected events (from SendInput) to prevent feedback loops
            if (kbd_struct.flags & LLKHF_INJECTED) != 0 {
                return CallNextHookEx(ptr::null_mut(), n_code, w_param, l_param);
            }
            
            // Only process key down events
            if w_param as u32 == WM_KEYDOWN || w_param as u32 == WM_SYSKEYDOWN {
                let mut block_key = false;
                HOOK_STATE.with(|state| {
                    if let Some(callback_ptr) = state.borrow().callback {
                        let mut modifiers = KeyModifier::new();
                        
                        // Properly populate key_state using GetAsync/GetKeyState
                        // GetKeyboardState is unreliable in LL hook for other threads
                        let mut key_state = [0u8; 256];

                        if (GetAsyncKeyState(VK_SHIFT) as u16 & 0x8000) != 0 {
                            key_state[VK_SHIFT as usize] = 0x80;
                            modifiers.add_shift();
                        }
                        if (GetAsyncKeyState(VK_CONTROL) as u16 & 0x8000) != 0 {
                            key_state[VK_CONTROL as usize] = 0x80;
                            modifiers.add_control();
                        }
                        if (GetAsyncKeyState(VK_MENU) as u16 & 0x8000) != 0 {
                            key_state[VK_MENU as usize] = 0x80;
                            modifiers.add_alt();
                        }
                        if (GetAsyncKeyState(VK_LWIN) as u16 & 0x8000) != 0
                            || (GetAsyncKeyState(VK_RWIN) as u16 & 0x8000) != 0
                        {
                            key_state[VK_LWIN as usize] = 0x80;
                            key_state[VK_RWIN as usize] = 0x80;
                            modifiers.add_super();
                        }
                        if (GetKeyState(VK_CAPITAL) & 1) != 0 {
                            key_state[VK_CAPITAL as usize] = 1;
                            modifiers.add_capslock();
                        }
                        
                        let virtual_key = kbd_struct.vkCode as u16;
                        let scan_code = kbd_struct.scanCode as u16;
                        
                        // Pass key_state and scan_code to virtual_key_to_pressed_key
                        let pressed_key = virtual_key_to_pressed_key(virtual_key, scan_code, &key_state);
                        
                        // Call the callback - if it returns true, block the key
                        let cb = &*callback_ptr;
                        if cb(0, EventTapType::KeyDown, pressed_key, modifiers) {
                            block_key = true;
                        }
                    }
                });
                if block_key {
                    return 1; // Block the key
                }
            }
        }
    }
    CallNextHookEx(ptr::null_mut(), n_code, w_param, l_param)
}

/// Convert Windows virtual key code to PressedKey, taking keyboard state into account for actual character
fn virtual_key_to_pressed_key(vk: u16, scan_code: u16, key_state: &[u8; 256]) -> Option<PressedKey> {
    let mut w_char_buf = [0u16; 2];
    let result = unsafe {
        ToUnicode(
            vk as u32,
            scan_code as u32,
            key_state.as_ptr(),
            w_char_buf.as_mut_ptr(),
            w_char_buf.len() as i32,
            0,
        )
    };

    if result > 0 {
        // A character was produced
        std::char::from_u32(w_char_buf[0] as u32).map(PressedKey::Char)
    } else {
        // No character (e.g., modifier key, function key, or key that doesn't produce a char)
        match vk as i32 {
            VK_RETURN => Some(PressedKey::Char(KEY_ENTER)),
            VK_SPACE => Some(PressedKey::Char(KEY_SPACE)),
            VK_TAB => Some(PressedKey::Char(KEY_TAB)),
            VK_BACK => Some(PressedKey::Char(KEY_DELETE)),
            VK_ESCAPE => Some(PressedKey::Char(KEY_ESCAPE)),
            _ => Some(PressedKey::Raw(vk)),
        }
    }
}

/// Start listening to keyboard events with a global hook
pub fn run_event_listener(callback: &CallbackFn) {
    unsafe {
        HOOK_STATE.with(|state| {
            state.borrow_mut().callback = Some(callback as *const CallbackFn);
        });
        
        let hook = SetWindowsHookExW(
            WH_KEYBOARD_LL,
            Some(keyboard_hook_proc),
            ptr::null_mut(),
            0,
        );
        
        if hook.is_null() {
            eprintln!("Failed to install keyboard hook");
            return;
        }
        
        HOOK_STATE.with(|state| {
            state.borrow_mut().hook = hook;
        });
        
        // Message loop to keep the hook active
        let mut msg: MSG = std::mem::zeroed();
        while GetMessageW(&mut msg, ptr::null_mut(), 0, 0) > 0 {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
}

pub fn ensure_accessibility_permission() -> bool {
    // Windows doesn't require explicit accessibility permissions like macOS
    // Return true to indicate permissions are available (or user needs to configure at OS level)
    true
}

pub fn is_in_text_selection() -> bool {
    // Windows text selection detection is complex without UI Automation
    // For now, return false. Could be enhanced with UI Automation API later
    false
}

/// Launch on login via Windows Registry
pub fn update_launch_on_login(is_enable: bool) -> Result<(), ()> {
    unsafe {
        let run_key = "Software\\Microsoft\\Windows\\CurrentVersion\\Run";
        let app_name = "GoxKey";
        
        let mut hkey: *mut c_void = ptr::null_mut();
        
        let run_key_wide: Vec<u16> = OsStr::new(run_key)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        
        // Open the Run registry key
        let result = RegOpenKeyExW(
            HKEY_CURRENT_USER,
            run_key_wide.as_ptr(),
            0,
            KEY_WRITE,
            std::mem::transmute(&mut hkey),
        ) as u32;
        
        if result != ERROR_SUCCESS {
            return Err(());
        }
        
        if is_enable {
            // Get the current executable path
            if let Ok(exe_path) = std::env::current_exe() {
                let app_name_wide: Vec<u16> = OsStr::new(app_name)
                    .encode_wide()
                    .chain(std::iter::once(0))
                    .collect();
                
                let exe_path_str = exe_path.to_string_lossy();
                let path_wide: Vec<u16> = exe_path_str.as_ref()
                    .encode_utf16()
                    .chain(std::iter::once(0))
                    .collect();
                
                let set_result = RegSetValueExW(
                    hkey as *mut _,
                    app_name_wide.as_ptr(),
                    0,
                    REG_SZ,
                    path_wide.as_ptr() as *const u8,
                    (path_wide.len() * 2) as u32,
                ) as u32;
                
                RegCloseKey(hkey as *mut _);
                
                if set_result != ERROR_SUCCESS {
                    return Err(());
                }
            } else {
                return Err(());
            }
        } else {
            // Remove from Run registry
            let app_name_wide: Vec<u16> = OsStr::new(app_name)
                .encode_wide()
                .chain(std::iter::once(0))
                .collect();
            
            RegDeleteValueW(hkey as *mut _, app_name_wide.as_ptr());
            RegCloseKey(hkey as *mut _);
        }
    }
    
    Ok(())
}

pub fn is_launch_on_login() -> bool {
    unsafe {
        let run_key = "Software\\Microsoft\\Windows\\CurrentVersion\\Run";
        let app_name = "GoxKey";
        
        let mut hkey: *mut c_void = ptr::null_mut();
        
        let run_key_wide: Vec<u16> = OsStr::new(run_key)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        
        let result = RegOpenKeyExW(
            HKEY_CURRENT_USER,
            run_key_wide.as_ptr(),
            0,
            KEY_READ,
            std::mem::transmute(&mut hkey),
        ) as u32;
        
        if result != ERROR_SUCCESS {
            return false;
        }
        
        let app_name_wide: Vec<u16> = OsStr::new(app_name)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        
        let mut value_type: u32 = 0;
        let mut data: [u8; 260] = [0; 260];
        let mut data_size: u32 = 260;
        
        let query_result = RegQueryValueExW(
            hkey as *mut _,
            app_name_wide.as_ptr(),
            ptr::null_mut(),
            &mut value_type,
            data.as_mut_ptr(),
            &mut data_size,
        ) as u32;
        
        RegCloseKey(hkey as *mut _);
        
        query_result == ERROR_SUCCESS
    }
}

pub fn get_active_app_name() -> String {
    unsafe {
        let foreground_hwnd = GetForegroundWindow();
        if foreground_hwnd.is_null() {
            return "Unknown".to_string();
        }
        
        let mut title_buffer: [u16; 256] = [0; 256];
        let len = GetWindowTextW(
            foreground_hwnd,
            title_buffer.as_mut_ptr(),
            title_buffer.len() as i32,
        );
        
        if len > 0 {
            String::from_utf16_lossy(&title_buffer[..len as usize]).to_string()
        } else {
            "Unknown".to_string()
        }
    }
}

pub fn add_app_change_callback(_callback: Box<dyn Fn() + Send + 'static>) {
    // Windows app change detection would require WinEventHook setup
    // For now, this is a placeholder - in production would use SetWinEventHook
    // with EVENT_SYSTEM_FOREGROUND to detect window changes
    println!("DEBUG: add_app_change_callback called (stub)");
}

pub fn defer_open_app_file_picker(_callback: Box<dyn Fn(Option<String>) + Send + 'static>) {
    // File picker would be implemented with GetOpenFileNameW
    println!("DEBUG: defer_open_app_file_picker called (stub)");
}

pub fn defer_open_text_file_picker(_callback: Box<dyn Fn(Option<String>) + Send + 'static>) {
    // File picker would be implemented with GetOpenFileNameW
    println!("DEBUG: defer_open_text_file_picker called (stub)");
}

pub fn defer_save_text_file_picker(_callback: Box<dyn Fn(Option<String>) + Send + 'static>) {
    // File picker would be implemented with GetSaveFileNameW
    println!("DEBUG: defer_save_text_file_picker called (stub)");
}

#[derive(Clone, PartialEq, Eq)]
pub struct SystemTray;

impl Data for SystemTray {
    fn same(&self, _other: &Self) -> bool {
        true
    }
}

impl SystemTray {
    pub fn new() -> Self {
        Self
    }
    
    pub fn set_title(&self, _title: &str) {
        // Windows system tray title management
    }
    
    pub fn set_menu_item_title(&self, _key: SystemTrayMenuItemKey, _title: &str) {
        // Update system tray menu item title
    }
    
    pub fn set_menu_item_callback(&self, _key: SystemTrayMenuItemKey, _callback: impl Fn() + 'static) {
        // Set callback for system tray menu item
    }
}

#[derive(Clone, PartialEq, Eq)]
pub enum SystemTrayMenuItemKey {
    Enable,
    ShowUI,
    TypingMethodTelex,
    TypingMethodVNI,
    TypingMethodTelexVNI,
    Exit,
}
