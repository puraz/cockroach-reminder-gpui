//! Platform-specific window/system integration.
//!
//! Uses OS-native APIs that don't conflict with GPUI's internal cocoa bindings.
//! On macOS, uses CoreGraphics (via `extern "C"`) for display queries instead of
//! objc2 to avoid runtime conflicts with GPUI's own `objc` v0.2.x dependency.

#[derive(Debug, Clone, Copy)]
pub struct ScreenFrame {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

// ---------------------------------------------------------------------------
// macOS – CoreGraphics via extern "C"
// ---------------------------------------------------------------------------

#[cfg(target_os = "macos")]
mod imp {
    use super::*;
    use raw_window_handle::RawWindowHandle;
    use std::ffi::{c_char, c_void};

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct CGPoint {
        x: f64,
        y: f64,
    }

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct CGSize {
        width: f64,
        height: f64,
    }

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct CGRect {
        origin: CGPoint,
        size: CGSize,
    }

    #[link(name = "CoreGraphics", kind = "framework")]
    extern "C" {
        fn CGMainDisplayID() -> u32;
        fn CGDisplayBounds(display: u32) -> CGRect;
        fn CGGetActiveDisplayList(maxDisplays: u32, displays: *mut u32, count: *mut u32) -> i32;
    }

    #[link(name = "objc")]
    extern "C" {
        fn objc_getClass(name: *const c_char) -> *mut c_void;
        fn sel_registerName(name: *const c_char) -> *mut c_void;
        fn objc_msgSend();
    }

    pub fn hide_dock() {
        unsafe {
            let application_class = objc_getClass(c"NSApplication".as_ptr());
            if application_class.is_null() {
                return;
            }

            let msg_send_id: unsafe extern "C" fn(*mut c_void, *mut c_void) -> *mut c_void =
                std::mem::transmute(objc_msgSend as *const ());
            let msg_send_policy: unsafe extern "C" fn(*mut c_void, *mut c_void, isize) -> i8 =
                std::mem::transmute(objc_msgSend as *const ());
            let shared_application = sel_registerName(c"sharedApplication".as_ptr());
            let set_activation_policy = sel_registerName(c"setActivationPolicy:".as_ptr());
            let application = msg_send_id(application_class, shared_application);

            if !application.is_null() {
                // NSApplicationActivationPolicyAccessory keeps menu-bar apps out of the Dock.
                msg_send_policy(application, set_activation_policy, 1);
            }
        }
    }

    /// Enumerate displays using CoreGraphics (no objc involved).
    pub fn screen_frames() -> Vec<ScreenFrame> {
        let mut out = Vec::new();
        unsafe {
            let main_id = CGMainDisplayID();
            let main = CGDisplayBounds(main_id);
            out.push(ScreenFrame {
                x: main.origin.x,
                y: main.origin.y,
                width: main.size.width,
                height: main.size.height,
            });

            // Try to get secondary displays.
            const MAX_DISPLAYS: u32 = 32;
            let mut displays = [0u32; MAX_DISPLAYS as usize];
            let mut count: u32 = 0;
            if CGGetActiveDisplayList(MAX_DISPLAYS, displays.as_mut_ptr(), &mut count) == 0 {
                for &did in displays.iter().take(count as usize) {
                    if did == main_id {
                        continue;
                    }
                    let bounds = CGDisplayBounds(did);
                    out.push(ScreenFrame {
                        x: bounds.origin.x,
                        y: bounds.origin.y,
                        width: bounds.size.width,
                        height: bounds.size.height,
                    });
                }
            }
        }
        out
    }

