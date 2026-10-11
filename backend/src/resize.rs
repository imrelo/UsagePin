use std::sync::atomic::{AtomicBool, Ordering};
use tauri::WebviewWindow;
use windows::Win32::{
    Foundation::{HWND, LPARAM, LRESULT, RECT, WPARAM},
    UI::{
        HiDpi::GetDpiForWindow,
        Shell::{DefSubclassProc, RemoveWindowSubclass, SetWindowSubclass},
        WindowsAndMessaging::{
            GetWindowRect, WMSZ_BOTTOM, WMSZ_BOTTOMLEFT, WMSZ_BOTTOMRIGHT, WMSZ_LEFT, WMSZ_RIGHT,
            WMSZ_TOP, WMSZ_TOPLEFT, WMSZ_TOPRIGHT, WM_ENTERSIZEMOVE, WM_NCDESTROY, WM_SIZING,
        },
    },
};

const ASPECT_RATIO: f64 = 32.0 / 14.0;
const COMPACT_ASPECT_RATIO: f64 = 320.0 / 98.0;
static COMPACT_VIEW: AtomicBool = AtomicBool::new(false);
const SUBCLASS_ID: usize = 0x55535052;

pub fn set_compact_view(compact: bool) {
    COMPACT_VIEW.store(compact, Ordering::Relaxed);
}

pub fn install(window: &WebviewWindow) -> Result<(), Box<dyn std::error::Error>> {
    let hwnd = window.hwnd()?;
    let mut start = RECT::default();
    // Tauri calls setup on the window's UI thread, as required by SetWindowSubclass.
    unsafe { GetWindowRect(hwnd, &mut start)? };
    let data = Box::into_raw(Box::new(start));
    if !unsafe { SetWindowSubclass(hwnd, Some(resize_proc), SUBCLASS_ID, data as usize) }.as_bool()
    {
        unsafe { drop(Box::from_raw(data)) };
        return Err(windows::core::Error::from_thread().into());
    }
    Ok(())
}

unsafe extern "system" fn resize_proc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
    subclass_id: usize,
    data: usize,
) -> LRESULT {
    unsafe {
        let start = data as *mut RECT;
        match message {
            WM_ENTERSIZEMOVE => {
                let _ = GetWindowRect(hwnd, start);
            }
            WM_SIZING if lparam.0 != 0 => {
                let rect = &mut *(lparam.0 as *mut RECT);
                constrain(
                    rect,
                    wparam.0 as u32,
                    &*start,
                    GetDpiForWindow(hwnd).max(96),
                    if COMPACT_VIEW.load(Ordering::Relaxed) { COMPACT_ASPECT_RATIO } else { ASPECT_RATIO },
                );
                return LRESULT(1);
            }
            WM_NCDESTROY => {
                let _ = RemoveWindowSubclass(hwnd, Some(resize_proc), subclass_id);
                drop(Box::from_raw(start));
            }
            _ => {}
        }
        DefSubclassProc(hwnd, message, wparam, lparam)
    }
}

