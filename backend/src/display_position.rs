use tauri::WebviewWindow;
use windows::{
    core::BOOL,
    Win32::{
        Foundation::{HWND, LPARAM, LRESULT, RECT, WPARAM},
        Graphics::Gdi::{
            EnumDisplayMonitors, GetMonitorInfoW, MonitorFromWindow, HDC, HMONITOR, MONITORINFOEXW,
            MONITOR_DEFAULTTONEAREST,
        },
        UI::{
            Shell::{DefSubclassProc, RemoveWindowSubclass, SetWindowSubclass},
            WindowsAndMessaging::{
                GetWindowRect, KillTimer, SetTimer, SetWindowPos, SWP_NOACTIVATE, SWP_NOSIZE,
                SWP_NOZORDER, WM_DISPLAYCHANGE, WM_ENTERSIZEMOVE, WM_EXITSIZEMOVE, WM_NCDESTROY,
                WM_TIMER, WM_WINDOWPOSCHANGED,
            },
        },
    },
};

const SUBCLASS_ID: usize = 0x55535050;
const RESTORE_TIMER_ID: usize = SUBCLASS_ID;
const DISPLAY_SETTLE_MS: u32 = 500;

#[derive(Clone, Copy, Debug, PartialEq)]
struct MonitorLayout {
    device: [u16; 32],
    bounds: RECT,
}

#[derive(Clone, Copy, Debug)]
struct SavedPosition {
    position: (i32, i32),
    monitor: MonitorLayout,
}

impl SavedPosition {
    fn restore_target(&self, monitors: &[MonitorLayout]) -> Option<(i32, i32)> {
        // Keep the original anchor while a game uses a temporary display mode.
        monitors.contains(&self.monitor).then_some(self.position)
    }
}

struct PositionState {
    saved: SavedPosition,
    interaction_start: Option<(i32, i32)>,
    restore_pending: bool,
}

impl PositionState {
    fn finish_interaction(&mut self, current: SavedPosition) {
        if let Some(start) = self.interaction_start.take() {
            // A click or cancelled drag must not save Windows' automatic relocation.
            if current.position != start {
                self.saved = current;
            }
        }
    }
}

fn monitor_layout(monitor: HMONITOR) -> Result<MonitorLayout, windows::core::Error> {
    let mut info = MONITORINFOEXW::default();
    info.monitorInfo.cbSize = std::mem::size_of::<MONITORINFOEXW>() as u32;
    unsafe { GetMonitorInfoW(monitor, &mut info.monitorInfo) }.ok()?;
    Ok(MonitorLayout {
        device: info.szDevice,
        bounds: info.monitorInfo.rcMonitor,
    })
}

fn capture(hwnd: HWND) -> Result<SavedPosition, windows::core::Error> {
    let mut rect = RECT::default();
    unsafe { GetWindowRect(hwnd, &mut rect)? };
    let monitor = unsafe { MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST) };
    Ok(SavedPosition {
        position: (rect.left, rect.top),
        monitor: monitor_layout(monitor)?,
    })
}

unsafe extern "system" fn collect_monitor(
    monitor: HMONITOR,
    _hdc: HDC,
    _rect: *mut RECT,
    data: LPARAM,
) -> BOOL {
    if let Ok(layout) = monitor_layout(monitor) {
        unsafe { &mut *(data.0 as *mut Vec<MonitorLayout>) }.push(layout);
    }
    BOOL(1)
}

fn monitor_layouts() -> Result<Vec<MonitorLayout>, windows::core::Error> {
    let mut monitors = Vec::new();
    unsafe {
        EnumDisplayMonitors(
            None,
            None,
            Some(collect_monitor),
            LPARAM(&mut monitors as *mut Vec<MonitorLayout> as isize),
        )
    }
    .ok()?;
    Ok(monitors)
}

pub fn install(window: &WebviewWindow) -> Result<(), Box<dyn std::error::Error>> {
    install_hwnd(window.hwnd()?)?;
    Ok(())
}

