pub(crate) mod overlay;
pub(crate) mod shapes;

use std::ffi::c_void;

use windows::Win32::Foundation::{HWND, POINT, RECT};
use windows::Win32::Graphics::Dwm::{DWMWA_EXTENDED_FRAME_BOUNDS, DwmGetWindowAttribute};
use windows::Win32::Graphics::Gdi::{
    BITMAPINFO, BITMAPINFOHEADER, BitBlt, CAPTUREBLT, CreateCompatibleBitmap, CreateCompatibleDC,
    DIB_RGB_COLORS, DeleteDC, DeleteObject, GetDC, GetDIBits, GetMonitorInfoW, HGDIOBJ,
    MONITOR_DEFAULTTONEAREST, MONITORINFO, MonitorFromPoint, ROP_CODE, ReleaseDC, SRCCOPY,
    SelectObject,
};
use windows::Win32::System::Threading::GetCurrentProcessId;
use windows::Win32::UI::HiDpi::{
    DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, SetProcessDpiAwarenessContext,
};
use windows::Win32::UI::WindowsAndMessaging::{
    GA_ROOT, GetAncestor, GetCursorPos, GetWindowRect, GetWindowThreadProcessId, WindowFromPoint,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PhysRect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

impl PhysRect {
    pub fn new(x: i32, y: i32, w: i32, h: i32) -> Self {
        Self { x, y, w, h }
    }
}

#[derive(Clone)]
pub struct Frame {
    pub w: u32,
    pub h: u32,

    pub rgba: Vec<u8>,

    pub origin: PhysRect,
}

impl Frame {
    pub fn pixel(&self, x: i32, y: i32) -> [u8; 3] {
        if self.w == 0 || self.h == 0 || self.rgba.len() < 4 {
            return [0, 0, 0];
        }
        let x = x.clamp(0, self.w as i32 - 1);
        let y = y.clamp(0, self.h as i32 - 1);
        let i = ((y as u32 * self.w + x as u32) * 4) as usize;
        [self.rgba[i], self.rgba[i + 1], self.rgba[i + 2]]
    }

    pub fn to_color_image(&self) -> egui::ColorImage {
        egui::ColorImage::from_rgba_unmultiplied([self.w as usize, self.h as usize], &self.rgba)
    }
}

pub fn set_dpi_aware() {
    unsafe {
        let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
    }
}

pub fn cursor_pos() -> (i32, i32) {
    let mut p = POINT::default();
    unsafe {
        let _ = GetCursorPos(&mut p);
    }
    (p.x, p.y)
}

pub fn monitor_at_cursor() -> PhysRect {
    let (x, y) = cursor_pos();
    unsafe {
        let hmon = MonitorFromPoint(POINT { x, y }, MONITOR_DEFAULTTONEAREST);
        let mut info = MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        if GetMonitorInfoW(hmon, &mut info).as_bool() {
            let r = info.rcMonitor;
            PhysRect::new(r.left, r.top, r.right - r.left, r.bottom - r.top)
        } else {
            PhysRect::new(0, 0, 1920, 1080)
        }
    }
}

pub fn capture(region: PhysRect) -> Option<Frame> {
    let w = region.w.max(1);
    let h = region.h.max(1);
    unsafe {
        let screen = GetDC(None);
        if screen.is_invalid() {
            return None;
        }
        let mem = CreateCompatibleDC(Some(screen));
        let bmp = CreateCompatibleBitmap(screen, w, h);
        if mem.is_invalid() || bmp.is_invalid() {
            let _ = ReleaseDC(None, screen);
            return None;
        }
        let old = SelectObject(mem, HGDIOBJ(bmp.0));

        let rop = ROP_CODE(SRCCOPY.0 | CAPTUREBLT.0);
        let blit_ok = BitBlt(mem, 0, 0, w, h, Some(screen), region.x, region.y, rop).is_ok();

        let mut bi = BITMAPINFO::default();
        bi.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
        bi.bmiHeader.biWidth = w;
        bi.bmiHeader.biHeight = -h;
        bi.bmiHeader.biPlanes = 1;
        bi.bmiHeader.biBitCount = 32;
        bi.bmiHeader.biCompression = 0;

        let mut buf = vec![0u8; (w as usize) * (h as usize) * 4];
        let lines = if blit_ok {
            GetDIBits(
                mem,
                bmp,
                0,
                h as u32,
                Some(buf.as_mut_ptr() as *mut c_void),
                &mut bi,
                DIB_RGB_COLORS,
            )
        } else {
            0
        };

        SelectObject(mem, old);
        let _ = DeleteObject(HGDIOBJ(bmp.0));
        let _ = DeleteDC(mem);
        let _ = ReleaseDC(None, screen);

        if lines == 0 {
            return None;
        }

        for px in buf.as_chunks_mut::<4>().0 {
            px.swap(0, 2);
            px[3] = 255;
        }

        Some(Frame {
            w: w as u32,
            h: h as u32,
            rgba: buf,
            origin: region,
        })
    }
}

pub fn window_rect_at_cursor() -> Option<PhysRect> {
    let (x, y) = cursor_pos();
    unsafe {
        let hwnd = WindowFromPoint(POINT { x, y });
        if hwnd.is_invalid() {
            return None;
        }
        let mut pid = 0u32;
        GetWindowThreadProcessId(hwnd, Some(&mut pid));
        if pid == GetCurrentProcessId() {
            return None;
        }
        let root = GetAncestor(hwnd, GA_ROOT);
        if root.is_invalid() {
            return None;
        }
        let rect = extended_frame(root).or_else(|| {
            let mut r = RECT::default();
            GetWindowRect(root, &mut r).ok().map(|_| r)
        })?;
        let w = rect.right - rect.left;
        let h = rect.bottom - rect.top;
        if w < 8 || h < 8 {
            return None;
        }
        Some(PhysRect::new(rect.left, rect.top, w, h))
    }
}

fn extended_frame(hwnd: HWND) -> Option<RECT> {
    let mut rect = RECT::default();
    let r = unsafe {
        DwmGetWindowAttribute(
            hwnd,
            DWMWA_EXTENDED_FRAME_BOUNDS,
            &mut rect as *mut RECT as *mut c_void,
            std::mem::size_of::<RECT>() as u32,
        )
    };
    if r.is_ok() { Some(rect) } else { None }
}
