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
    HOTKEYS.iter().position(|(name, _)| *name == label).unwrap_or(3)
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
                std::slice::from_raw_parts(
                    command.as_ptr() as *const u8,
                    (command.len() + 1) * 2,
                )
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