    /// Configure a GPUI popup as a click-through desktop overlay.
    pub fn configure_overlay(handle: &RawWindowHandle) {
        let RawWindowHandle::AppKit(handle) = handle else {
            return;
        };
        unsafe {
            let msg_send_id: unsafe extern "C" fn(*mut c_void, *mut c_void) -> *mut c_void =
                std::mem::transmute(objc_msgSend as *const ());
            let msg_send_bool: unsafe extern "C" fn(*mut c_void, *mut c_void, i8) =
                std::mem::transmute(objc_msgSend as *const ());
            let view = handle.ns_view.as_ptr();
            let window_selector = sel_registerName(c"window".as_ptr());
            let ignores_mouse_selector = sel_registerName(c"setIgnoresMouseEvents:".as_ptr());
            let window = msg_send_id(view, window_selector);
            if !window.is_null() {
                msg_send_bool(window, ignores_mouse_selector, 1);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Windows – Win32 API
// ---------------------------------------------------------------------------

#[cfg(windows)]
mod imp {
    use super::*;
    use raw_window_handle::RawWindowHandle;
    use windows_sys::Win32::Foundation::{BOOL, HWND, LPARAM, RECT, TRUE};
    use windows_sys::Win32::Graphics::Gdi::{EnumDisplayMonitors, HDC};
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GetWindowLongPtrW, SetWindowLongPtrW, SetWindowPos, GWL_EXSTYLE, HWND_TOPMOST,
        SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW,
        WS_EX_TRANSPARENT,
    };

    pub fn hide_dock() {}
    unsafe extern "system" fn monitor_enum_proc(
        _hmonitor: HDC,
        _hdc: HDC,
        rect: *mut RECT,
        data: LPARAM,
    ) -> BOOL {
        let frames = &mut *(data as *mut Vec<ScreenFrame>);
        frames.push(ScreenFrame {
            x: (*rect).left as f64,
            y: (*rect).top as f64,
            width: ((*rect).right - (*rect).left) as f64,
            height: ((*rect).bottom - (*rect).top) as f64,
        });
        TRUE
    }

    pub fn screen_frames() -> Vec<ScreenFrame> {
        let mut frames = Vec::new();
        unsafe {
            EnumDisplayMonitors(
                std::ptr::null_mut(),
                std::ptr::null(),
                Some(monitor_enum_proc),
                &mut frames as *mut _ as isize,
            );
        }
        frames
    }

    pub fn configure_overlay(handle: &RawWindowHandle) {
        if let RawWindowHandle::Win32(h) = handle {
            unsafe {
                let hwnd = h.hwnd.get() as HWND;
                let style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
                SetWindowLongPtrW(
                    hwnd,
                    GWL_EXSTYLE,
                    style
                        | WS_EX_TRANSPARENT as isize
                        | WS_EX_NOACTIVATE as isize
                        | WS_EX_TOOLWINDOW as isize,
                );
                SetWindowPos(
                    hwnd,
                    HWND_TOPMOST,
                    0,
                    0,
                    0,
                    0,
                    SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Linux (X11)
// ---------------------------------------------------------------------------

#[cfg(all(unix, not(target_os = "macos")))]
mod imp {
    use super::*;
    use raw_window_handle::RawWindowHandle;
    use x11rb::connection::Connection;
    use x11rb::protocol::xinerama::ConnectionExt as _;
    use x11rb::protocol::xproto::ConnectionExt as _;

    pub fn hide_dock() {}
    pub fn screen_frames() -> Vec<ScreenFrame> {
        let mut frames = Vec::new();
        let (conn, _sn) = match x11rb::connect(None) {
            Ok(v) => v,
            Err(_) => return frames,
        };
        if let Ok(c) = conn.xinerama_is_active() {
            if let Ok(r) = c.reply() {
                if r.state != 0 {
                    if let Ok(c) = conn.xinerama_query_screens() {
                        if let Ok(r) = c.reply() {
                            for si in r.screen_info {
                                frames.push(ScreenFrame {
                                    x: si.x_org as f64,
                                    y: si.y_org as f64,
                                    width: si.width as f64,
                                    height: si.height as f64,
                                });
                            }
                        }
                    }
                }
            }
        }
        if frames.is_empty() {
            if let Some(s) = conn.setup().roots.get(_sn) {
                frames.push(ScreenFrame {
                    x: 0.0,
                    y: 0.0,
                    width: s.width_in_pixels as f64,
                    height: s.height_in_pixels as f64,
                });
            }
        }
        frames
    }

    pub fn configure_overlay(handle: &RawWindowHandle) {
        let window = match handle {
            RawWindowHandle::Xlib(h) => h.window as u32,
            _ => return,
        };
        let (conn, _) = match x11rb::connect(None) {
            Ok(v) => v,
            Err(_) => return,
        };
        let setup = conn.setup();
        if setup.roots.is_empty() {
            return;
        }
        let root = setup.roots[0].root;
        let state_atom = || -> Option<u32> {
            conn.intern_atom(false, b"_NET_WM_STATE")
                .ok()?
                .reply()
                .ok()
                .map(|r| r.atom)
        }()
        .unwrap_or(0);
        let above_atom = || -> Option<u32> {
            conn.intern_atom(false, b"_NET_WM_STATE_ABOVE")
                .ok()?
                .reply()
                .ok()
                .map(|r| r.atom)
        }()
        .unwrap_or(0);
        if state_atom == 0 || above_atom == 0 {
            return;
        }
        let event = x11rb::protocol::xproto::ClientMessageEvent::new(
            32,
            window,
            state_atom,
            [2u32, above_atom, 0, 0, 0],
        );
        let _ = conn.send_event(
            false,
            root,
            x11rb::protocol::xproto::EventMask::SUBSTRUCTURE_REDIRECT
                | x11rb::protocol::xproto::EventMask::SUBSTRUCTURE_NOTIFY,
            event,
        );
        let _ = conn.flush();
    }
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

pub use imp::{configure_overlay, hide_dock, screen_frames};
