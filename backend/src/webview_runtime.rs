use std::{
    os::windows::process::CommandExt,
    process::Command,
    thread,
    time::{Duration, Instant},
};
use windows::{
    core::{w, PCWSTR},
    Win32::{
        Foundation::HWND,
        Graphics::Gdi::UpdateWindow,
        UI::WindowsAndMessaging::{
            CreateWindowExW, DestroyWindow, DispatchMessageW, GetSystemMetrics, MessageBoxW,
            PeekMessageW, TranslateMessage, IDRETRY, MB_ICONERROR, MB_RETRYCANCEL, MSG, PM_REMOVE,
            SM_CXSCREEN, SM_CYSCREEN, WINDOW_STYLE, WS_CAPTION, WS_CHILD, WS_EX_DLGMODALFRAME,
            WS_VISIBLE,
        },
    },
};

const BOOTSTRAPPER: &str = "MicrosoftEdgeWebview2Setup.exe";
const INSTALL_TIMEOUT: Duration = Duration::from_secs(600);
const DETECTION_TIMEOUT: Duration = Duration::from_secs(30);
const POLL_INTERVAL: Duration = Duration::from_millis(100);
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

trait RuntimeSetup {
    fn available(&mut self) -> bool;
    fn install(&mut self) -> Result<(), String>;
    fn retry(&mut self, error: &str) -> bool;
}

// Run before Tauri creates its event loop: it caches runtime availability on startup.
pub fn ensure_available() -> bool {
    ensure_with(&mut WindowsSetup)
}

fn ensure_with(setup: &mut impl RuntimeSetup) -> bool {
    loop {
        if setup.available() {
            return true;
        }
        let result = setup.install();
        // Recheck even after failure: another app's installer may have finished meanwhile.
        if setup.available() {
            return true;
        }
        let error = result.err().unwrap_or_else(|| {
            "Installation finished, but WebView2 is still unavailable for this Windows account."
                .to_string()
        });
        if !setup.retry(&error) {
            return false;
        }
    }
}

struct WindowsSetup;

impl RuntimeSetup for WindowsSetup {
    fn available(&mut self) -> bool {
        tauri::webview_version().is_ok()
    }

    fn install(&mut self) -> Result<(), String> {
        let executable = std::env::current_exe().map_err(|error| error.to_string())?;
        let installer = executable.with_file_name(BOOTSTRAPPER);
        if !installer.is_file() {
            return Err(
                "The WebView2 installer is missing. Reinstall UsagePin from its complete package, \
                 or install Microsoft Edge WebView2 Evergreen Runtime from \
                 https://developer.microsoft.com/microsoft-edge/webview2/."
                    .to_string(),
            );
        }

        let notice = InstallNotice::new();
        let mut child = Command::new(&installer)
            .args(["/silent", "/install"])
            .creation_flags(CREATE_NO_WINDOW)
            .spawn()
            .map_err(|error| format!("Could not start the WebView2 installer: {error}"))?;
        let started = Instant::now();
        let status = loop {
            pump(&notice);
            match child.try_wait() {
                Ok(Some(status)) => break status,
                Ok(None) => {}
                Err(error) => {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(format!(
                        "Could not wait for the WebView2 installer: {error}"
                    ));
                }
            }
            if started.elapsed() >= INSTALL_TIMEOUT {
                let _ = child.kill();
                let _ = child.wait();
                return Err("WebView2 installation timed out after 10 minutes.".to_string());
            }
            thread::sleep(POLL_INTERVAL);
        };
        if !status.success() {
            return Err(format!("The WebView2 installer exited with {status}."));
        }

        // An installer can exit before the runtime becomes discoverable. Wait briefly
        // instead of starting Tauri with a cached 'runtime missing' result.
        let detected = Instant::now();
        while !self.available() {
            if detected.elapsed() >= DETECTION_TIMEOUT {
                return Err("WebView2 was not detected after installation.".to_string());
            }
            pump(&notice);
            thread::sleep(POLL_INTERVAL);
        }
        Ok(())
    }

    fn retry(&mut self, error: &str) -> bool {
        let message: Vec<u16> = format!(
            "UsagePin needs Microsoft Edge WebView2 to display its interface.\n\n\
             {error}\n\n\
             Check your Internet connection and try again. If this is a managed PC, \
             your administrator may need to allow WebView2 installation.\n\n\
             Retry to try again, or Cancel to close UsagePin."
        )
        .encode_utf16()
        .chain(Some(0))
        .collect();
        unsafe {
            MessageBoxW(
                None,
                PCWSTR(message.as_ptr()),
                w!("UsagePin"),
                MB_RETRYCANCEL | MB_ICONERROR,
            ) == IDRETRY
        }
    }
}

