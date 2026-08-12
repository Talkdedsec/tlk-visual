// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Talkdedsec

use serde::{Deserialize, Serialize};

pub const LUM_R: f32 = 0.2125;
pub const LUM_G: f32 = 0.7154;
pub const LUM_B: f32 = 0.0721;

/// Row-major 5x5 affine color matrix, row-vector convention:
/// `[r g b a 1] * M`, so row 4 carries the per-channel offset.
pub type Matrix = [f32; 25];

pub const IDENTITY: Matrix = [
    1.0, 0.0, 0.0, 0.0, 0.0, //
    0.0, 1.0, 0.0, 0.0, 0.0, //
    0.0, 0.0, 1.0, 0.0, 0.0, //
    0.0, 0.0, 0.0, 1.0, 0.0, //
    0.0, 0.0, 0.0, 0.0, 1.0,
];

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub brightness: f32,
    pub contrast: f32,
    pub saturation: f32,
    pub hue: f32,
    pub night_vision: f32,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            brightness: 0.0,
            contrast: 1.0,
            saturation: 1.0,
            hue: 0.0,
            night_vision: 0.0,
        }
    }
}

impl Settings {
    pub const BRIGHTNESS_RANGE: (f32, f32) = (-0.5, 0.5);
    pub const CONTRAST_RANGE: (f32, f32) = (0.5, 2.0);
    pub const SATURATION_RANGE: (f32, f32) = (0.0, 3.0);
    pub const HUE_RANGE: (f32, f32) = (-180.0, 180.0);
    pub const NIGHT_VISION_RANGE: (f32, f32) = (0.0, 1.0);

    pub fn clamped(self) -> Self {
        Self {
            brightness: clamp(self.brightness, Self::BRIGHTNESS_RANGE),
            contrast: clamp(self.contrast, Self::CONTRAST_RANGE),
            saturation: clamp(self.saturation, Self::SATURATION_RANGE),
            hue: clamp(self.hue, Self::HUE_RANGE),
            night_vision: clamp(self.night_vision, Self::NIGHT_VISION_RANGE),
        }
    }

    pub fn is_neutral(&self) -> bool {
        near(self.brightness, 0.0)
            && near(self.contrast, 1.0)
            && near(self.saturation, 1.0)
            && near(self.hue, 0.0)
            && near(self.night_vision, 0.0)
    }

