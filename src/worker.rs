//! Background thread: global hotkeys, single-instance IPC and "pin to screen" windows.
//!
//! Win32 window procedures are inherently unsafe, keep them readable.
#![allow(unsafe_op_in_unsafe_fn)]

use std::ffi::c_void;
use std::sync::atomic::{AtomicIsize, Ordering};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Mutex, OnceLock};

use windows::Win32::Foundation::{GetLastError, HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::{
    BITMAPINFO, BITMAPINFOHEADER, BeginPaint, DIB_RGB_COLORS, EndPaint, GetMonitorInfoW,
    InvalidateRect, MONITORINFO, MONITOR_DEFAULTTONEAREST, MonitorFromPoint, PAINTSTRUCT, SRCCOPY,
    StretchDIBits,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    MOD_ALT, MOD_CONTROL, MOD_NOREPEAT, RegisterHotKey, UnregisterHotKey, VK_ESCAPE, VK_F1, VK_F2,
    VK_F3, VK_Q, VK_S,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CREATESTRUCTW, CS_DBLCLKS, CS_HREDRAW, CS_VREDRAW, CreateWindowExW, DefWindowProcW,
    DestroyWindow, DispatchMessageW, FindWindowW, GWLP_USERDATA, GetClientRect, GetMessageW,
    GetWindowLongPtrW, GetWindowRect, HTCAPTION, HWND_MESSAGE, IsWindowVisible, MSG, PostMessageW,
    RegisterClassW, RegisterWindowMessageW, SW_SHOW, SWP_NOACTIVATE, SWP_NOZORDER, SendMessageW,
    SetForegroundWindow, SetWindowLongPtrW, SetWindowPos, ShowWindow, TranslateMessage,
    WINDOW_STYLE, WM_APP, WM_HOTKEY, WM_KEYDOWN, WM_LBUTTONDBLCLK, WM_LBUTTONDOWN, WM_MOUSEWHEEL,
    WM_NCCREATE, WM_NCDESTROY, WM_NCLBUTTONDOWN, WM_PAINT, WM_RBUTTONUP, WM_SETCURSOR, WNDCLASSW,
    WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP, WS_VISIBLE,
};
use windows::core::PCWSTR;

use crate::util::wide;

pub const WORKER_CLASS: &str = "RustShotWorkerWnd";
const PIN_CLASS: &str = "RustShotPinWnd";
const WM_CMD: u32 = WM_APP + 1;

/// Commands sent to the running instance by a second process.
pub const CMD_REGION: usize = 1;
pub const CMD_FULL: usize = 2;
pub const CMD_PIN: usize = 3;
pub const CMD_PANEL: usize = 4;

#[derive(Clone, Copy, Debug)]
pub enum WorkerEvent {
    CaptureRegion,
    CaptureFull,
    PinClipboard,
    ShowPanel,
    Quit,
}

#[derive(Clone)]
pub struct PinBitmap {
    pub w: u32,
    pub h: u32,
    /// BGRA, top-down.
    pub bgra: Vec<u8>,
}

impl PinBitmap {
    pub fn from_rgba(w: u32, h: u32, rgba: &[u8]) -> Self {
        let mut bgra = rgba.to_vec();
        for px in bgra.as_chunks_mut::<4>().0 {
            px.swap(0, 2);
        }
        Self { w, h, bgra }
    }

    pub fn from_image(img: &image::RgbaImage) -> Self {
        Self::from_rgba(img.width(), img.height(), img.as_raw())
    }
}

pub enum Cmd {
    Pin(Arc<PinBitmap>, (i32, i32)),
}

static EVENTS: OnceLock<Mutex<Sender<WorkerEvent>>> = OnceLock::new();
static WORKER_HWND: AtomicIsize = AtomicIsize::new(0);
static CMD_MSG: OnceLock<u32> = OnceLock::new();

pub struct Worker {
    pub rx: Receiver<WorkerEvent>,
    hwnd: isize,
}

impl Worker {
    pub fn spawn() -> Self {
        let (tx, rx) = channel();
        let _ = EVENTS.set(Mutex::new(tx));
        let (ready_tx, ready_rx) = channel::<isize>();
        std::thread::Builder::new()
            .name("rustshot-worker".into())
            .spawn(move || worker_thread(ready_tx))
            .expect("worker thread");
        let hwnd = ready_rx.recv().unwrap_or(0);
        if hwnd != 0 {
            WORKER_HWND.store(hwnd, Ordering::SeqCst);
        }
        Self { rx, hwnd }
    }

    pub fn send(&self, cmd: Cmd) {
        if self.hwnd == 0 {
            return;
        }
        let ptr = Box::into_raw(Box::new(cmd));
        unsafe {
            let _ = PostMessageW(
                Some(HWND(self.hwnd as *mut c_void)),
                WM_CMD,
                WPARAM(0),
                LPARAM(ptr as isize),
            );
        }
    }
}

fn emit(ev: WorkerEvent) {
    if let Some(m) = EVENTS.get()
        && let Ok(tx) = m.lock()
    {
        let _ = tx.send(ev);
    }
    crate::app::request_repaint();
}

/// Wake the running UI from another process / thread.
pub fn forward_to_running(cmd: usize) -> bool {
    unsafe {
        let class = wide(WORKER_CLASS);
        let Ok(hwnd) = FindWindowW(PCWSTR(class.as_ptr()), None) else {
            return false;
        };
        let msg = *CMD_MSG.get_or_init(registration_message);
        PostMessageW(Some(hwnd), msg, WPARAM(cmd), LPARAM(0)).is_ok()
    }
}

fn registration_message() -> u32 {
    let name = wide("RustShotCommand");
    unsafe { RegisterWindowMessageW(PCWSTR(name.as_ptr())) }
}

fn worker_thread(ready: Sender<isize>) {
    unsafe {
        let hinst = match GetModuleHandleW(None) {
            Ok(h) => h,
            Err(_) => {
                let _ = ready.send(0);
                return;
            }
        };
        let class_name = wide(WORKER_CLASS);
        let pin_class = wide(PIN_CLASS);

        let wc = WNDCLASSW {
            style: CS_DBLCLKS,
            lpfnWndProc: Some(worker_wndproc),
            hInstance: hinst.into(),
            lpszClassName: PCWSTR(class_name.as_ptr()),
            ..Default::default()
        };
        RegisterClassW(&wc);

        let hwnd = match CreateWindowExW(
            Default::default(),
            PCWSTR(class_name.as_ptr()),
            PCWSTR(class_name.as_ptr()),
            WINDOW_STYLE(0),
            0,
            0,
            0,
            0,
            Some(HWND_MESSAGE),
            None,
            Some(hinst.into()),
            None,
        ) {
            Ok(h) => h,
            Err(_) => {
                let _ = ready.send(0);
                return;
            }
        };

        let pc = WNDCLASSW {
            style: CS_DBLCLKS | CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(pin_wndproc),
            hInstance: hinst.into(),
            lpszClassName: PCWSTR(pin_class.as_ptr()),
            ..Default::default()
        };
        RegisterClassW(&pc);

        // Global hotkeys.
        let _ = RegisterHotKey(Some(hwnd), 1, MOD_NOREPEAT, VK_F1.0 as u32);
        let _ = RegisterHotKey(Some(hwnd), 2, MOD_NOREPEAT, VK_F2.0 as u32);
        let _ = RegisterHotKey(Some(hwnd), 3, MOD_NOREPEAT, VK_F3.0 as u32);
        let _ = RegisterHotKey(
            Some(hwnd),
            4,
            MOD_CONTROL | MOD_ALT | MOD_NOREPEAT,
            VK_S.0 as u32,
        );
        let _ = RegisterHotKey(
            Some(hwnd),
            5,
            MOD_CONTROL | MOD_ALT | MOD_NOREPEAT,
            VK_Q.0 as u32,
        );

        let _ = ready.send(hwnd.0 as isize);

        let mut msg = MSG::default();
        loop {
            let r = GetMessageW(&mut msg, None, 0, 0);
            if r.0 == 0 || r.0 == -1 {
                break;
            }
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }

        let _ = UnregisterHotKey(Some(hwnd), 1);
        let _ = UnregisterHotKey(Some(hwnd), 2);
        let _ = UnregisterHotKey(Some(hwnd), 3);
        let _ = UnregisterHotKey(Some(hwnd), 4);
        let _ = UnregisterHotKey(Some(hwnd), 5);
    }
}

unsafe extern "system" fn worker_wndproc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_HOTKEY => {
            match wparam.0 {
                1 => emit(WorkerEvent::CaptureRegion),
                2 => emit(WorkerEvent::CaptureFull),
                3 => emit(WorkerEvent::PinClipboard),
                4 => emit(WorkerEvent::CaptureRegion),
                5 => emit(WorkerEvent::Quit),
                _ => {}
            }
            LRESULT(0)
        }
        WM_CMD => {
            let ptr = lparam.0 as *mut Cmd;
            if !ptr.is_null() {
                let cmd = *Box::from_raw(ptr);
                match cmd {
                    Cmd::Pin(img, pos) => {
                        create_pin(img, pos);
                    }
                }
            }
            LRESULT(0)
        }
        m if m == registration_message() => {
            match wparam.0 {
                CMD_REGION => emit(WorkerEvent::CaptureRegion),
                CMD_FULL => emit(WorkerEvent::CaptureFull),
                CMD_PIN => emit(WorkerEvent::PinClipboard),
                CMD_PANEL => emit(WorkerEvent::ShowPanel),
                _ => {}
            }
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

// ---------------------------------------------------------------------------
// pinned images
// ---------------------------------------------------------------------------

struct PinState {
    img: Arc<PinBitmap>,
    scale: f32,
}

/// Where and how big a new pin should be: keep it on the monitor and shrink it
/// so that a full-screen shot does not land mostly off-screen.
fn pin_placement(w: i32, h: i32, pos: (i32, i32)) -> (i32, i32, i32, i32, f32) {
    unsafe {
        let mon = MonitorFromPoint(
            POINT {
                x: pos.0,
                y: pos.1,
            },
            MONITOR_DEFAULTTONEAREST,
        );
        let mut info = MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        let work = if GetMonitorInfoW(mon, &mut info).as_bool() {
            info.rcWork
        } else {
            RECT {
                left: 0,
                top: 0,
                right: 1920,
                bottom: 1080,
            }
        };
        let avail_w = (work.right - work.left - 24).max(120);
        let avail_h = (work.bottom - work.top - 24).max(90);
        let mut scale = 1.0f32;
        if w > avail_w || h > avail_h {
            scale = (avail_w as f32 / w as f32).min(avail_h as f32 / h as f32);
        }
        let nw = ((w as f32 * scale) as i32).max(24);
        let nh = ((h as f32 * scale) as i32).max(24);
        let mut x = pos.0 + 16;
        let mut y = pos.1 + 16;
        if x + nw > work.right - 8 {
            x = work.right - nw - 8;
        }
        if y + nh > work.bottom - 8 {
            y = work.bottom - nh - 8;
        }
        x = x.max(work.left + 8);
        y = y.max(work.top + 8);
        (x, y, nw, nh, scale)
    }
}

fn create_pin(img: Arc<PinBitmap>, pos: (i32, i32)) -> Option<HWND> {
    unsafe {
        let hinst = GetModuleHandleW(None).ok()?;
        let class = wide(PIN_CLASS);
        let title = wide("RustShot");
        let (x, y, w, h, scale) = pin_placement(img.w as i32, img.h as i32, pos);
        let state = Box::into_raw(Box::new(PinState {
            img: img.clone(),
            scale,
        }));
        let hwnd = match CreateWindowExW(
            WS_EX_TOPMOST | WS_EX_TOOLWINDOW,
            PCWSTR(class.as_ptr()),
            PCWSTR(title.as_ptr()),
            WS_POPUP | WS_VISIBLE,
            x,
            y,
            w,
            h,
            None,
            None,
            Some(hinst.into()),
            Some(state as *const c_void),
        ) {
            Ok(h) => h,
            Err(e) => {
                crate::util::log(&format!("create_pin failed: {e}"));
                return None;
            }
        };
        crate::util::log(&format!("pin created {w}x{h} at ({x},{y}) scale={scale:.2}"));
        let _ = SetWindowLongPtrW(hwnd, GWLP_USERDATA, state as isize);
        let _ = ShowWindow(hwnd, SW_SHOW);
        let _ = SetForegroundWindow(hwnd);
        Some(hwnd)
    }
}

unsafe fn pin_state(hwnd: HWND) -> Option<&'static mut PinState> {
    let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut PinState;
    ptr.as_mut()
}

unsafe extern "system" fn pin_wndproc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_NCCREATE => {
            let cs = lparam.0 as *const CREATESTRUCTW;
            if !cs.is_null() {
                let ptr = (*cs).lpCreateParams;
                let _ = SetWindowLongPtrW(hwnd, GWLP_USERDATA, ptr as isize);
            }
            LRESULT(1)
        }
        WM_PAINT => {
            let mut ps = PAINTSTRUCT::default();
            let hdc = BeginPaint(hwnd, &mut ps);
            let mut rc = RECT::default();
            let _ = GetClientRect(hwnd, &mut rc);
            if let Some(st) = pin_state(hwnd) {
                let mut bi = BITMAPINFO::default();
                bi.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
                bi.bmiHeader.biWidth = st.img.w as i32;
                bi.bmiHeader.biHeight = -(st.img.h as i32);
                bi.bmiHeader.biPlanes = 1;
                bi.bmiHeader.biBitCount = 32;
                bi.bmiHeader.biCompression = 0;
                StretchDIBits(
                    hdc,
                    0,
                    0,
                    rc.right - rc.left,
                    rc.bottom - rc.top,
                    0,
                    0,
                    st.img.w as i32,
                    st.img.h as i32,
                    Some(st.img.bgra.as_ptr() as *const c_void),
                    &bi,
                    DIB_RGB_COLORS,
                    SRCCOPY,
                );
            }
            let _ = EndPaint(hwnd, &ps);
            LRESULT(0)
        }
        WM_LBUTTONDOWN => {
            let _ = SendMessageW(
                hwnd,
                WM_NCLBUTTONDOWN,
                Some(WPARAM(HTCAPTION as usize)),
                Some(LPARAM(0)),
            );
            LRESULT(0)
        }
        WM_LBUTTONDBLCLK | WM_RBUTTONUP => {
            let _ = DestroyWindow(hwnd);
            LRESULT(0)
        }
        WM_KEYDOWN if wparam.0 == VK_ESCAPE.0 as usize => {
            let _ = DestroyWindow(hwnd);
            LRESULT(0)
        }
        WM_MOUSEWHEEL => {
            if let Some(st) = pin_state(hwnd) {
                let delta = (((wparam.0 >> 16) & 0xffff) as u16 as i16) as i32;
                let factor = if delta > 0 { 1.1 } else { 0.9 };
                let new_scale = (st.scale * factor).clamp(0.15, 6.0);
                if (new_scale - st.scale).abs() > 0.001 {
                    st.scale = new_scale;
                    let mut rc = RECT::default();
                    let _ = GetWindowRect(hwnd, &mut rc);
                    let cx = (rc.left + rc.right) / 2;
                    let cy = (rc.top + rc.bottom) / 2;
                    let nw = (st.img.w as f32 * new_scale).max(24.0) as i32;
                    let nh = (st.img.h as f32 * new_scale).max(24.0) as i32;
                    let _ = SetWindowPos(
                        hwnd,
                        None,
                        cx - nw / 2,
                        cy - nh / 2,
                        nw,
                        nh,
                        SWP_NOZORDER | SWP_NOACTIVATE,
                    );
                    let _ = InvalidateRect(Some(hwnd), None, true);
                }
            }
            LRESULT(0)
        }
        WM_NCDESTROY => {
            let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut PinState;
            if !ptr.is_null() {
                drop(Box::from_raw(ptr));
                let _ = SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
            }
            LRESULT(0)
        }
        WM_SETCURSOR => {
            let _ = IsWindowVisible(hwnd);
            LRESULT(1)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

/// True when another instance already owns the single-instance mutex.
pub fn instance_exists() -> bool {
    unsafe {
        use windows::Win32::Foundation::ERROR_ALREADY_EXISTS;
        use windows::Win32::System::Threading::CreateMutexW;
        let name = wide("Local\\RustShotSingleInstance");
        let _h = CreateMutexW(None, true, PCWSTR(name.as_ptr()));
        GetLastError() == ERROR_ALREADY_EXISTS
    }
}
