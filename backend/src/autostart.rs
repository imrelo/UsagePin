use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager, State};

pub struct AutostartState(Mutex<Result<bool, String>>);

pub fn initialize(app: &AppHandle) {
    let result = platform::initialize();
    if let Err(error) = &result {
        eprintln!("{error}");
    }
    app.manage(AutostartState(Mutex::new(result)));
}

#[tauri::command]
pub fn get_start_with_windows(state: State<'_, AutostartState>) -> Result<bool, String> {
    state.0.lock().map_err(|error| error.to_string())?.clone()
}

#[tauri::command]
pub fn set_start_with_windows(
    app: AppHandle,
    state: State<'_, AutostartState>,
    enabled: bool,
) -> Result<bool, String> {
    let mut current = state.0.lock().map_err(|error| error.to_string())?;
    platform::set(enabled)?;
    *current = Ok(enabled);
    let _ = app.emit("start-with-windows-changed", enabled);
    Ok(enabled)
}

#[cfg(windows)]
mod platform {
    use std::{os::windows::ffi::OsStrExt, path::Path};
    use windows::{
        core::{w, PCWSTR},
        Win32::{
            Foundation::{ERROR_FILE_NOT_FOUND, WIN32_ERROR},
            System::Registry::{
                RegDeleteKeyValueW, RegGetValueW, RegSetKeyValueW, HKEY_CURRENT_USER, REG_DWORD,
                REG_SZ, RRF_RT_REG_DWORD,
            },
        },
    };

    const PREFERENCE_KEY: PCWSTR = w!("Software\\UsagePin");
    const RUN_KEY: PCWSTR = w!("Software\\Microsoft\\Windows\\CurrentVersion\\Run");
    const PREFERENCE_NAME: PCWSTR = w!("StartWithWindows");
    const RUN_NAME: PCWSTR = w!("UsagePin");

    fn check(result: WIN32_ERROR) -> Result<(), String> {
        result
            .ok()
            .map_err(|error| format!("Could not update Windows startup: {error}"))
    }

    fn preference(key: PCWSTR) -> Result<Option<bool>, String> {
        let mut value = 0u32;
        let mut size = std::mem::size_of_val(&value) as u32;
        let result = unsafe {
            RegGetValueW(
                HKEY_CURRENT_USER,
                key,
                PREFERENCE_NAME,
                RRF_RT_REG_DWORD,
                None,
                Some((&mut value as *mut u32).cast()),
                Some(&mut size),
            )
        };
        if result == ERROR_FILE_NOT_FOUND {
            return Ok(None);
        }
        check(result)?;
        Ok(Some(value != 0))
    }

    fn save_preference(key: PCWSTR, value: Option<bool>) -> Result<(), String> {
        let result = if let Some(enabled) = value {
            let value = u32::from(enabled);
            unsafe {
                RegSetKeyValueW(
                    HKEY_CURRENT_USER,
                    key,
                    PREFERENCE_NAME,
                    REG_DWORD.0,
                    Some((&value as *const u32).cast()),
                    std::mem::size_of_val(&value) as u32,
                )
            }
        } else {
            unsafe { RegDeleteKeyValueW(HKEY_CURRENT_USER, key, PREFERENCE_NAME) }
        };
        if result == ERROR_FILE_NOT_FOUND {
            return Ok(());
        }
        check(result)
    }

    fn startup_command(executable: &Path) -> Result<Vec<u16>, String> {
        let mut command = vec![b'"' as u16];
        command.extend(executable.as_os_str().encode_wide());
        command.push(b'"' as u16);
        if command.len() > 260 {
            return Err("Move UsagePin to a shorter path to enable Windows startup.".to_owned());
        }
        command.push(0);
        Ok(command)
    }

    fn set_with(keys: (PCWSTR, PCWSTR), enabled: bool, executable: &Path) -> Result<bool, String> {
        let previous = preference(keys.0)?;
        let command = if enabled {
            Some(startup_command(executable)?)
        } else {
            None
        };
        save_preference(keys.0, Some(enabled))?;
        let result = if let Some(command) = command {
            unsafe {
                RegSetKeyValueW(
                    HKEY_CURRENT_USER,
                    keys.1,
                    RUN_NAME,
                    REG_SZ.0,
                    Some(command.as_ptr().cast()),
                    (command.len() * 2) as u32,
                )
            }
        } else {
            unsafe { RegDeleteKeyValueW(HKEY_CURRENT_USER, keys.1, RUN_NAME) }
        };
        if result != ERROR_FILE_NOT_FOUND || enabled {
            if let Err(error) = check(result) {
                save_preference(keys.0, previous)
                    .map_err(|rollback| format!("{error} {rollback}"))?;
                return Err(error);
            }
        }
        Ok(enabled)
    }