// A native notice works before WebView2 is installed. No Tauri window is created here.
struct InstallNotice(HWND);

impl InstallNotice {
    fn new() -> Option<Self> {
        unsafe {
            let window = CreateWindowExW(
                WS_EX_DLGMODALFRAME,
                w!("STATIC"),
                w!("UsagePin"),
                WS_CAPTION | WS_VISIBLE,
                (GetSystemMetrics(SM_CXSCREEN) - 460) / 2,
                (GetSystemMetrics(SM_CYSCREEN) - 150) / 2,
                460,
                150,
                None,
                None,
                None,
                None,
            )
            .ok()?;
            let notice = Self(window);
            // SS_CENTER is the static-control style 0x1.
            let _ = CreateWindowExW(
                Default::default(),
                w!("STATIC"),
                w!("\nInstalling Microsoft Edge WebView2...\n\nPlease wait. This requires Internet and may take a few minutes."),
                WS_CHILD | WS_VISIBLE | WINDOW_STYLE(1),
                0,
                0,
                454,
                120,
                Some(window),
                None,
                None,
                None,
            );
            let _ = UpdateWindow(window);
            Some(notice)
        }
    }

    fn pump(&self) {
        unsafe {
            let mut message = MSG::default();
            while PeekMessageW(&mut message, Some(self.0), 0, 0, PM_REMOVE).as_bool() {
                let _ = TranslateMessage(&message);
                DispatchMessageW(&message);
            }
        }
    }
}

impl Drop for InstallNotice {
    fn drop(&mut self) {
        unsafe {
            let _ = DestroyWindow(self.0);
        }
    }
}

fn pump(notice: &Option<InstallNotice>) {
    if let Some(notice) = notice {
        notice.pump();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;

    struct FakeSetup {
        checks: VecDeque<bool>,
        installs: VecDeque<Result<(), String>>,
        retries: VecDeque<bool>,
        install_calls: usize,
        errors: Vec<String>,
    }

    impl RuntimeSetup for FakeSetup {
        fn available(&mut self) -> bool {
            self.checks.pop_front().expect("unexpected runtime check")
        }

        fn install(&mut self) -> Result<(), String> {
            self.install_calls += 1;
            self.installs.pop_front().expect("unexpected installation")
        }

        fn retry(&mut self, error: &str) -> bool {
            self.errors.push(error.to_string());
            self.retries.pop_front().expect("unexpected retry prompt")
        }
    }

    fn fake(checks: &[bool], installs: Vec<Result<(), String>>, retries: &[bool]) -> FakeSetup {
        FakeSetup {
            checks: checks.iter().copied().collect(),
            installs: installs.into(),
            retries: retries.iter().copied().collect(),
            install_calls: 0,
            errors: Vec::new(),
        }
    }

    #[test]
    fn installed_runtime_does_not_start_an_installer() {
        let mut setup = fake(&[true], vec![], &[]);
        assert!(ensure_with(&mut setup));
        assert_eq!(setup.install_calls, 0);
        assert!(setup.errors.is_empty());
    }

    #[test]
    fn missing_runtime_installs_and_is_rechecked_before_startup() {
        let mut setup = fake(&[false, true], vec![Ok(())], &[]);
        assert!(ensure_with(&mut setup));
        assert_eq!(setup.install_calls, 1);
        assert!(setup.errors.is_empty());
    }

    #[test]
    fn failed_installation_can_retry_then_start() {
        let mut setup = fake(
            &[false, false, false, true],
            vec![Err("network unavailable".into()), Ok(())],
            &[true],
        );
        assert!(ensure_with(&mut setup));
        assert_eq!(setup.install_calls, 2);
        assert_eq!(setup.errors, ["network unavailable"]);
    }

    #[test]
    fn cancel_after_failure_prevents_tauri_startup() {
        let mut setup = fake(
            &[false, false],
            vec![Err("installer missing".into())],
            &[false],
        );
        assert!(!ensure_with(&mut setup));
        assert_eq!(setup.install_calls, 1);
        assert_eq!(setup.errors, ["installer missing"]);
    }

    #[test]
    fn success_exit_without_runtime_does_not_start_tauri() {
        let mut setup = fake(&[false, false], vec![Ok(())], &[false]);
        assert!(!ensure_with(&mut setup));
        assert!(setup.errors[0].contains("still unavailable"));
    }

    #[test]
    fn concurrent_install_can_succeed_despite_installer_error() {
        let mut setup = fake(
            &[false, true],
            vec![Err("another installer is running".into())],
            &[],
        );
        assert!(ensure_with(&mut setup));
        assert!(setup.errors.is_empty());
    }
}