fn install_hwnd(hwnd: HWND) -> Result<(), windows::core::Error> {
    let state = Box::new(PositionState {
        saved: capture(hwnd)?,
        interaction_start: None,
        restore_pending: false,
    });
    let data = Box::into_raw(state);
    // Setup runs on the window's UI thread, as required by SetWindowSubclass.
    if !unsafe { SetWindowSubclass(hwnd, Some(position_proc), SUBCLASS_ID, data as usize) }
        .as_bool()
    {
        unsafe { drop(Box::from_raw(data)) };
        return Err(windows::core::Error::from_thread());
    }
    Ok(())
}

unsafe extern "system" fn position_proc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
    subclass_id: usize,
    data: usize,
) -> LRESULT {
    unsafe {
        let state = data as *mut PositionState;
        match message {
            WM_ENTERSIZEMOVE => {
                let _ = KillTimer(Some(hwnd), RESTORE_TIMER_ID);
                let mut rect = RECT::default();
                if GetWindowRect(hwnd, &mut rect).is_ok() {
                    (*state).interaction_start = Some((rect.left, rect.top));
                }
            }
            WM_EXITSIZEMOVE => {
                match capture(hwnd) {
                    Ok(current) => (*state).finish_interaction(current),
                    Err(error) => {
                        (*state).interaction_start = None;
                        eprintln!("Could not save widget position: {error}");
                    }
                }
                if (*state).restore_pending {
                    SetTimer(Some(hwnd), RESTORE_TIMER_ID, DISPLAY_SETTLE_MS, None);
                }
            }
            WM_DISPLAYCHANGE => {
                (*state).restore_pending = true;
                SetTimer(Some(hwnd), RESTORE_TIMER_ID, DISPLAY_SETTLE_MS, None);
            }
            WM_WINDOWPOSCHANGED
                if (*state).restore_pending && (*state).interaction_start.is_none() =>
            {
                // Wait until Windows finishes relocating the window after the mode switch.
                SetTimer(Some(hwnd), RESTORE_TIMER_ID, DISPLAY_SETTLE_MS, None);
            }
            WM_TIMER if wparam.0 == RESTORE_TIMER_ID => {
                if (*state).restore_pending && (*state).interaction_start.is_none() {
                    let _ = KillTimer(Some(hwnd), RESTORE_TIMER_ID);
                    (*state).restore_pending = false;
                    match monitor_layouts() {
                        Ok(monitors) => {
                            let target = (*state).saved.restore_target(&monitors);
                            if let Some((x, y)) = target {
                                // Do not hold a state reference across reentrant window messages.
                                if let Err(error) = SetWindowPos(
                                    hwnd,
                                    None,
                                    x,
                                    y,
                                    0,
                                    0,
                                    SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
                                ) {
                                    eprintln!("Could not restore widget position: {error}");
                                }
                            }
                        }
                        Err(error) => eprintln!("Could not read display layout: {error}"),
                    }
                }
                return LRESULT(0);
            }
            WM_NCDESTROY => {
                let _ = KillTimer(Some(hwnd), RESTORE_TIMER_ID);
                let _ = RemoveWindowSubclass(hwnd, Some(position_proc), subclass_id);
                drop(Box::from_raw(state));
            }
            _ => {}
        }
        DefSubclassProc(hwnd, message, wparam, lparam)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use windows::{
        core::w,
        Win32::UI::WindowsAndMessaging::{
            CreateWindowExW, DestroyWindow, SendMessageW, WINDOW_EX_STYLE, WS_POPUP,
        },
    };

    fn monitor(device: u16, x: i32, width: i32, height: i32) -> MonitorLayout {
        let mut name = [0; 32];
        name[0] = device;
        MonitorLayout {
            device: name,
            bounds: RECT {
                left: x,
                top: 0,
                right: x + width,
                bottom: height,
            },
        }
    }

    #[test]
    fn game_resolution_round_trip_restores_the_original_anchor() {
        let desktop = monitor(1, 0, 1920, 1080);
        let saved = SavedPosition {
            position: (1712, 64),
            monitor: desktop,
        };
        for game in [monitor(1, 0, 1280, 720), monitor(1, 0, 1024, 768)] {
            assert_eq!(saved.restore_target(&[game]), None);
            assert_eq!(saved.restore_target(&[desktop]), Some((1712, 64)));
        }
    }

    #[test]
    fn monitor_identity_and_negative_origin_survive_disconnect_and_reconnect() {
        let secondary = monitor(2, -1920, 1920, 1080);
        let primary = monitor(1, 0, 1920, 1080);
        let saved = SavedPosition {
            position: (-400, 120),
            monitor: secondary,
        };
        assert_eq!(saved.restore_target(&[primary]), None);
        assert_eq!(saved.restore_target(&[monitor(3, -1920, 1920, 1080)]), None);
        assert_eq!(
            saved.restore_target(&[primary, secondary]),
            Some((-400, 120))
        );
    }

    #[test]
    fn cancelled_drag_keeps_the_anchor_but_deliberate_drag_replaces_it() {
        let desktop = monitor(1, 0, 1920, 1080);
        let game = monitor(1, 0, 1280, 720);
        let mut state = PositionState {
            saved: SavedPosition {
                position: (1712, 64),
                monitor: desktop,
            },
            interaction_start: Some((1072, 64)),
            restore_pending: false,
        };
        state.finish_interaction(SavedPosition {
            position: (1072, 64),
            monitor: game,
        });
        assert_eq!(state.saved.restore_target(&[desktop]), Some((1712, 64)));
        state.interaction_start = Some((1072, 64));
        state.finish_interaction(SavedPosition {
            position: (600, 200),
            monitor: game,
        });
        assert_eq!(state.saved.restore_target(&[game]), Some((600, 200)));
        assert_eq!(state.saved.restore_target(&[desktop]), None);
    }

    #[test]
    fn native_display_messages_restore_position_and_respect_user_drag() {
        struct TestWindow(HWND);
        impl Drop for TestWindow {
            fn drop(&mut self) {
                unsafe {
                    let _ = DestroyWindow(self.0);
                }
            }
        }
        unsafe {
            // Use a hidden window: never change the user's actual display mode or widget.
            let hwnd = CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                w!("STATIC"),
                w!("UsagePin position test"),
                WS_POPUP,
                100,
                100,
                192,
                84,
                None,
                None,
                None,
                None,
            )
            .unwrap();
            let _window = TestWindow(hwnd);
            install_hwnd(hwnd).unwrap();
            let original = capture(hwnd).unwrap().position;
            let relocate = |x, y| {
                SetWindowPos(
                    hwnd,
                    None,
                    x,
                    y,
                    0,
                    0,
                    SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
                )
                .unwrap();
            };
            let display_change = || {
                SendMessageW(hwnd, WM_DISPLAYCHANGE, None, None);
            };
            let settle = || {
                SendMessageW(hwnd, WM_TIMER, Some(WPARAM(RESTORE_TIMER_ID)), None);
            };

            display_change();
            relocate(original.0 + 80, original.1 + 40);
            settle();
            assert_eq!(capture(hwnd).unwrap().position, original);

            SendMessageW(hwnd, WM_ENTERSIZEMOVE, None, None);
            let user_position = (original.0 + 240, original.1 + 160);
            relocate(user_position.0, user_position.1);
            SendMessageW(hwnd, WM_EXITSIZEMOVE, None, None);
            display_change();
            relocate(original.0, original.1);
            SendMessageW(hwnd, WM_ENTERSIZEMOVE, None, None);
            settle();
            assert_eq!(capture(hwnd).unwrap().position, original);
            SendMessageW(hwnd, WM_EXITSIZEMOVE, None, None);
            settle();
            assert_eq!(capture(hwnd).unwrap().position, user_position);
            let mut rect = RECT::default();
            GetWindowRect(hwnd, &mut rect).unwrap();
            assert_eq!((rect.right - rect.left, rect.bottom - rect.top), (192, 84));
        }
    }
}
