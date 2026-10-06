// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Talkdedsec

use global_hotkey::hotkey::{Code, HotKey};
use global_hotkey::GlobalHotKeyManager;

pub const RUN_VALUE: &str = "TalkdedsecVisual";

pub const HOTKEYS: &[(&str, Code)] = &[
    ("F6", Code::F6),
    ("F7", Code::F7),
    ("F8", Code::F8),
    ("F9", Code::F9),
    ("F10", Code::F10),
    ("F11", Code::F11),
    ("F12", Code::F12),
];

pub fn hotkey_index(label: &str) -> usize {
    HOTKEYS
        .iter()
        .position(|(name, _)| *name == label)
        .unwrap_or(3)
}

pub struct Hotkeys {
    manager: GlobalHotKeyManager,
    current: Option<HotKey>,
}

impl Hotkeys {
    pub fn new() -> Option<Self> {
        GlobalHotKeyManager::new().ok().map(|manager| Self {
            manager,
            current: None,
        })
    }

    pub fn id(&self) -> Option<u32> {
        self.current.map(|k| k.id())
    }

    pub fn bind(&mut self, label: &str) -> bool {
        if let Some(previous) = self.current.take() {
            let _ = self.manager.unregister(previous);
        }
        let Some((_, code)) = HOTKEYS.iter().find(|(name, _)| *name == label) else {
            return false;
        };
        let key = HotKey::new(None, *code);
        match self.manager.register(key) {
            Ok(()) => {
                self.current = Some(key);
                true
            }
            Err(_) => false,
        }
    }
}

#[cfg(windows)]
mod registry {
    use super::RUN_VALUE;
    use windows::core::{w, HSTRING, PCWSTR};
    use windows::Win32::System::Registry::{
        RegCloseKey, RegDeleteValueW, RegOpenKeyExW, RegQueryValueExW, RegSetValueExW, HKEY,
        HKEY_CURRENT_USER, KEY_READ, KEY_WRITE, REG_SZ,
    };

    fn open(access: u32) -> Option<HKEY> {
        let mut key = HKEY::default();
        let result = unsafe {
            RegOpenKeyExW(
                HKEY_CURRENT_USER,
                w!("Software\\Microsoft\\Windows\\CurrentVersion\\Run"),
                None,
                windows::Win32::System::Registry::REG_SAM_FLAGS(access),
                &mut key,
            )
        };
        result.is_ok().then_some(key)
    }

    pub fn enabled() -> bool {
        let Some(key) = open(KEY_READ.0) else {
            return false;
        };
        let name = HSTRING::from(RUN_VALUE);
        let mut size = 0u32;
        let result = unsafe {
            RegQueryValueExW(
                key,
                PCWSTR(name.as_ptr()),
                None,
                None,
                None,
                Some(&mut size),
            )
        };
        unsafe {
            let _ = RegCloseKey(key);
        }
        result.is_ok() && size > 0
    }

    pub fn set(on: bool) -> bool {
        let Some(key) = open(KEY_WRITE.0) else {
            return false;
        };
        let name = HSTRING::from(RUN_VALUE);
        let ok = if on {
            let Ok(exe) = std::env::current_exe() else {
                unsafe {
                    let _ = RegCloseKey(key);
                }
                return false;
            };
            let command = HSTRING::from(format!("\"{}\" --tray", exe.display()));
            let bytes: &[u8] = unsafe {
                std::slice::from_raw_parts(command.as_ptr() as *const u8, (command.len() + 1) * 2)
            };
            unsafe { RegSetValueExW(key, PCWSTR(name.as_ptr()), None, REG_SZ, Some(bytes)) }.is_ok()
        } else {
            unsafe { RegDeleteValueW(key, PCWSTR(name.as_ptr())) }.is_ok()
        };
        unsafe {
            let _ = RegCloseKey(key);
        }
        ok
    }
}

