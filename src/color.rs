// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Talkdedsec

use serde::{Deserialize, Serialize};

/// A Windows gamma ramp: 256 entries per channel, red then green then blue.
pub type Ramp = [u16; 768];

pub const LEVELS: usize = 256;

pub fn identity() -> Ramp {
    let mut ramp = [0u16; 768];
    for i in 0..LEVELS {
        let v = (i * 257) as u16;
        ramp[i] = v;
        ramp[LEVELS + i] = v;
        ramp[LEVELS * 2 + i] = v;
    }
    ramp
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub brightness: f32,
    pub contrast: f32,
    pub gamma: f32,
    pub temperature: f32,
    pub night_vision: f32,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            brightness: 0.0,
            contrast: 1.0,
            gamma: 1.0,
            temperature: 0.0,
            night_vision: 0.0,
        }
    }
}

impl Settings {
    pub const BRIGHTNESS_RANGE: (f32, f32) = (-0.35, 0.35);
    pub const CONTRAST_RANGE: (f32, f32) = (0.60, 1.80);
    pub const GAMMA_RANGE: (f32, f32) = (0.50, 2.20);
    pub const TEMPERATURE_RANGE: (f32, f32) = (-1.0, 1.0);
    pub const NIGHT_VISION_RANGE: (f32, f32) = (0.0, 1.0);

    pub fn clamped(self) -> Self {
        Self {
            brightness: clamp(self.brightness, Self::BRIGHTNESS_RANGE),
            contrast: clamp(self.contrast, Self::CONTRAST_RANGE),
            gamma: clamp(self.gamma, Self::GAMMA_RANGE),
            temperature: clamp(self.temperature, Self::TEMPERATURE_RANGE),
            night_vision: clamp(self.night_vision, Self::NIGHT_VISION_RANGE),
        }
    }

    pub fn is_neutral(&self) -> bool {
        let d = Settings::default();
        near(self.brightness, d.brightness)
            && near(self.contrast, d.contrast)
            && near(self.gamma, d.gamma)
            && near(self.temperature, d.temperature)
            && near(self.night_vision, d.night_vision)
    }

    /// Value of one channel at input level `v` (0..1), before the ramp is quantised.
    pub fn channel(&self, v: f32, channel: usize) -> f32 {
        let s = self.clamped();
        let mut x = v.clamp(0.0, 1.0);

        // shadow lift first, so the curve below only reshapes what is already there
        if s.night_vision > 0.0 {
            x += s.night_vision * 0.45 * (1.0 - x).powf(2.2);
        }

        x = x.clamp(0.0, 1.0).powf(1.0 / s.gamma);
        x = (x - 0.5) * s.contrast + 0.5;
        x += s.brightness;

        let warm = s.temperature * 0.22;
        x *= match channel {
            0 => 1.0 + warm,
            2 => 1.0 - warm,
            _ => 1.0 - warm.abs() * 0.06,
        };

        x.clamp(0.0, 1.0)
    }

    pub fn ramp(&self) -> Ramp {
        let mut ramp = [0u16; 768];
        for channel in 0..3 {
            for i in 0..LEVELS {
                let v = i as f32 / (LEVELS - 1) as f32;
                let out = self.channel(v, channel);
                ramp[channel * LEVELS + i] = (out * 65535.0 + 0.5) as u16;
            }
        }
        ramp
    }

    /// Blends toward neutral. Windows rejects ramps that stray too far from
    /// linear, so the engine walks this down until the driver accepts one.
    pub fn scaled(&self, factor: f32) -> Self {
        let f = factor.clamp(0.0, 1.0);
        let d = Settings::default();
        Self {
            brightness: d.brightness + (self.brightness - d.brightness) * f,
            contrast: d.contrast + (self.contrast - d.contrast) * f,
            gamma: d.gamma + (self.gamma - d.gamma) * f,
            temperature: d.temperature + (self.temperature - d.temperature) * f,
            night_vision: d.night_vision + (self.night_vision - d.night_vision) * f,
        }
    }
}

fn clamp(v: f32, (lo, hi): (f32, f32)) -> f32 {
    if v.is_finite() {
        v.max(lo).min(hi)
    } else {
        lo
    }
}

fn near(a: f32, b: f32) -> bool {
    (a - b).abs() < 1e-4
}

#[cfg(test)]
mod tests {
    use super::*;

    fn red(ramp: &Ramp, i: usize) -> u16 {
        ramp[i]
    }
    fn blue(ramp: &Ramp, i: usize) -> u16 {
        ramp[LEVELS * 2 + i]
    }

    #[test]
    fn defaults_are_neutral_and_produce_the_identity_ramp() {
        let s = Settings::default();
        assert!(s.is_neutral());
        let ramp = s.ramp();
        let ideal = identity();
        for i in 0..LEVELS {
            assert!(
                (ramp[i] as i32 - ideal[i] as i32).abs() <= 1,
                "level {i}: {} vs {}",
                ramp[i],
                ideal[i]
            );
        }
    }

