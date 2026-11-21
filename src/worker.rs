//! Background thread: global hotkeys and single-instance IPC.
//!
//! The window procedure is inherently unsafe; keep it readable.
#![allow(unsafe_op_in_unsafe_fn)]

use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Mutex, OnceLock};

use windows::Win32::Foundation::{
    CloseHandle, GetLastError, HANDLE, HWND, LPARAM, LRESULT, WPARAM,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    MOD_ALT, MOD_CONTROL, MOD_NOREPEAT, RegisterHotKey, UnregisterHotKey, VK_F1, VK_F2, VK_Q, VK_S,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DispatchMessageW, FindWindowW, GetMessageW, HWND_MESSAGE, MSG,
    PostMessageW, RegisterClassW, RegisterWindowMessageW, TranslateMessage, WINDOW_STYLE,
    WM_HOTKEY, WNDCLASSW, WS_POPUP,
};
use windows::core::PCWSTR;

use crate::util::wide;

pub const WORKER_CLASS: &str = "RustShotWorkerWnd";
/// Hidden top-level window that hears the system-wide `WM_SETTINGCHANGE`.
pub const THEME_CLASS: &str = "RustShotThemeWnd";

/// Commands sent to the running instance by a second process.
pub const CMD_REGION: usize = 1;
pub const CMD_FULL: usize = 2;
pub const CMD_PANEL: usize = 4;
pub const CMD_CHECK_UPDATE: usize = 8;

#[derive(Clone, Copy, Debug)]
pub enum WorkerEvent {
    CaptureRegion,
    /// Full screen straight to the clipboard (CLI `--full`).
    CaptureFull,
    /// Full screen through the overlay, whole frame preselected (F2).
    CaptureFullOverlay,
    ShowPanel,
    /// `--check-update` asked the running instance to check for updates.
    CheckUpdate,
    /// The OS light/dark setting may have changed.
    ThemeChanged,
    Quit,
}

static EVENTS: OnceLock<Mutex<Sender<WorkerEvent>>> = OnceLock::new();
static CMD_MSG: OnceLock<u32> = OnceLock::new();

pub struct Worker {
    pub rx: Receiver<WorkerEvent>,
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
        let _ = ready_rx.recv();
        Self { rx }
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
        PostMessageW(Some(hwnd), cmd_message(), WPARAM(cmd), LPARAM(0)).is_ok()
    }
}

/// Cached `RegisterWindowMessageW("RustShotCommand")` id (one syscall per process).
fn cmd_message() -> u32 {
    *CMD_MSG.get_or_init(registration_message)
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

        let wc = WNDCLASSW {
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

        // A message-only window never receives broadcast messages, so watching
        // the OS light/dark switch needs a real — but never shown — top-level
        // window.
        let theme_class = wide(THEME_CLASS);
        let theme_wc = WNDCLASSW {
            lpfnWndProc: Some(theme_wndproc),
            hInstance: hinst.into(),
            lpszClassName: PCWSTR(theme_class.as_ptr()),
            ..Default::default()
        };
        RegisterClassW(&theme_wc);
        let _theme_wnd = CreateWindowExW(
            Default::default(),
            PCWSTR(theme_class.as_ptr()),
            PCWSTR(theme_class.as_ptr()),
            WS_POPUP,
            0,
            0,
            0,
            0,
            None,
            None,
            Some(hinst.into()),
            None,
        );

        // Global hotkeys.
        let _ = RegisterHotKey(Some(hwnd), 1, MOD_NOREPEAT, VK_F1.0 as u32);
        let _ = RegisterHotKey(Some(hwnd), 2, MOD_NOREPEAT, VK_F2.0 as u32);
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
                2 => emit(WorkerEvent::CaptureFullOverlay),
                4 => emit(WorkerEvent::CaptureRegion),
                5 => emit(WorkerEvent::Quit),
                _ => {}
            }
            LRESULT(0)
        }
        m if m == cmd_message() => {
            match wparam.0 {
                CMD_REGION => emit(WorkerEvent::CaptureRegion),
                CMD_FULL => emit(WorkerEvent::CaptureFull),
                CMD_PANEL => emit(WorkerEvent::ShowPanel),
                CMD_CHECK_UPDATE => emit(WorkerEvent::CheckUpdate),
                _ => {}
            }
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

unsafe extern "system" fn theme_wndproc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    /// Broadcast whenever the user changes colours, metrics or the theme.
    const WM_SETTINGCHANGE: u32 = 0x001A;
    if msg == WM_SETTINGCHANGE {
        // Sent for many unrelated settings too; the app re-reads the value and
        // ignores the ones that did not change the theme.
        emit(WorkerEvent::ThemeChanged);
        return LRESULT(0);
    }
    DefWindowProcW(hwnd, msg, wparam, lparam)
}

/// Handle of our own single-instance mutex, kept until the process exits.
/// Dropping the handle would release ownership and let a second instance in.
static INSTANCE_MUTEX: std::sync::atomic::AtomicIsize = std::sync::atomic::AtomicIsize::new(0);

/// Does any process still hold the single-instance mutex?
///
/// The updater asks this before running the installer: Inno refuses to install
/// while `AppMutex` exists, and with `/VERYSILENT /SUPPRESSMSGBOXES` that shows
/// up as a silent abort.
pub fn mutex_exists() -> bool {
    unsafe {
        use windows::Win32::Foundation::ERROR_ALREADY_EXISTS;
        use windows::Win32::System::Threading::CreateMutexW;
        let name = wide("Local\\RustShotSingleInstance");
        match CreateMutexW(None, false, PCWSTR(name.as_ptr())) {
            Ok(handle) => {
                let existed = GetLastError() == ERROR_ALREADY_EXISTS;
                let _ = CloseHandle(handle);
                existed
            }
            Err(_) => false,
        }
    }
}

/// Drop the single-instance mutex before quitting for an update, otherwise the
/// freshly installed copy would see the old process and exit silently.
pub fn release_instance() {
    let raw = INSTANCE_MUTEX.swap(0, std::sync::atomic::Ordering::SeqCst);
    if raw != 0 {
        unsafe {
            let _ = CloseHandle(HANDLE(raw as *mut std::ffi::c_void));
        }
    }
}

/// True when another instance already owns the single-instance mutex.
pub fn instance_exists() -> bool {
    unsafe {
        use windows::Win32::Foundation::ERROR_ALREADY_EXISTS;
        use windows::Win32::System::Threading::CreateMutexW;
        let name = wide("Local\\RustShotSingleInstance");
        match CreateMutexW(None, true, PCWSTR(name.as_ptr())) {
            Ok(handle) => {
                INSTANCE_MUTEX.store(handle.0 as isize, std::sync::atomic::Ordering::SeqCst);
                GetLastError() == ERROR_ALREADY_EXISTS
            }
            Err(_) => false,
        }
    }
}
