// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Talkdedsec

use crate::color::{Matrix, Settings, IDENTITY};

#[derive(Debug)]
pub enum EngineError {
    Init,
    Apply,
}

impl std::fmt::Display for EngineError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Init => write!(f, "magnification engine could not start"),
            Self::Apply => write!(f, "color matrix was rejected by the system"),
        }
    }
}

impl std::error::Error for EngineError {}

pub struct Engine {
    applied: Option<Matrix>,
}

#[cfg(windows)]
mod sys {
    use super::{EngineError, Matrix};
    use windows::Win32::UI::Magnification::{
        MagInitialize, MagSetFullscreenColorEffect, MagUninitialize, MAGCOLOREFFECT,
    };

    pub fn init() -> Result<(), EngineError> {
        unsafe { MagInitialize() }.as_bool().then_some(()).ok_or(EngineError::Init)
    }

    pub fn set(matrix: &Matrix) -> Result<(), EngineError> {
        let effect = MAGCOLOREFFECT { transform: *matrix };
        unsafe { MagSetFullscreenColorEffect(&effect) }
            .as_bool()
            .then_some(())
            .ok_or(EngineError::Apply)
    }

    pub fn shutdown() {
        unsafe {
            let _ = MagUninitialize();
        }
    }
}

#[cfg(not(windows))]
mod sys {
    use super::{EngineError, Matrix};

    pub fn init() -> Result<(), EngineError> {
        Err(EngineError::Init)
    }
    pub fn set(_: &Matrix) -> Result<(), EngineError> {
        Err(EngineError::Apply)
    }
    pub fn shutdown() {}
}

impl Engine {
    pub fn new() -> Result<Self, EngineError> {
        sys::init()?;
        Ok(Self { applied: None })
    }

    pub fn apply(&mut self, settings: &Settings) -> Result<(), EngineError> {
        let matrix = settings.matrix();
        if self.applied == Some(matrix) {
            return Ok(());
        }
        sys::set(&matrix)?;
        self.applied = Some(matrix);
        Ok(())
    }

    pub fn reset(&mut self) -> Result<(), EngineError> {
        if self.applied == Some(IDENTITY) {
            return Ok(());
        }
        sys::set(&IDENTITY)?;
        self.applied = Some(IDENTITY);
        Ok(())
    }
}

impl Drop for Engine {
    fn drop(&mut self) {
        let _ = self.reset();
        sys::shutdown();
    }
}