    #[test]
    fn every_ramp_is_monotonic() {
        let cases = [
            Settings::default(),
            Settings { brightness: 0.3, ..Default::default() },
            Settings { contrast: 1.8, ..Default::default() },
            Settings { gamma: 2.2, ..Default::default() },
            Settings { gamma: 0.5, ..Default::default() },
            Settings { temperature: 1.0, ..Default::default() },
            Settings { temperature: -1.0, ..Default::default() },
            Settings { night_vision: 1.0, ..Default::default() },
        ];
        for s in cases {
            let ramp = s.ramp();
            for channel in 0..3 {
                for i in 1..LEVELS {
                    let prev = ramp[channel * LEVELS + i - 1];
                    let cur = ramp[channel * LEVELS + i];
                    assert!(cur >= prev, "{s:?} channel {channel} dips at {i}");
                }
            }
        }
    }

    #[test]
    fn brightness_lifts_the_whole_curve() {
        let lifted = Settings { brightness: 0.2, ..Default::default() }.ramp();
        let flat = Settings::default().ramp();
        assert!(lifted[10] > flat[10]);
        assert!(lifted[128] > flat[128]);
    }

    #[test]
    fn contrast_pivots_on_mid_grey() {
        let s = Settings { contrast: 1.8, ..Default::default() };
        let mid = s.channel(0.5, 0);
        assert!((mid - 0.5).abs() < 1e-4, "mid grey moved to {mid}");
        assert!(s.channel(0.25, 0) < 0.25);
        assert!(s.channel(0.75, 0) > 0.75);
    }

    #[test]
    fn gamma_reshapes_midtones_without_moving_the_ends() {
        let bright = Settings { gamma: 2.2, ..Default::default() };
        assert!(bright.channel(0.5, 0) > 0.5);
        assert!(bright.channel(0.0, 0).abs() < 1e-6);
        assert!((bright.channel(1.0, 0) - 1.0).abs() < 1e-6);

        let dark = Settings { gamma: 0.5, ..Default::default() };
        assert!(dark.channel(0.5, 0) < 0.5);
    }

    #[test]
    fn temperature_separates_red_from_blue() {
        let warm = Settings { temperature: 1.0, ..Default::default() }.ramp();
        let cool = Settings { temperature: -1.0, ..Default::default() }.ramp();
        assert!(red(&warm, 200) > blue(&warm, 200), "warm should favour red");
        assert!(blue(&cool, 200) > red(&cool, 200), "cool should favour blue");
    }

    #[test]
    fn night_vision_lifts_shadows_far_more_than_highlights() {
        let s = Settings { night_vision: 1.0, ..Default::default() };
        let shadow_gain = s.channel(0.02, 0) - 0.02;
        let highlight_gain = s.channel(0.9, 0) - 0.9;
        assert!(shadow_gain > 0.3, "shadows barely moved: {shadow_gain}");
        assert!(
            highlight_gain < shadow_gain * 0.1,
            "highlights moved too much: {highlight_gain}"
        );
    }

    #[test]
    fn scaling_to_zero_returns_neutral_and_to_one_returns_the_original() {
        let s = Settings {
            brightness: 0.2,
            contrast: 1.5,
            gamma: 1.8,
            temperature: 0.6,
            night_vision: 0.4,
        };
        assert!(s.scaled(0.0).is_neutral());
        assert_eq!(s.scaled(1.0), s);
        let half = s.scaled(0.5);
        assert!((half.brightness - 0.1).abs() < 1e-5);
        assert!((half.contrast - 1.25).abs() < 1e-5);
    }

    #[test]
    fn out_of_range_input_is_clamped() {
        let s = Settings {
            brightness: 99.0,
            contrast: -5.0,
            gamma: f32::NAN,
            temperature: 40.0,
            night_vision: 2.0,
        }
        .clamped();
        assert_eq!(s.brightness, Settings::BRIGHTNESS_RANGE.1);
        assert_eq!(s.contrast, Settings::CONTRAST_RANGE.0);
        assert_eq!(s.gamma, Settings::GAMMA_RANGE.0);
        assert_eq!(s.temperature, Settings::TEMPERATURE_RANGE.1);
        assert_eq!(s.night_vision, Settings::NIGHT_VISION_RANGE.1);
    }

    #[test]
    fn output_never_leaves_the_representable_range() {
        let extreme = Settings {
            brightness: 0.35,
            contrast: 1.8,
            gamma: 2.2,
            temperature: 1.0,
            night_vision: 1.0,
        };
        for channel in 0..3 {
            for i in 0..LEVELS {
                let v = extreme.channel(i as f32 / 255.0, channel);
                assert!((0.0..=1.0).contains(&v), "channel {channel} level {i} = {v}");
            }
        }
    }
}
