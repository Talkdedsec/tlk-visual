// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Talkdedsec

use slint::{Model, VecModel};

use crate::color::Settings;
use crate::i18n::t;
use crate::preview::Scene;
use crate::Preset as UiPreset;

pub struct BuiltIn {
    pub name: &'static str,
    pub hint: &'static str,
    pub settings: Settings,
}

pub const ALL: &[BuiltIn] = &[
    BuiltIn {
        name: "Clear",
        hint: "a little crisper",
        settings: Settings {
            brightness: 0.03,
            contrast: 1.16,
            gamma: 1.08,
            temperature: 0.0,
            night_vision: 0.0,
        },
    },
    BuiltIn {
        name: "Night Vision",
        hint: "detail in the dark",
        settings: Settings {
            brightness: 0.04,
            contrast: 1.05,
            gamma: 1.35,
            temperature: -0.1,
            night_vision: 0.85,
        },
    },
    BuiltIn {
        name: "Warm",
        hint: "easy on the eyes",
        settings: Settings {
            brightness: 0.0,
            contrast: 1.0,
            gamma: 1.05,
            temperature: 0.55,
            night_vision: 0.0,
        },
    },
    BuiltIn {
        name: "Cool",
        hint: "blue and sharp",
        settings: Settings {
            brightness: 0.02,
            contrast: 1.12,
            gamma: 1.0,
            temperature: -0.5,
            night_vision: 0.0,
        },
    },
    BuiltIn {
        name: "Night Reading",
        hint: "dim and warm",
        settings: Settings {
            brightness: -0.14,
            contrast: 0.92,
            gamma: 0.88,
            temperature: 0.7,
            night_vision: 0.0,
        },
    },
    BuiltIn {
        name: "Hard Contrast",
        hint: "crushed shadows",
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
            name: t(p.name).into(),
            hint: t(p.hint).into(),
            thumb: scene.render(&p.settings),
        })
        .collect()
}

/// Names in the current language on an existing model; the thumbnails stay.
pub fn relabel(model: &VecModel<UiPreset>) {
    for (i, p) in ALL.iter().enumerate() {
        if let Some(mut row) = model.row_data(i) {
            row.name = t(p.name).into();
            row.hint = t(p.hint).into();
            model.set_row_data(i, row);
        }
    }
}