    fn initialize_with(keys: (PCWSTR, PCWSTR), executable: &Path) -> Result<bool, String> {
        set_with(keys, preference(keys.0)?.unwrap_or(true), executable)
    }

    pub fn initialize() -> Result<bool, String> {
        let executable = std::env::current_exe().map_err(|error| error.to_string())?;
        initialize_with((PREFERENCE_KEY, RUN_KEY), &executable)
    }

    pub fn set(enabled: bool) -> Result<(), String> {
        let executable = std::env::current_exe().map_err(|error| error.to_string())?;
        set_with((PREFERENCE_KEY, RUN_KEY), enabled, &executable).map(|_| ())
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use windows::Win32::System::Registry::{RegDeleteTreeW, RRF_RT_REG_SZ};

        #[test]
        fn default_toggle_restart_and_moved_executable_use_persistent_registration() {
            let key: Vec<u16> = format!("Software\\UsagePin\\Test-{}", std::process::id())
                .encode_utf16()
                .chain(std::iter::once(0))
                .collect();
            let key = PCWSTR(key.as_ptr());
            struct Cleanup(PCWSTR);
            impl Drop for Cleanup {
                fn drop(&mut self) {
                    unsafe {
                        let _ = RegDeleteTreeW(HKEY_CURRENT_USER, self.0);
                    }
                }
            }
            let _cleanup = Cleanup(key);
            unsafe {
                let _ = RegDeleteTreeW(HKEY_CURRENT_USER, key);
            }
            let keys = (key, key);
            let first = Path::new("C:\\My Apps\\UsagePin.exe");
            assert_eq!(preference(key).unwrap(), None);
            assert!(initialize_with(keys, first).unwrap());
            assert_eq!(preference(key).unwrap(), Some(true));
            let mut command = [0u16; 261];
            let mut size = std::mem::size_of_val(&command) as u32;
            check(unsafe {
                RegGetValueW(
                    HKEY_CURRENT_USER,
                    key,
                    RUN_NAME,
                    RRF_RT_REG_SZ,
                    None,
                    Some(command.as_mut_ptr().cast()),
                    Some(&mut size),
                )
            })
            .unwrap();
            assert_eq!(
                String::from_utf16(&command[..size as usize / 2 - 1]).unwrap(),
                "\"C:\\My Apps\\UsagePin.exe\""
            );
            assert!(!set_with(keys, false, first).unwrap());
            assert!(!initialize_with(keys, first).unwrap());
            assert_eq!(preference(key).unwrap(), Some(false));
            assert_eq!(
                unsafe {
                    RegGetValueW(
                        HKEY_CURRENT_USER,
                        key,
                        RUN_NAME,
                        RRF_RT_REG_SZ,
                        None,
                        None,
                        None,
                    )
                },
                ERROR_FILE_NOT_FOUND
            );
            let moved = Path::new("C:\\Apps\\Ứng dụng\\UsagePin.exe");
            assert!(set_with(keys, true, first).unwrap());
            assert!(initialize_with(keys, moved).unwrap());
            size = std::mem::size_of_val(&command) as u32;
            check(unsafe {
                RegGetValueW(
                    HKEY_CURRENT_USER,
                    key,
                    RUN_NAME,
                    RRF_RT_REG_SZ,
                    None,
                    Some(command.as_mut_ptr().cast()),
                    Some(&mut size),
                )
            })
            .unwrap();
            assert_eq!(
                command[..size as usize / 2],
                startup_command(moved).unwrap()
            );
            assert!(!set_with(keys, false, first).unwrap());
            let too_long = format!("C:\\{}\\UsagePin.exe", "a".repeat(260));
            assert!(set_with(keys, true, Path::new(&too_long)).is_err());
            assert_eq!(preference(key).unwrap(), Some(false));
        }
    }
}

#[cfg(not(windows))]
mod platform {
    pub fn initialize() -> Result<bool, String> {
        Err("Start with Windows is only available on Windows.".to_owned())
    }
    pub fn set(_: bool) -> Result<(), String> {
        initialize().map(|_| ())
    }
}
