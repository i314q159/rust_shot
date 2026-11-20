//! A couple of Win32 window helpers used to place our own windows exactly.

use windows::Win32::Foundation::HWND;
use windows::Win32::UI::WindowsAndMessaging::{
    FindWindowW, IsIconic, IsWindowVisible, SWP_NOACTIVATE, SWP_NOZORDER, SetWindowPos,
};
use windows::core::PCWSTR;

use crate::capture::PhysRect;
use crate::util::wide;

pub fn find_by_title(title: &str) -> Option<HWND> {
    let t = wide(title);
    let hwnd = unsafe { FindWindowW(None, PCWSTR(t.as_ptr())) }.ok()?;
    if hwnd.is_invalid() { None } else { Some(hwnd) }
}

/// Move/resize a window using device pixels, bypassing all DPI guesswork.
pub fn force_rect(hwnd: HWND, r: PhysRect) -> bool {
    unsafe {
        SetWindowPos(
            hwnd,
            None,
            r.x,
            r.y,
            r.w,
            r.h,
            SWP_NOZORDER | SWP_NOACTIVATE,
        )
        .is_ok()
    }
}

pub fn force_rect_by_title(title: &str, r: PhysRect) -> bool {
    match find_by_title(title) {
        Some(h) => force_rect(h, r),
        None => false,
    }
}

/// A window that exists, is visible and is not minimized is ready to host
/// child viewports (eframe cannot create them from a hidden window).
pub fn is_ready(title: &str) -> bool {
    match find_by_title(title) {
        Some(h) => unsafe { IsWindowVisible(h).as_bool() && !IsIconic(h).as_bool() },
        None => false,
    }
}
