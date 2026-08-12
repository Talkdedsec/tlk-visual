// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Talkdedsec

use crate::color::Matrix;
use slint::{Image, Rgb8Pixel, SharedPixelBuffer};

pub struct Scene {
    width: u32,
    height: u32,
    pixels: Vec<f32>,
}

impl Scene {
    pub fn new(width: u32, height: u32) -> Self {
        Self::build(width, height, true)
    }

    /// Preset thumbnails are too small for the calibration strip to read.
    pub fn thumbnail(width: u32, height: u32) -> Self {
        Self::build(width, height, false)
    }

    fn build(width: u32, height: u32, swatches: bool) -> Self {
        let mut pixels = Vec::with_capacity((width * height * 3) as usize);
        for y in 0..height {
            for x in 0..width {
                let u = x as f32 / (width - 1).max(1) as f32;
                let v = y as f32 / (height - 1).max(1) as f32;
                let [r, g, b] = sample(u, v, swatches);
                pixels.push(r);
                pixels.push(g);
                pixels.push(b);
            }
        }
        Self { width, height, pixels }
    }

    pub fn render(&self, matrix: &Matrix) -> Image {
        let mut buffer = SharedPixelBuffer::<Rgb8Pixel>::new(self.width, self.height);
        let out = buffer.make_mut_slice();
        for (i, px) in out.iter_mut().enumerate() {
            let base = i * 3;
            let (r, g, b) = (self.pixels[base], self.pixels[base + 1], self.pixels[base + 2]);
            let mut channel = [0.0f32; 3];
            for (c, slot) in channel.iter_mut().enumerate() {
                *slot = r * matrix[c] + g * matrix[5 + c] + b * matrix[10 + c] + matrix[15 + c] + matrix[20 + c];
            }
            *px = Rgb8Pixel {
                r: to_byte(channel[0]),
                g: to_byte(channel[1]),
                b: to_byte(channel[2]),
            };
        }
        Image::from_rgb8(buffer)
    }
}

fn to_byte(v: f32) -> u8 {
    (v.clamp(0.0, 1.0) * 255.0 + 0.5) as u8
}

fn mix(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    let t = t.clamp(0.0, 1.0);
    [
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
    ]
}

fn hash(x: f32, y: f32) -> f32 {
    let n = (x * 127.1 + y * 311.7).sin() * 43758.545;
    n - n.floor()
}

fn noise(x: f32, y: f32) -> f32 {
    let (xi, yi) = (x.floor(), y.floor());
    let (xf, yf) = (x - xi, y - yi);
    let sx = xf * xf * (3.0 - 2.0 * xf);
    let sy = yf * yf * (3.0 - 2.0 * yf);
    let n00 = hash(xi, yi);
    let n10 = hash(xi + 1.0, yi);
    let n01 = hash(xi, yi + 1.0);
    let n11 = hash(xi + 1.0, yi + 1.0);
    let top = n00 + (n10 - n00) * sx;
    let bottom = n01 + (n11 - n01) * sx;
    top + (bottom - top) * sy
}

const HORIZON: f32 = 0.46;
const SWATCH_TOP: f32 = 0.865;

/// A synthetic outdoor frame: sky, treeline, terrain, a deep shadow pocket and a
/// calibration strip. Every control on the panel has something to bite on here.
fn sample(u: f32, v: f32, swatches: bool) -> [f32; 3] {
    let floor = if swatches { SWATCH_TOP } else { 1.0 };
    if swatches && v >= SWATCH_TOP {
        return swatch(u, v);
    }

    let mut rgb = if v < HORIZON {
        sky(u, v)
    } else {
        terrain(u, v, floor)
    };

    rgb = mix(rgb, [0.055, 0.085, 0.045], treeline(u, v));
    rgb = mix(rgb, [0.20, 0.19, 0.18], rocks(u, v, floor));

    let shadow = shadow_mask(u, v);
    rgb = mix(rgb, [rgb[0] * 0.12, rgb[1] * 0.13, rgb[2] * 0.16], shadow);

    let vignette = 1.0 - 0.28 * ((u - 0.5).powi(2) + (v - 0.5).powi(2));
    [rgb[0] * vignette, rgb[1] * vignette, rgb[2] * vignette]
}

