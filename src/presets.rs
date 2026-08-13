// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Talkdedsec

use crate::color::Settings;
use crate::preview::Scene;
use crate::Preset as UiPreset;

pub struct BuiltIn {
    pub name: &'static str,
    pub hint: &'static str,
    pub settings: Settings,
}

pub const ALL: &[BuiltIn] = &[
    BuiltIn {
        name: "Berrak",
        hint: "biraz daha net",
        settings: Settings {
            brightness: 0.03,
            contrast: 1.16,
            gamma: 1.08,
            temperature: 0.0,
            night_vision: 0.0,
        },
    },
    BuiltIn {
        name: "Gece Görüşü",
        hint: "karanlıkta detay",
        settings: Settings {
            brightness: 0.04,
            contrast: 1.05,
            gamma: 1.35,
            temperature: -0.1,
            night_vision: 0.85,
        },
    },
    BuiltIn {
        name: "Sıcak",
        hint: "göz yormayan ton",
        settings: Settings {
            brightness: 0.0,
            contrast: 1.0,
            gamma: 1.05,
            temperature: 0.55,
            night_vision: 0.0,
        },
    },
    BuiltIn {
        name: "Soğuk",
        hint: "mavi ve sert",
        settings: Settings {
            brightness: 0.02,
            contrast: 1.12,
            gamma: 1.0,
            temperature: -0.5,
            night_vision: 0.0,
        },
    },
    BuiltIn {
        name: "Gece Okuma",
        hint: "kısık ve sıcak",
        settings: Settings {
            brightness: -0.14,
            contrast: 0.92,
            gamma: 0.88,
            temperature: 0.7,
            night_vision: 0.0,
        },
    },
    BuiltIn {
        name: "Sert Kontrast",
        hint: "gölgeler kapanır",
        settings: Settings {
            brightness: 0.0,
            contrast: 1.55,
            gamma: 1.0,
            temperature: 0.0,
            night_vision: 0.0,
        },
    },
];

pub fn at(index: usize) -> Option<&'static BuiltIn> {
    ALL.get(index)
}

pub fn index_of(settings: &Settings) -> i32 {
    ALL.iter()
        .position(|p| p.settings == *settings)
        .map_or(-1, |i| i as i32)
}

pub fn ui_models(scene: &Scene) -> Vec<UiPreset> {
    ALL.iter()
        .map(|p| UiPreset {
            name: p.name.into(),
            hint: p.hint.into(),
            thumb: scene.render(&p.settings),
        })
        .collect()
}