    pub fn matrix(&self) -> Matrix {
        let s = self.clamped();
        let mut m = contrast_matrix(s.contrast);
        m = mul(m, saturation_matrix(s.saturation));
        m = mul(m, hue_matrix(s.hue));
        m = mul(m, night_vision_matrix(s.night_vision));
        mul(m, brightness_matrix(s.brightness))
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

/// `a` applied first, then `b`.
pub fn mul(a: Matrix, b: Matrix) -> Matrix {
    let mut out = [0.0f32; 25];
    for row in 0..5 {
        for col in 0..5 {
            let mut sum = 0.0;
            for k in 0..5 {
                sum += a[row * 5 + k] * b[k * 5 + col];
            }
            out[row * 5 + col] = sum;
        }
    }
    out
}

pub fn brightness_matrix(b: f32) -> Matrix {
    let mut m = IDENTITY;
    m[20] = b;
    m[21] = b;
    m[22] = b;
    m
}

pub fn contrast_matrix(c: f32) -> Matrix {
    let offset = 0.5 * (1.0 - c);
    let mut m = IDENTITY;
    m[0] = c;
    m[6] = c;
    m[12] = c;
    m[20] = offset;
    m[21] = offset;
    m[22] = offset;
    m
}

pub fn saturation_matrix(s: f32) -> Matrix {
    let inv = 1.0 - s;
    let (r, g, b) = (LUM_R * inv, LUM_G * inv, LUM_B * inv);
    [
        r + s, r, r, 0.0, 0.0, //
        g, g + s, g, 0.0, 0.0, //
        b, b, b + s, 0.0, 0.0, //
        0.0, 0.0, 0.0, 1.0, 0.0, //
        0.0, 0.0, 0.0, 0.0, 1.0,
    ]
}

pub fn hue_matrix(degrees: f32) -> Matrix {
    let theta = degrees.to_radians();
    let (sin, cos) = theta.sin_cos();

    let rr = 0.213 + cos * 0.787 - sin * 0.213;
    let rg = 0.715 - cos * 0.715 - sin * 0.715;
    let rb = 0.072 - cos * 0.072 + sin * 0.928;
    let gr = 0.213 - cos * 0.213 + sin * 0.143;
    let gg = 0.715 + cos * 0.285 + sin * 0.140;
    let gb = 0.072 - cos * 0.072 - sin * 0.283;
    let br = 0.213 - cos * 0.213 - sin * 0.787;
    let bg = 0.715 - cos * 0.715 + sin * 0.715;
    let bb = 0.072 + cos * 0.928 + sin * 0.072;

    [
        rr, gr, br, 0.0, 0.0, //
        rg, gg, bg, 0.0, 0.0, //
        rb, gb, bb, 0.0, 0.0, //
        0.0, 0.0, 0.0, 1.0, 0.0, //
        0.0, 0.0, 0.0, 0.0, 1.0,
    ]
}

/// Blends toward a luminance-driven green cast with a black lift, so detail in
/// dark areas separates instead of clipping to black.
pub fn night_vision_matrix(n: f32) -> Matrix {
    if n <= 0.0 {
        return IDENTITY;
    }
    const DIM: f32 = 0.35;
    const GAIN: f32 = 1.75;
    const LIFT: f32 = 0.10;

    let target = [
        LUM_R * DIM, LUM_R * GAIN, LUM_R * DIM, 0.0, 0.0, //
        LUM_G * DIM, LUM_G * GAIN, LUM_G * DIM, 0.0, 0.0, //
        LUM_B * DIM, LUM_B * GAIN, LUM_B * DIM, 0.0, 0.0, //
        0.0, 0.0, 0.0, 1.0, 0.0, //
        LIFT * 0.6, LIFT, LIFT * 0.6, 0.0, 1.0,
    ];

    let mut out = [0.0f32; 25];
    for i in 0..25 {
        out[i] = IDENTITY[i] * (1.0 - n) + target[i] * n;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn apply(m: &Matrix, rgb: [f32; 3]) -> [f32; 3] {
        let v = [rgb[0], rgb[1], rgb[2], 1.0, 1.0];
        let mut out = [0.0f32; 3];
        for col in 0..3 {
            let mut sum = 0.0;
            for k in 0..5 {
                sum += v[k] * m[k * 5 + col];
            }
            out[col] = sum;
        }
        out
    }

    fn assert_close(a: [f32; 3], b: [f32; 3]) {
        for i in 0..3 {
            assert!((a[i] - b[i]).abs() < 1e-3, "{a:?} != {b:?}");
        }
    }

    #[test]
    fn default_settings_are_neutral() {
        let s = Settings::default();
        assert!(s.is_neutral());
        assert_close(apply(&s.matrix(), [0.3, 0.6, 0.9]), [0.3, 0.6, 0.9]);
    }

    #[test]
    fn zero_saturation_gives_luminance() {
        let m = saturation_matrix(0.0);
        let out = apply(&m, [0.2, 0.5, 0.8]);
        let lum = LUM_R * 0.2 + LUM_G * 0.5 + LUM_B * 0.8;
        assert_close(out, [lum, lum, lum]);
    }

    #[test]
    fn contrast_pivots_around_mid_gray() {
        let m = contrast_matrix(1.8);
        assert_close(apply(&m, [0.5, 0.5, 0.5]), [0.5, 0.5, 0.5]);
    }

    #[test]
    fn hue_rotation_is_identity_at_zero_and_full_turn() {
        assert_close(apply(&hue_matrix(0.0), [0.4, 0.2, 0.7]), [0.4, 0.2, 0.7]);
        assert_close(apply(&hue_matrix(360.0), [0.4, 0.2, 0.7]), [0.4, 0.2, 0.7]);
    }

    #[test]
    fn hue_rotation_preserves_gray() {
        assert_close(apply(&hue_matrix(90.0), [0.5, 0.5, 0.5]), [0.5, 0.5, 0.5]);
    }

    #[test]
    fn night_vision_lifts_shadows_toward_green() {
        let dark = [0.05, 0.05, 0.05];
        let out = apply(&night_vision_matrix(1.0), dark);
        assert!(out[1] > dark[1], "green should lift: {out:?}");
        assert!(out[1] > out[0] && out[1] > out[2], "green cast: {out:?}");
    }

    #[test]
    fn brightness_offsets_every_channel() {
        assert_close(apply(&brightness_matrix(0.2), [0.1, 0.2, 0.3]), [0.3, 0.4, 0.5]);
    }

    #[test]
    fn composition_matches_sequential_application() {
        let s = Settings {
            brightness: 0.08,
            contrast: 1.2,
            saturation: 1.6,
            hue: 25.0,
            night_vision: 0.0,
        };
        let rgb = [0.35, 0.55, 0.2];
        let stepwise = apply(
            &brightness_matrix(s.brightness),
            apply(
                &hue_matrix(s.hue),
                apply(
                    &saturation_matrix(s.saturation),
                    apply(&contrast_matrix(s.contrast), rgb),
                ),
            ),
        );
        assert_close(apply(&s.matrix(), rgb), stepwise);
    }

    #[test]
    fn out_of_range_input_is_clamped() {
        let s = Settings {
            brightness: 99.0,
            contrast: -5.0,
            saturation: f32::NAN,
            hue: 400.0,
            night_vision: 2.0,
        }
        .clamped();
        assert_eq!(s.brightness, Settings::BRIGHTNESS_RANGE.1);
        assert_eq!(s.contrast, Settings::CONTRAST_RANGE.0);
        assert_eq!(s.saturation, Settings::SATURATION_RANGE.0);
        assert_eq!(s.hue, Settings::HUE_RANGE.1);
        assert_eq!(s.night_vision, Settings::NIGHT_VISION_RANGE.1);
    }
}
