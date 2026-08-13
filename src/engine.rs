// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Talkdedsec

use crate::color::{identity, Ramp, Settings};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Applied {
    /// The driver took the ramp exactly as asked.
    Full,
    /// Windows refused the full strength; this fraction is what it accepted.
    Limited(f32),
    /// Nothing was accepted, not even a whisper of the effect.
    Rejected,
}

#[derive(Debug)]
pub enum EngineError {
    NoDisplay,
}

impl std::fmt::Display for EngineError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoDisplay => write!(f, "no display accepted a gamma ramp"),
        }
    }
}

impl std::error::Error for EngineError {}

/// Windows silently rejects ramps that stray too far from linear unless
/// `GdiIcmGammaRange` is widened, so the engine offers the strongest setting the
/// driver will take rather than failing outright.
const BACKOFF: [f32; 7] = [1.0, 0.85, 0.7, 0.55, 0.4, 0.25, 0.12];

pub struct Engine {
    original: Vec<(sys::Display, Ramp)>,
    applied: Option<Ramp>,
}

#[cfg(windows)]
mod sys {
    use crate::color::Ramp;
    use windows::core::PCWSTR;
    use windows::core::BOOL;
    use windows::Win32::Graphics::Gdi::{
        CreateDCW, DeleteDC, EnumDisplayDevicesW, GetDC, ReleaseDC, DISPLAY_DEVICEW,
        DISPLAY_DEVICE_ATTACHED_TO_DESKTOP, DISPLAY_DEVICE_STATE_FLAGS, HDC,
    };

    // windows-rs 0.62 does not bind the gamma ramp calls, so declare them here.
    #[link(name = "gdi32")]
    unsafe extern "system" {
        fn SetDeviceGammaRamp(hdc: HDC, lpramp: *const core::ffi::c_void) -> BOOL;
        fn GetDeviceGammaRamp(hdc: HDC, lpramp: *mut core::ffi::c_void) -> BOOL;
    }

    #[derive(Debug, Clone)]
    pub enum Display {
        Desktop,
        Adapter(Vec<u16>),
    }

    struct Handle {
        hdc: HDC,
        owned: bool,
    }

    impl Handle {
        fn open(display: &Display) -> Option<Self> {
            match display {
                Display::Desktop => {
                    let hdc = unsafe { GetDC(None) };
                    (!hdc.is_invalid()).then_some(Self { hdc, owned: false })
                }
                Display::Adapter(name) => {
                    let hdc = unsafe { CreateDCW(PCWSTR(name.as_ptr()), None, None, None) };
                    (!hdc.is_invalid()).then_some(Self { hdc, owned: true })
                }
            }
        }
    }

    impl Drop for Handle {
        fn drop(&mut self) {
            unsafe {
                if self.owned {
                    let _ = DeleteDC(self.hdc);
                } else {
                    ReleaseDC(None, self.hdc);
                }
            }
        }
    }

    pub fn displays() -> Vec<Display> {
        let mut found = vec![Display::Desktop];
        let mut index = 0u32;
        loop {
            let mut device = DISPLAY_DEVICEW {
                cb: std::mem::size_of::<DISPLAY_DEVICEW>() as u32,
                ..Default::default()
            };
            let ok = unsafe { EnumDisplayDevicesW(None, index, &mut device, 0) };
            if !ok.as_bool() {
                break;
            }
            index += 1;
            if device.StateFlags & DISPLAY_DEVICE_ATTACHED_TO_DESKTOP == DISPLAY_DEVICE_STATE_FLAGS(0) {
                continue;
            }
            let name: Vec<u16> = device
                .DeviceName
                .iter()
                .copied()
                .take_while(|c| *c != 0)
                .chain(std::iter::once(0))
                .collect();
            found.push(Display::Adapter(name));
        }
        found
    }

    pub fn get(display: &Display) -> Option<Ramp> {
        let handle = Handle::open(display)?;
        let mut ramp = [0u16; 768];
        let ok = unsafe { GetDeviceGammaRamp(handle.hdc, ramp.as_mut_ptr() as *mut _) };
        ok.as_bool().then_some(ramp)
    }

    pub fn set(display: &Display, ramp: &Ramp) -> bool {
        let Some(handle) = Handle::open(display) else {
            return false;
        };
        unsafe { SetDeviceGammaRamp(handle.hdc, ramp.as_ptr() as *const _) }.as_bool()
    }
}

#[cfg(not(windows))]
mod sys {
    use crate::color::Ramp;

    #[derive(Debug, Clone)]
    pub enum Display {
        Desktop,
    }

    pub fn displays() -> Vec<Display> {
        Vec::new()
    }
    pub fn get(_: &Display) -> Option<Ramp> {
        None
    }
    pub fn set(_: &Display, _: &Ramp) -> bool {
        false
    }
}

impl Engine {
    pub fn new() -> Result<Self, EngineError> {
        let original: Vec<_> = sys::displays()
            .into_iter()
            .filter_map(|display| sys::get(&display).map(|ramp| (display, ramp)))
            .collect();

        if original.is_empty() {
            return Err(EngineError::NoDisplay);
        }
        Ok(Self {
            original,
            applied: None,
        })
    }

    pub fn displays(&self) -> usize {
        self.original.len()
    }

    fn push(&mut self, ramp: &Ramp) -> bool {
        let mut any = false;
        for (display, _) in &self.original {
            any |= sys::set(display, ramp);
        }
        if any {
            self.applied = Some(*ramp);
        }
        any
    }

    pub fn apply(&mut self, settings: &Settings) -> Applied {
        if settings.is_neutral() {
            self.reset();
            return Applied::Full;
        }
        for factor in BACKOFF {
            let ramp = settings.scaled(factor).ramp();
            if self.applied == Some(ramp) {
                return if factor >= 1.0 {
                    Applied::Full
                } else {
                    Applied::Limited(factor)
                };
            }
            if self.push(&ramp) {
                return if factor >= 1.0 {
                    Applied::Full
                } else {
                    Applied::Limited(factor)
                };
            }
        }
        Applied::Rejected
    }

    /// Gamma ramps outlive the process that set them, so putting the screen back
    /// is not optional.
    pub fn reset(&mut self) {
        let restored = self
            .original
            .iter()
            .map(|(display, ramp)| sys::set(display, ramp))
            .fold(false, |acc, ok| acc || ok);

        if !restored {
            let flat = identity();
            for (display, _) in &self.original {
                sys::set(display, &flat);
            }
        }
        self.applied = None;
    }
}

impl Drop for Engine {
    fn drop(&mut self) {
        self.reset();
    }
}
