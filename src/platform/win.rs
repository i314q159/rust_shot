

use windows::Win32::Foundation::{HWND, RECT};
use windows::Win32::UI::WindowsAndMessaging::{
    FindWindowW, GWL_EXSTYLE, GetWindowLongW, GetWindowPlacement, GetWindowRect, IsIconic,
    IsWindowVisible, SW_MINIMIZE, SW_SHOWNOACTIVATE, SWP_FRAMECHANGED, SWP_NOACTIVATE, SWP_NOSIZE,
    SWP_NOZORDER, SetWindowLongW, SetWindowPos, ShowWindow, WINDOWPLACEMENT, WS_EX_APPWINDOW,
    WS_EX_TOOLWINDOW,
};
use windows::core::PCWSTR;

use crate::capture::PhysRect;
use crate::platform::util::wide;

pub fn find_by_title(title: &str) -> Option<HWND> {
    let t = wide(title);
    let hwnd = unsafe { FindWindowW(None, PCWSTR(t.as_ptr())) }.ok()?;
    if hwnd.is_invalid() { None } else { Some(hwnd) }
}


pub fn force_rect(hwnd: HWND, r: PhysRect) -> bool {
    unsafe {
        let ok = SetWindowPos(
            hwnd,
            None,
            r.x,
            r.y,
            r.w,
            r.h,
            SWP_NOZORDER | SWP_NOACTIVATE,
        )
        .is_ok();
        if ok {
            let mut actual = RECT::default();
            if GetWindowRect(hwnd, &mut actual).is_ok() {
                let aw = actual.right - actual.left;
                let ah = actual.bottom - actual.top;
                if aw != r.w || ah != r.h || actual.left != r.x || actual.top != r.y {
                    crate::platform::util::log(&format!(
                        "force_rect mismatch: want {}x{} at ({},{}), got {}x{} at ({},{})",
                        r.w, r.h, r.x, r.y, aw, ah, actual.left, actual.top
                    ));
                }
            }
        }
        ok
    }
}

pub fn force_rect_by_title(title: &str, r: PhysRect) -> bool {
    match find_by_title(title) {
        Some(h) => force_rect(h, r),
        None => false,
    }
}



pub fn is_ready(title: &str) -> bool {
    match find_by_title(title) {
        Some(h) => unsafe { IsWindowVisible(h).as_bool() && !IsIconic(h).as_bool() },
        None => false,
    }
}


const PARK_X: i32 = -32000;
const PARK_Y: i32 = -32000;



#[derive(Clone, Copy, Debug)]
pub struct Parked {
    pub rect: PhysRect,
    
    pub app_window: bool,
}



fn normal_rect(hwnd: HWND) -> Option<PhysRect> {
    unsafe {
        let mut wp = WINDOWPLACEMENT {
            length: std::mem::size_of::<WINDOWPLACEMENT>() as u32,
            ..Default::default()
        };
        GetWindowPlacement(hwnd, &mut wp).ok()?;
        let r = wp.rcNormalPosition;
        let rect = PhysRect::new(r.left, r.top, r.right - r.left, r.bottom - r.top);
        (rect.w > 0 && rect.h > 0).then_some(rect)
    }
}





pub fn park_offscreen(title: &str) -> Option<Parked> {
    let hwnd = find_by_title(title)?;
    let rect = normal_rect(hwnd)?;
    unsafe {
        let ex = GetWindowLongW(hwnd, GWL_EXSTYLE);
        let tool = WS_EX_TOOLWINDOW.0 as i32;
        let app = WS_EX_APPWINDOW.0 as i32;
        let app_window = ex & app != 0;
        if ex & tool == 0 {
            SetWindowLongW(hwnd, GWL_EXSTYLE, (ex | tool) & !app);
        }
        
        SetWindowPos(
            hwnd,
            None,
            PARK_X,
            PARK_Y,
            0,
            0,
            SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE | SWP_FRAMECHANGED,
        )
        .ok()?;
        
        
        
        let _ = ShowWindow(hwnd, SW_MINIMIZE);
        
        
        
        let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
        Some(Parked { rect, app_window })
    }
}


pub fn unpark(title: &str, parked: Parked) -> bool {
    let Some(hwnd) = find_by_title(title) else {
        return false;
    };
    unsafe {
        let ex = GetWindowLongW(hwnd, GWL_EXSTYLE);
        let tool = WS_EX_TOOLWINDOW.0 as i32;
        let app = WS_EX_APPWINDOW.0 as i32;
        let mut ex = ex & !tool;
        if parked.app_window {
            ex |= app;
        }
        if ex != GetWindowLongW(hwnd, GWL_EXSTYLE) {
            SetWindowLongW(hwnd, GWL_EXSTYLE, ex);
        }
        SetWindowPos(
            hwnd,
            None,
            parked.rect.x,
            parked.rect.y,
            parked.rect.w,
            parked.rect.h,
            SWP_NOZORDER | SWP_NOACTIVATE | SWP_FRAMECHANGED,
        )
        .is_ok()
    }
}




pub fn hide_from_taskbar(title: &str) {
    let Some(hwnd) = find_by_title(title) else {
        return;
    };
    unsafe {
        let ex = GetWindowLongW(hwnd, GWL_EXSTYLE);
        let tool = WS_EX_TOOLWINDOW.0 as i32;
        let app = WS_EX_APPWINDOW.0 as i32;
        if ex & tool == 0 || ex & app != 0 {
            SetWindowLongW(hwnd, GWL_EXSTYLE, (ex | tool) & !app);
        }
    }
}


pub fn show_without_activation(title: &str) -> bool {
    match find_by_title(title) {
        Some(h) => unsafe {
            let _ = ShowWindow(h, SW_SHOWNOACTIVATE);
            true
        },
        None => false,
    }
}
