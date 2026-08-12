// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Talkdedsec

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::color::Settings;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Profile {
    pub name: String,
    pub settings: Settings,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Store {
    pub last: Settings,
    pub auto_apply: bool,
    pub hotkey: String,
    pub tray_on_close: bool,
    pub profiles: Vec<Profile>,
}

impl Default for Store {
    fn default() -> Self {
        Self {
            last: Settings::default(),
            auto_apply: true,
            hotkey: "F9".to_string(),
            tray_on_close: true,
            profiles: Vec::new(),
        }
    }
}

impl Store {
    pub fn load() -> Self {
        let Some(path) = config_path() else {
            return Self::default();
        };
        std::fs::read_to_string(path)
            .ok()
            .and_then(|text| serde_json::from_str::<Self>(&text).ok())
            .map(|mut store| {
                store.last = store.last.clamped();
                for profile in &mut store.profiles {
                    profile.settings = profile.settings.clamped();
                }
                store
            })
            .unwrap_or_default()
    }

    pub fn save(&self) {
        let Some(path) = config_path() else {
            return;
        };
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        if let Ok(text) = serde_json::to_string_pretty(self) {
            let _ = std::fs::write(path, text);
        }
    }

    pub fn names(&self) -> Vec<slint::SharedString> {
        self.profiles.iter().map(|p| p.name.as_str().into()).collect()
    }

    /// Saving under an existing name overwrites it, so repeated saves do not
    /// pile up duplicates.
    pub fn upsert(&mut self, name: &str, settings: Settings) -> bool {
        let name = name.trim();
        if name.is_empty() {
            return false;
        }
        match self.profiles.iter_mut().find(|p| p.name == name) {
            Some(existing) => existing.settings = settings,
            None => self.profiles.push(Profile {
                name: name.to_string(),
                settings,
            }),
        }
        self.save();
        true
    }

    pub fn remove(&mut self, index: usize) -> bool {
        if index >= self.profiles.len() {
            return false;
        }
        self.profiles.remove(index);
        self.save();
        true
    }

    pub fn export_to(&self, path: &Path) -> std::io::Result<usize> {
        let text = serde_json::to_string_pretty(&self.profiles)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        std::fs::write(path, text)?;
        Ok(self.profiles.len())
    }

    pub fn import_from(&mut self, path: &Path) -> std::io::Result<usize> {
        let text = std::fs::read_to_string(path)?;
        let incoming: Vec<Profile> = serde_json::from_str(&text)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        let mut added = 0;
        for profile in incoming {
            let name = profile.name.trim();
            if name.is_empty() {
                continue;
            }
            let settings = profile.settings.clamped();
            match self.profiles.iter_mut().find(|p| p.name == name) {
                Some(existing) => existing.settings = settings,
                None => {
                    self.profiles.push(Profile {
                        name: name.to_string(),
                        settings,
                    });
                }
            }
            added += 1;
        }
        self.save();
        Ok(added)
    }
}

fn config_path() -> Option<PathBuf> {
    let base = std::env::var_os("APPDATA")?;
    Some(
        PathBuf::from(base)
            .join("Talkdedsec")
            .join("Visual")
            .join("config.json"),
    )
}