#[cfg(not(windows))]
mod registry {
    pub fn enabled() -> bool {
        false
    }
    pub fn set(_: bool) -> bool {
        false
    }
}

pub use registry::{enabled as autostart_enabled, set as set_autostart};

/// One copy at a time. A second launch hands over to the first, which shows its
/// window, instead of putting a second icon in the tray and a second hand on the
/// gamma ramp.
pub enum Instance {
    First(Summons),
    Already,
}

/// Raised by a later launch asking this copy to show itself.
pub struct Summons(#[cfg(windows)] Option<windows::Win32::Foundation::HANDLE>);

#[cfg(windows)]
pub fn claim() -> Instance {
    use windows::core::w;
    use windows::Win32::Foundation::{CloseHandle, GetLastError, ERROR_ALREADY_EXISTS};
    use windows::Win32::System::Threading::{CreateEventW, SetEvent};
    use windows::Win32::UI::WindowsAndMessaging::{AllowSetForegroundWindow, ASFW_ANY};

    // Auto-reset, so each launch is answered exactly once.
    let Ok(event) =
        (unsafe { CreateEventW(None, false, false, w!("Local\\TalkdedsecVisual.Show")) })
    else {
        return Instance::First(Summons(None));
    };
    if unsafe { GetLastError() } != ERROR_ALREADY_EXISTS {
        return Instance::First(Summons(Some(event)));
    }
    unsafe {
        // This launch came from a click, so it may pass the foreground on.
        let _ = AllowSetForegroundWindow(ASFW_ANY);
        let _ = SetEvent(event);
        let _ = CloseHandle(event);
    }
    Instance::Already
}

#[cfg(not(windows))]
pub fn claim() -> Instance {
    Instance::First(Summons())
}

impl Summons {
    /// True once for each later launch since the last look.
    pub fn raised(&self) -> bool {
        #[cfg(windows)]
        {
            use windows::Win32::Foundation::WAIT_OBJECT_0;
            use windows::Win32::System::Threading::WaitForSingleObject;

            self.0
                .is_some_and(|event| unsafe { WaitForSingleObject(event, 0) } == WAIT_OBJECT_0)
        }
        #[cfg(not(windows))]
        false
    }
}

#[cfg(windows)]
impl Drop for Summons {
    fn drop(&mut self) {
        if let Some(event) = self.0 {
            unsafe {
                let _ = windows::Win32::Foundation::CloseHandle(event);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hotkey_table_is_unique_and_ordered() {
        let names: Vec<&str> = HOTKEYS.iter().map(|(n, _)| *n).collect();
        let mut sorted = names.clone();
        sorted.dedup();
        assert_eq!(names.len(), sorted.len(), "duplicate hotkey label");
        assert_eq!(names.first(), Some(&"F6"));
        assert_eq!(names.last(), Some(&"F12"));
    }

    #[test]
    fn hotkey_index_round_trips_and_falls_back_to_f9() {
        for (i, (name, _)) in HOTKEYS.iter().enumerate() {
            assert_eq!(hotkey_index(name), i);
        }
        assert_eq!(hotkey_index("F9"), 3);
        assert_eq!(hotkey_index("nonsense"), 3);
        assert_eq!(HOTKEYS[hotkey_index("nonsense")].0, "F9");
    }

    #[cfg(windows)]
    #[test]
    fn autostart_writes_and_removes_the_run_value() {
        let original = autostart_enabled();

        if !set_autostart(true) {
            // Hosted runners and locked-down profiles refuse HKCU\...\Run outright.
            // Nothing to assert about a round trip that the environment will not allow.
            eprintln!("skipped: this environment does not permit writing the Run value");
            return;
        }
        assert!(autostart_enabled(), "Run value missing after enabling");

        assert!(set_autostart(false), "could not delete the Run value");
        assert!(!autostart_enabled(), "Run value survived deletion");

        if original {
            set_autostart(true);
        }
        assert_eq!(
            autostart_enabled(),
            original,
            "test left the registry dirty"
        );
    }
}