fn treeline(u: f32, v: f32) -> f32 {
    let crown = noise(u * 11.0, 1.7) * 0.6 + noise(u * 27.0, 4.3) * 0.4;
    let top = HORIZON - 0.015 - 0.115 * crown;
    let bottom = HORIZON + 0.035;
    if v < top || v > bottom {
        return 0.0;
    }
    let fade = 1.0 - ((v - top) / (bottom - top)).clamp(0.0, 1.0).powf(2.2);
    (fade * 0.95).min(1.0)
}

fn rocks(u: f32, v: f32, floor: f32) -> f32 {
    const SPOTS: [(f32, f32, f32, f32); 3] = [
        (0.31, 0.74, 0.055, 0.028),
        (0.47, 0.88, 0.035, 0.018),
        (0.82, 0.79, 0.045, 0.022),
    ];
    let vv = (v - HORIZON) / (floor - HORIZON).max(0.0001);
    let mut cover: f32 = 0.0;
    for (cx, cy, rx, ry) in SPOTS {
        let d = ((u - cx) / rx).powi(2) + ((vv - cy) / (ry / (floor - HORIZON).max(0.0001) * 0.5)).powi(2);
        cover = cover.max((1.0 - d).clamp(0.0, 1.0).powf(0.35));
    }
    cover
}

fn sky(u: f32, v: f32) -> [f32; 3] {
    let t = v / HORIZON;
    let mut rgb = mix([0.09, 0.20, 0.38], [0.55, 0.68, 0.80], t.powf(0.75));

    let sun = 1.0 - (((u - 0.74).powi(2) * 2.4 + (v - 0.10).powi(2) * 6.0).sqrt() * 3.2).min(1.0);
    rgb = mix(rgb, [0.98, 0.95, 0.84], sun * 0.75);

    let cloud = noise(u * 6.0, v * 9.0) * 0.6 + noise(u * 14.0, v * 18.0) * 0.4;
    let band = ((1.0 - t) * 1.4).min(1.0);
    mix(rgb, [0.86, 0.89, 0.93], ((cloud - 0.55).max(0.0) * 1.9 * band).min(0.85))
}

fn terrain(u: f32, v: f32, floor: f32) -> [f32; 3] {
    let t = ((v - HORIZON) / (floor - HORIZON)).clamp(0.0, 1.0);
    let mut rgb = mix([0.20, 0.26, 0.14], [0.34, 0.30, 0.17], t.powf(0.8));

    let grain = noise(u * 26.0, v * 34.0);
    rgb = mix(rgb, [0.16, 0.22, 0.11], (grain - 0.5).max(0.0) * 0.9);

    let foliage = noise(u * 8.0 + 3.0, v * 5.0);
    rgb = mix(rgb, [0.11, 0.19, 0.09], ((foliage - 0.62).max(0.0) * 2.6).min(0.9));

    let path = 1.0 - ((u - 0.58 - t * 0.16).abs() * 9.0).min(1.0);
    mix(rgb, [0.42, 0.37, 0.26], path * 0.55 * t)
}

fn shadow_mask(u: f32, v: f32) -> f32 {
    let edge = 1.0 - ((u - 0.14).abs() * 3.4).min(1.0);
    let depth = ((v - HORIZON) * 2.6).clamp(0.0, 1.0);
    (edge * depth).powf(0.7)
}

fn swatch(u: f32, v: f32) -> [f32; 3] {
    let strip = ((v - SWATCH_TOP) / (1.0 - SWATCH_TOP)).clamp(0.0, 1.0);
    if strip < 0.06 {
        return [0.04, 0.04, 0.05];
    }

    const COLORS: [[f32; 3]; 12] = [
        [0.02, 0.02, 0.02],
        [0.25, 0.25, 0.25],
        [0.50, 0.50, 0.50],
        [0.75, 0.75, 0.75],
        [0.98, 0.98, 0.98],
        [0.78, 0.14, 0.14],
        [0.90, 0.58, 0.12],
        [0.88, 0.82, 0.16],
        [0.16, 0.62, 0.26],
        [0.14, 0.44, 0.82],
        [0.48, 0.20, 0.72],
        [0.86, 0.66, 0.52],
    ];

    let slot = (u * COLORS.len() as f32).floor() as usize;
    let color = COLORS[slot.min(COLORS.len() - 1)];
    let local = u * COLORS.len() as f32 - slot as f32;
    let gap = (local * 22.0).min(1.0) * ((1.0 - local) * 22.0).min(1.0);
    mix([0.04, 0.04, 0.05], color, gap)
}