fn constrain(rect: &mut RECT, edge: u32, start: &RECT, dpi: u32, aspect_ratio: f64) {
    let proposed_width = f64::from(rect.right - rect.left);
    let proposed_height = f64::from(rect.bottom - rect.top);
    let corner = matches!(
        edge,
        WMSZ_TOPLEFT | WMSZ_TOPRIGHT | WMSZ_BOTTOMLEFT | WMSZ_BOTTOMRIGHT
    );
    let width_change = (proposed_width - f64::from(start.right - start.left)).abs();
    let height_change =
        (proposed_height - f64::from(start.bottom - start.top)).abs() * aspect_ratio;
    let height_driven =
        matches!(edge, WMSZ_TOP | WMSZ_BOTTOM) || (corner && height_change > width_change);
    let scale = f64::from(dpi) / 96.0;
    let minimum_height = scale.ceil();
    let minimum_width = scale.ceil().max(minimum_height * aspect_ratio);
    let width = if height_driven {
        proposed_height * aspect_ratio
    } else {
        proposed_width
    };
    let width = width.max(minimum_width).round() as i32;
    let height = (f64::from(width) / aspect_ratio).round() as i32;

    if matches!(edge, WMSZ_LEFT | WMSZ_TOPLEFT | WMSZ_BOTTOMLEFT) {
        rect.left = rect.right - width;
    } else {
        rect.right = rect.left + width;
    }
    if matches!(edge, WMSZ_TOP | WMSZ_TOPLEFT | WMSZ_TOPRIGHT) {
        rect.top = rect.bottom - height;
    } else {
        rect.bottom = rect.top + height;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn original() -> RECT {
        RECT {
            left: 100,
            top: 200,
            right: 420,
            bottom: 340,
        }
    }

    #[test]
    fn resizing_all_edges_keeps_ratio_and_the_opposite_anchor() {
        for edge in [
            WMSZ_LEFT,
            WMSZ_RIGHT,
            WMSZ_TOP,
            WMSZ_BOTTOM,
            WMSZ_TOPLEFT,
            WMSZ_TOPRIGHT,
            WMSZ_BOTTOMLEFT,
            WMSZ_BOTTOMRIGHT,
        ] {
            let start = original();
            let mut rect = start;
            if matches!(edge, WMSZ_LEFT | WMSZ_TOPLEFT | WMSZ_BOTTOMLEFT) {
                rect.left -= 64;
            }
            if matches!(edge, WMSZ_RIGHT | WMSZ_TOPRIGHT | WMSZ_BOTTOMRIGHT) {
                rect.right += 64;
            }
            if matches!(edge, WMSZ_TOP | WMSZ_TOPLEFT | WMSZ_TOPRIGHT) {
                rect.top -= 28;
            }
            if matches!(edge, WMSZ_BOTTOM | WMSZ_BOTTOMLEFT | WMSZ_BOTTOMRIGHT) {
                rect.bottom += 28;
            }
            constrain(&mut rect, edge, &start, 96, ASPECT_RATIO);
            assert_eq!(rect.right - rect.left, 384, "edge {edge}");
            assert_eq!(rect.bottom - rect.top, 168, "edge {edge}");
            if matches!(edge, WMSZ_LEFT | WMSZ_TOPLEFT | WMSZ_BOTTOMLEFT) {
                assert_eq!(rect.right, start.right);
            } else {
                assert_eq!(rect.left, start.left);
            }
            if matches!(edge, WMSZ_TOP | WMSZ_TOPLEFT | WMSZ_TOPRIGHT) {
                assert_eq!(rect.bottom, start.bottom);
            } else {
                assert_eq!(rect.top, start.top);
            }
        }
    }

    #[test]
    fn corners_can_resize_from_vertical_pointer_movement() {
        for change in [-28, 56] {
            let start = original();
            let mut rect = start;
            rect.bottom += change;
            constrain(&mut rect, WMSZ_BOTTOMRIGHT, &start, 96, ASPECT_RATIO);
            assert_eq!(rect.bottom - rect.top, 140 + change);
            assert_eq!(rect.right - rect.left, if change < 0 { 256 } else { 448 });
            assert_eq!((rect.left, rect.top), (start.left, start.top));
        }
    }

    #[test]
    fn minimum_size_respects_display_scaling_and_ratio() {
        for dpi in [96, 101, 120, 144, 168, 192] {
            let start = original();
            let mut rect = RECT {
                right: 100,
                bottom: 200,
                ..start
            };
            constrain(&mut rect, WMSZ_BOTTOMRIGHT, &start, dpi, ASPECT_RATIO);
            let width = f64::from(rect.right - rect.left);
            let height = f64::from(rect.bottom - rect.top);
            assert!(width >= (f64::from(dpi) / 96.0).ceil());
            assert!(height >= (f64::from(dpi) / 96.0).ceil());
            assert!((height - width / ASPECT_RATIO).abs() <= 0.5);
        }
    }

    #[test]
    fn resizing_small_default_does_not_jump_back_to_old_minimum() {
        let start = RECT {
            left: 100,
            top: 200,
            right: 292,
            bottom: 284,
        };
        for dpi in [96, 144, 192, 384] {
            let mut rect = RECT {
                right: 260,
                bottom: 270,
                ..start
            };
            constrain(&mut rect, WMSZ_BOTTOMRIGHT, &start, dpi, ASPECT_RATIO);
            assert_eq!(rect.right - rect.left, 160);
            assert_eq!(rect.bottom - rect.top, 70);
        }
    }

    #[test]
    fn compact_resizing_keeps_compact_ratio_and_window_anchor() {
        let start = RECT { left: 100, top: 200, right: 420, bottom: 298 };
        let mut horizontal = RECT { right: 500, ..start };
        constrain(&mut horizontal, WMSZ_RIGHT, &start, 144, COMPACT_ASPECT_RATIO);
        assert_eq!((horizontal.right - horizontal.left, horizontal.bottom - horizontal.top), (400, 123));
        assert_eq!((horizontal.left, horizontal.top), (100, 200));

        let mut vertical = RECT { bottom: 347, ..start };
        constrain(&mut vertical, WMSZ_BOTTOM, &start, 144, COMPACT_ASPECT_RATIO);
        assert_eq!((vertical.right - vertical.left, vertical.bottom - vertical.top), (480, 147));
        assert_eq!((vertical.left, vertical.top), (100, 200));
    }
}
