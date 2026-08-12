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
        name: "Canlı",
        hint: "renkler öne çıkar",
        settings: Settings {
            brightness: 0.06,
            contrast: 1.05,
            saturation: 1.52,
            hue: 0.0,
            night_vision: 0.0,
        },
    },
    BuiltIn {
        name: "Yaz",
        hint: "sıcak ve doygun",
        settings: Settings {
            brightness: 0.03,
            contrast: 1.06,
            saturation: 1.38,
            hue: -6.0,
            night_vision: 0.0,
        },
    },
    BuiltIn {
        name: "Kış",
        hint: "soğuk ve net",
        settings: Settings {
            brightness: 0.07,
            contrast: 1.16,
            saturation: 0.82,
            hue: -14.0,
            night_vision: 0.0,
        },
    },
    BuiltIn {
        name: "Çöl",
        hint: "toprak tonları",
        settings: Settings {
            brightness: 0.02,
            contrast: 1.04,
            saturation: 1.24,
            hue: 14.0,
            night_vision: 0.0,
        },
    },
    BuiltIn {
        name: "Gece Görüşü",
        hint: "karanlıkta detay",
        settings: Settings {
            brightness: 0.08,
            contrast: 1.10,
            saturation: 0.60,
            hue: 0.0,
            night_vision: 0.85,
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
            thumb: scene.render(&p.settings.matrix()),
        })
        .collect()
}
