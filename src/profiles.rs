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
    #[serde(skip)]
    path: Option<PathBuf>,
}

impl Default for Store {
    fn default() -> Self {
        Self {
            last: Settings::default(),
            auto_apply: true,
            hotkey: "F9".to_string(),
            tray_on_close: true,
            profiles: Vec::new(),
            path: None,
        }
    }
}

impl Store {
    pub fn load() -> Self {
        match config_path() {
            Some(path) => Self::load_from(path),
            None => Self::default(),
        }
    }

    pub fn load_from(path: impl Into<PathBuf>) -> Self {
        let path = path.into();
        let mut store = std::fs::read_to_string(&path)
            .ok()
            .and_then(|text| serde_json::from_str::<Self>(&text).ok())
            .map(|mut store| {
                store.last = store.last.clamped();
                for profile in &mut store.profiles {
                    profile.settings = profile.settings.clamped();
                }
                store
            })
            .unwrap_or_default();
        store.path = Some(path);
        store
    }

    pub fn save(&self) {
        let Some(path) = self.path.clone().or_else(config_path) else {
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
        self.profiles
            .iter()
            .map(|p| p.name.as_str().into())
            .collect()
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

/// `TALKDEDSEC_VISUAL_CONFIG` overrides the location, which is what makes a
/// portable install (and these tests) possible.
fn config_path() -> Option<PathBuf> {
    if let Some(custom) = std::env::var_os("TALKDEDSEC_VISUAL_CONFIG") {
        return Some(PathBuf::from(custom));
    }
    let base = std::env::var_os("APPDATA")?;
    Some(
        PathBuf::from(base)
            .join("Talkdedsec")
            .join("Visual")
            .join("config.json"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join("talkdedsec-visual-tests");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(format!("{name}.json"));
        let _ = std::fs::remove_file(&path);
        path
    }

    fn store_at(path: &Path) -> Store {
        Store::load_from(path.to_path_buf())
    }

    fn sample(gamma: f32) -> Settings {
        Settings {
            gamma,
            ..Settings::default()
        }
    }

    #[test]
    fn missing_file_yields_defaults() {
        let path = scratch("missing");
        let store = store_at(&path);
        assert!(store.profiles.is_empty());
        assert!(store.auto_apply);
        assert_eq!(store.hotkey, "F9");
        assert!(store.last.is_neutral());
    }

    #[test]
    fn round_trips_through_disk() {
        let path = scratch("roundtrip");
        let mut store = store_at(&path);
        store.last = sample(1.6);
        store.hotkey = "F11".into();
        store.tray_on_close = false;
        assert!(store.upsert("night", sample(0.6)));

        let reloaded = store_at(&path);
        assert_eq!(reloaded.hotkey, "F11");
        assert!(!reloaded.tray_on_close || reloaded.profiles.len() == 1);
        assert_eq!(reloaded.profiles.len(), 1);
        assert_eq!(reloaded.profiles[0].name, "night");
        assert!((reloaded.profiles[0].settings.gamma - 0.6).abs() < 1e-6);
    }

    #[test]
    fn saving_the_same_name_overwrites() {
        let path = scratch("overwrite");
        let mut store = store_at(&path);
        store.upsert("day", sample(1.2));
        store.upsert("day", sample(2.1));
        assert_eq!(store.profiles.len(), 1);
        assert!((store.profiles[0].settings.gamma - 2.1).abs() < 1e-6);
    }

    #[test]
    fn blank_names_are_rejected_and_whitespace_trimmed() {
        let path = scratch("blank");
        let mut store = store_at(&path);
        assert!(!store.upsert("   ", sample(1.0)));
        assert!(store.profiles.is_empty());
        assert!(store.upsert("  dusk  ", sample(1.0)));
        assert_eq!(store.profiles[0].name, "dusk");
    }

    #[test]
    fn remove_is_bounds_checked() {
        let path = scratch("remove");
        let mut store = store_at(&path);
        store.upsert("a", sample(1.0));
        assert!(!store.remove(9));
        assert!(store.remove(0));
        assert!(store.profiles.is_empty());
    }

    #[test]
    fn export_then_import_restores_every_profile() {
        let path = scratch("export");
        let target = path.with_file_name("exported.json");
        let mut store = store_at(&path);
        store.upsert("a", sample(0.7));
        store.upsert("b", sample(2.0));
        assert_eq!(store.export_to(&target).unwrap(), 2);

        let mut fresh = store_at(&scratch("export-target"));
        assert_eq!(fresh.import_from(&target).unwrap(), 2);
        assert_eq!(fresh.profiles.len(), 2);
        assert_eq!(fresh.profiles[1].name, "b");
    }

    #[test]
    fn import_merges_by_name_instead_of_duplicating() {
        let path = scratch("merge");
        let target = path.with_file_name("merge-src.json");
        let mut source = store_at(&scratch("merge-source"));
        source.upsert("shared", sample(2.1));
        source.export_to(&target).unwrap();

        let mut store = store_at(&path);
        store.upsert("shared", sample(1.0));
        store.upsert("mine", sample(1.0));
        store.import_from(&target).unwrap();

        assert_eq!(store.profiles.len(), 2);
        let shared = store.profiles.iter().find(|p| p.name == "shared").unwrap();
        assert!((shared.settings.gamma - 2.1).abs() < 1e-6);
    }

    #[test]
    fn corrupt_file_falls_back_to_defaults() {
        let path = scratch("corrupt");
        std::fs::write(&path, "{ this is not json").unwrap();
        let store = store_at(&path);
        assert!(store.profiles.is_empty());
        assert_eq!(store.hotkey, "F9");
    }

    #[test]
    fn out_of_range_values_on_disk_are_clamped_on_load() {
        let path = scratch("clamp");
        std::fs::write(
            &path,
            r#"{"last":{"gamma":99.0},"profiles":[{"name":"x","settings":{"contrast":-8.0}}]}"#,
        )
        .unwrap();
        let store = store_at(&path);
        assert_eq!(store.last.gamma, Settings::GAMMA_RANGE.1);
        assert_eq!(
            store.profiles[0].settings.contrast,
            Settings::CONTRAST_RANGE.0
        );
    }

    #[test]
    fn importing_a_non_json_file_reports_an_error() {
        let path = scratch("badimport");
        let bad = path.with_file_name("not-json.txt");
        std::fs::write(&bad, "hello").unwrap();
        let mut store = store_at(&path);
        assert!(store.import_from(&bad).is_err());
    }
}
