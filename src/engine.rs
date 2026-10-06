// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Talkdedsec

use std::collections::{HashMap, VecDeque};
use std::time::{Duration, Instant};

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

/// Something that replaces the ramp this often is fighting for the display, not
/// resetting it by accident. A game alt-tabbed in and out of exclusive fullscreen
/// stays under it; a second colour tool rewriting the ramp does not.
const FIGHT_WINDOW: Duration = Duration::from_secs(20);
const FIGHT_LIMIT: usize = 5;

/// How close a ramp found at startup has to be to one this program writes to
/// count as left behind by a run that never got to restore it. Drivers may
/// quantise what they store, so exact equality is too strict.
const LEFTOVER_TOLERANCE: u16 = 512;

/// One display as Windows reports it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Monitor {
    /// Survives the panel being unplugged and plugged back in.
    pub id: String,
    /// GDI device name, `\\.\DISPLAY1`, which the ramp is written through.
    pub device: String,
    /// What the panel calls itself, when its EDID says.
    pub name: Option<String>,
    /// A laptop's own panel rather than something on a cable.
    pub internal: bool,
    pub primary: bool,
}

/// Where ramps are read and written: Windows in the program, a fake in the tests.
pub trait Backend {
    fn scan(&mut self) -> Vec<Monitor>;
    fn get(&mut self, device: &str) -> Option<Ramp>;
    fn set(&mut self, device: &str, ramp: &Ramp) -> bool;
}

/// What one watch pass found and did.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Watch {
    /// A display was plugged in or taken away.
    pub displays_changed: bool,
    /// Displays whose ramp something else had replaced, written again.
    pub restored: usize,
    /// Displays left alone from now on, because something keeps replacing the ramp.
    pub gave_up: usize,
}

struct Output {
    monitor: Monitor,
    original: Ramp,
    enabled: bool,
    /// What the driver held right after the last write here; `None` while the
    /// original is on screen.
    written: Option<Ramp>,
    /// When something else recently replaced the ramp here.
    overwritten: VecDeque<Instant>,
    /// Given up on after too many replacements, until the next deliberate apply.
    contested: bool,
}

pub struct Engine<B: Backend = sys::Gdi> {
    backend: B,
    outputs: Vec<Output>,
    /// The original of every display seen this run, kept after it leaves, so a
    /// display that drops out and comes back is not taken for a fresh one.
    originals: HashMap<String, Ramp>,
    /// Displays the user left out, connected or not.
    excluded: Vec<String>,
    applied: Option<Ramp>,
    /// The step of the backoff ladder the driver took for `applied`.
    strength: Option<f32>,
}

impl Engine {
    /// `excluded` lists the displays to leave alone. `previous` is what the last
    /// run was showing, so a ramp it never got to restore is recognised as such.
    pub fn new(excluded: &[String], previous: &Settings) -> Result<Self, EngineError> {
        Self::with_backend(sys::Gdi, excluded, previous)
    }
}

impl<B: Backend> Engine<B> {
    pub fn with_backend(
        backend: B,
        excluded: &[String],
        previous: &Settings,
    ) -> Result<Self, EngineError> {
        let mut engine = Self {
            backend,
            outputs: Vec::new(),
            originals: HashMap::new(),
            excluded: excluded.to_vec(),
            applied: None,
            strength: None,
        };
        let leftovers = leftover_ramps(previous);
        for monitor in engine.backend.scan() {
            engine.adopt(monitor, &leftovers);
        }
        if engine.outputs.is_empty() {
            return Err(EngineError::NoDisplay);
        }
        engine.keep_one_enabled();
        Ok(engine)
    }

    pub fn displays(&self) -> impl Iterator<Item = (&Monitor, bool)> {
        self.outputs.iter().map(|o| (&o.monitor, o.enabled))
    }

    pub fn excluded(&self) -> &[String] {
        &self.excluded
    }

    /// How much of the asked-for setting is on screen: 1.0 in full, less where
    /// Windows clamped it, `None` while the displays show their originals.
    pub fn strength(&self) -> Option<f32> {
        self.strength
    }

    /// True while the engine has stepped aside on a display that something else
    /// keeps rewriting.
    pub fn paused(&self) -> bool {
        self.outputs.iter().any(|o| o.contested)
    }

    /// Takes a display on. Its ramp as found becomes the one to restore, unless
    /// it is one this program wrote and a previous run left behind.
    fn adopt(&mut self, monitor: Monitor, leftovers: &[Ramp]) -> bool {
        let Some(current) = self.backend.get(&monitor.device) else {
            return false;
        };
        let leftover = leftovers.iter().any(|ramp| near(ramp, &current));
        let original = *self
            .originals
            .entry(monitor.id.clone())
            .or_insert(if leftover { identity() } else { current });
        if leftover {
            // Nothing else is going to take it off the screen.
            self.backend.set(&monitor.device, &original);
        }
        self.outputs.push(Output {
            enabled: !self.excluded.contains(&monitor.id),
            monitor,
            original,
            written: None,
            overwritten: VecDeque::new(),
            contested: false,
        });
        true
    }

    /// The panel must always have somewhere to show the effect: if every
    /// connected display is left out, they all come back in.
    fn keep_one_enabled(&mut self) {
        if self.outputs.is_empty() || self.outputs.iter().any(|o| o.enabled) {
            return;
        }
        for output in &mut self.outputs {
            output.enabled = true;
        }
        let outputs = &self.outputs;
        self.excluded
            .retain(|id| !outputs.iter().any(|o| &o.monitor.id == id));
    }

    /// Writes `ramp` to one display and notes what the driver kept.
    fn write(&mut self, index: usize, ramp: &Ramp) -> bool {
        let Self {
            backend, outputs, ..
        } = self;
        let output = &mut outputs[index];
        if !backend.set(&output.monitor.device, ramp) {
            return false;
        }
        output.written = Some(backend.get(&output.monitor.device).unwrap_or(*ramp));
        true
    }

    fn restore(backend: &mut B, output: &mut Output) {
        if output.written.take().is_none() {
            return;
        }
        if !backend.set(&output.monitor.device, &output.original) {
            backend.set(&output.monitor.device, &identity());
        }
    }

    fn push(&mut self, ramp: &Ramp) -> bool {
        let mut any = false;
        for index in 0..self.outputs.len() {
            if self.outputs[index].enabled {
                any |= self.write(index, ramp);
            }
        }
        if any {
            self.applied = Some(*ramp);
        }
        any
    }

    pub fn apply(&mut self, settings: &Settings) -> Applied {
        // A deliberate change takes the display back from whatever was fighting for it.
        if self.outputs.iter().any(|o| o.contested) {
            self.applied = None;
        }
        for output in &mut self.outputs {
            output.contested = false;
            output.overwritten.clear();
        }

        if settings.is_neutral() {
            self.reset();
            return Applied::Full;
        }
        for factor in BACKOFF {
            let ramp = settings.scaled(factor).ramp();
            if self.applied == Some(ramp) || self.push(&ramp) {
                self.strength = Some(factor);
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
    /// is not optional. Displays this program never wrote to are not touched.
    pub fn reset(&mut self) {
        let Self {
            backend, outputs, ..
        } = self;
        for output in outputs.iter_mut() {
            Self::restore(backend, output);
        }
        self.applied = None;
        self.strength = None;
    }

    /// Includes or leaves out one display. Leaving out the last one is refused.
    pub fn set_enabled(&mut self, index: usize, on: bool) -> bool {
        let Some(output) = self.outputs.get(index) else {
            return false;
        };
        if output.enabled == on {
            return true;
        }
        if !on && self.outputs.iter().filter(|o| o.enabled).count() == 1 {
            return false;
        }
        let id = output.monitor.id.clone();
        self.outputs[index].enabled = on;
        if on {
            self.excluded.retain(|e| *e != id);
            if let Some(ramp) = self.applied {
                self.write(index, &ramp);
            }
        } else {
            self.excluded.push(id);
            let Self {
                backend, outputs, ..
            } = self;
            Self::restore(backend, &mut outputs[index]);
        }
        true
    }

    /// Meant to run about once a second. Notices displays coming and going, and
    /// puts the effect back where something replaced it: a game entering
    /// exclusive fullscreen, a panel waking up, a resolution change.
    pub fn watch(&mut self) -> Watch {
        let mut report = Watch::default();

        let seen = self.backend.scan();
        // An empty scan is a hiccup in enumeration, not every display unplugged at once.
        if !seen.is_empty() {
            let before = self.outputs.len();
            self.outputs
                .retain(|o| seen.iter().any(|m| m.id == o.monitor.id));
            report.displays_changed = self.outputs.len() != before;
            for monitor in seen {
                match self.outputs.iter_mut().find(|o| o.monitor.id == monitor.id) {
                    Some(output) => output.monitor = monitor,
                    None => report.displays_changed |= self.adopt(monitor, &[]),
                }
            }
            if report.displays_changed {
                self.keep_one_enabled();
            }
        }

        let Some(ramp) = self.applied else {
            return report;
        };
        let now = Instant::now();
        for index in 0..self.outputs.len() {
            let output = &self.outputs[index];
            if !output.enabled || output.contested {
                continue;
            }
            let Some(written) = output.written else {
                // Newly connected, or it refused the last write.
                self.write(index, &ramp);
                continue;
            };
            // Unreadable usually means the secure desktop or a sleeping panel.
            let Some(current) = self.backend.get(&output.monitor.device) else {
                continue;
            };
            if current == written {
                continue;
            }

            let output = &mut self.outputs[index];
            while output
                .overwritten
                .front()
                .is_some_and(|at| now.duration_since(*at) > FIGHT_WINDOW)
            {
                output.overwritten.pop_front();
            }
            output.overwritten.push_back(now);
            if output.overwritten.len() >= FIGHT_LIMIT {
                output.contested = true;
                // What is on screen now belongs to someone else; not ours to restore.
                output.written = None;
                report.gave_up += 1;
            } else if self.write(index, &ramp) {
                report.restored += 1;
            }
        }
        report
    }
}

impl<B: Backend> Drop for Engine<B> {
    fn drop(&mut self) {
        self.reset();
    }
}

/// Every ramp the last run could have left on screen: its settings at each step
/// of the backoff ladder.
fn leftover_ramps(previous: &Settings) -> Vec<Ramp> {
    if previous.is_neutral() {
        return Vec::new();
    }
    BACKOFF
        .iter()
        .map(|&factor| previous.scaled(factor).ramp())
        .collect()
}

fn near(a: &Ramp, b: &Ramp) -> bool {
    a.iter()
        .zip(b)
        .all(|(x, y)| x.abs_diff(*y) <= LEFTOVER_TOLERANCE)
}

#[cfg(windows)]
mod sys {
    use super::{Backend, Monitor};
    use crate::color::Ramp;
    use windows::core::{BOOL, PCWSTR};
    use windows::Win32::Devices::Display::{
        DisplayConfigGetDeviceInfo, GetDisplayConfigBufferSizes, QueryDisplayConfig,
        DISPLAYCONFIG_DEVICE_INFO_GET_SOURCE_NAME, DISPLAYCONFIG_DEVICE_INFO_GET_TARGET_NAME,
        DISPLAYCONFIG_DEVICE_INFO_HEADER, DISPLAYCONFIG_DEVICE_INFO_TYPE, DISPLAYCONFIG_MODE_INFO,
        DISPLAYCONFIG_OUTPUT_TECHNOLOGY_DISPLAYPORT_EMBEDDED,
        DISPLAYCONFIG_OUTPUT_TECHNOLOGY_INTERNAL, DISPLAYCONFIG_OUTPUT_TECHNOLOGY_UDI_EMBEDDED,
        DISPLAYCONFIG_PATH_INFO, DISPLAYCONFIG_SOURCE_DEVICE_NAME,
        DISPLAYCONFIG_TARGET_DEVICE_NAME, QDC_ONLY_ACTIVE_PATHS,
    };
    use windows::Win32::Foundation::{ERROR_INSUFFICIENT_BUFFER, LUID};
    use windows::Win32::Graphics::Gdi::{
        CreateDCW, DeleteDC, EnumDisplayDevicesW, DISPLAY_DEVICEW,
        DISPLAY_DEVICE_ATTACHED_TO_DESKTOP, DISPLAY_DEVICE_PRIMARY_DEVICE,
        DISPLAY_DEVICE_STATE_FLAGS, HDC,
    };

    // windows-rs 0.62 does not bind the gamma ramp calls, so declare them here.
    #[link(name = "gdi32")]
    unsafe extern "system" {
        fn SetDeviceGammaRamp(hdc: HDC, lpramp: *const core::ffi::c_void) -> BOOL;
        fn GetDeviceGammaRamp(hdc: HDC, lpramp: *mut core::ffi::c_void) -> BOOL;
    }

    pub struct Gdi;

    /// A device context on one display, deleted when dropped.
    struct Dc(HDC);

    impl Dc {
        fn open(device: &str) -> Option<Self> {
            let name = wide(device);
            let hdc = unsafe { CreateDCW(PCWSTR(name.as_ptr()), None, None, None) };
            (!hdc.is_invalid()).then_some(Self(hdc))
        }
    }

    impl Drop for Dc {
        fn drop(&mut self) {
            unsafe {
                let _ = DeleteDC(self.0);
            }
        }
    }

    fn wide(text: &str) -> Vec<u16> {
        text.encode_utf16().chain(std::iter::once(0)).collect()
    }

    fn narrow(buffer: &[u16]) -> String {
        let end = buffer.iter().position(|c| *c == 0).unwrap_or(buffer.len());
        String::from_utf16_lossy(&buffer[..end])
    }

    /// What the display configuration API knows about the panel behind one GDI device.
    struct Target {
        device: String,
        path: String,
        name: String,
        internal: bool,
    }

    fn active_paths() -> Vec<DISPLAYCONFIG_PATH_INFO> {
        // The topology can change between sizing the buffers and filling them.
        for _ in 0..3 {
            let (mut path_count, mut mode_count) = (0u32, 0u32);
            let sized = unsafe {
                GetDisplayConfigBufferSizes(QDC_ONLY_ACTIVE_PATHS, &mut path_count, &mut mode_count)
            };
            if sized.is_err() {
                break;
            }
            let mut paths = vec![DISPLAYCONFIG_PATH_INFO::default(); path_count as usize];
            let mut modes = vec![DISPLAYCONFIG_MODE_INFO::default(); mode_count as usize];
            let result = unsafe {
                QueryDisplayConfig(
                    QDC_ONLY_ACTIVE_PATHS,
                    &mut path_count,
                    paths.as_mut_ptr(),
                    &mut mode_count,
                    modes.as_mut_ptr(),
                    None,
                )
            };
            if result == ERROR_INSUFFICIENT_BUFFER {
                continue;
            }
            if result.is_ok() {
                paths.truncate(path_count as usize);
                return paths;
            }
            break;
        }
        Vec::new()
    }

    fn header<T>(
        kind: DISPLAYCONFIG_DEVICE_INFO_TYPE,
        adapter: LUID,
        id: u32,
    ) -> DISPLAYCONFIG_DEVICE_INFO_HEADER {
        DISPLAYCONFIG_DEVICE_INFO_HEADER {
            r#type: kind,
            size: std::mem::size_of::<T>() as u32,
            adapterId: adapter,
            id,
        }
    }

    fn targets() -> Vec<Target> {
        active_paths()
            .iter()
            .filter_map(|path| {
                let mut source = DISPLAYCONFIG_SOURCE_DEVICE_NAME {
                    header: header::<DISPLAYCONFIG_SOURCE_DEVICE_NAME>(
                        DISPLAYCONFIG_DEVICE_INFO_GET_SOURCE_NAME,
                        path.sourceInfo.adapterId,
                        path.sourceInfo.id,
                    ),
                    ..Default::default()
                };
                // The pointer covers the whole packet; the call writes past the header.
                if unsafe { DisplayConfigGetDeviceInfo(std::ptr::addr_of_mut!(source).cast()) } != 0
                {
                    return None;
                }

                let mut target = DISPLAYCONFIG_TARGET_DEVICE_NAME {
                    header: header::<DISPLAYCONFIG_TARGET_DEVICE_NAME>(
                        DISPLAYCONFIG_DEVICE_INFO_GET_TARGET_NAME,
                        path.targetInfo.adapterId,
                        path.targetInfo.id,
                    ),
                    ..Default::default()
                };
                if unsafe { DisplayConfigGetDeviceInfo(std::ptr::addr_of_mut!(target).cast()) } != 0
                {
                    return None;
                }

                let technology = target.outputTechnology;
                Some(Target {
                    device: narrow(&source.viewGdiDeviceName),
                    path: narrow(&target.monitorDevicePath),
                    name: narrow(&target.monitorFriendlyDeviceName),
                    internal: technology == DISPLAYCONFIG_OUTPUT_TECHNOLOGY_INTERNAL
                        || technology == DISPLAYCONFIG_OUTPUT_TECHNOLOGY_DISPLAYPORT_EMBEDDED
                        || technology == DISPLAYCONFIG_OUTPUT_TECHNOLOGY_UDI_EMBEDDED,
                })
            })
            .collect()
    }

    impl Backend for Gdi {
        fn scan(&mut self) -> Vec<Monitor> {
            let targets = targets();
            let mut found = Vec::new();
            for index in 0u32.. {
                let mut adapter = DISPLAY_DEVICEW {
                    cb: std::mem::size_of::<DISPLAY_DEVICEW>() as u32,
                    ..Default::default()
                };
                if !unsafe { EnumDisplayDevicesW(None, index, &mut adapter, 0) }.as_bool() {
                    break;
                }
                if adapter.StateFlags & DISPLAY_DEVICE_ATTACHED_TO_DESKTOP
                    == DISPLAY_DEVICE_STATE_FLAGS(0)
                {
                    continue;
                }
                let device = narrow(&adapter.DeviceName);
                let target = targets.iter().find(|t| t.device == device);
                found.push(Monitor {
                    id: target
                        .map(|t| t.path.clone())
                        .filter(|p| !p.is_empty())
                        .unwrap_or_else(|| device.clone()),
                    name: target
                        .map(|t| t.name.trim().to_string())
                        .filter(|n| !n.is_empty()),
                    internal: target.is_some_and(|t| t.internal),
                    primary: adapter.StateFlags & DISPLAY_DEVICE_PRIMARY_DEVICE
                        != DISPLAY_DEVICE_STATE_FLAGS(0),
                    device,
                });
            }
            // Main display first, then DISPLAY2 before DISPLAY10.
            found.sort_by(|a, b| {
                b.primary
                    .cmp(&a.primary)
                    .then(a.device.len().cmp(&b.device.len()))
                    .then_with(|| a.device.cmp(&b.device))
            });
            found
        }

        fn get(&mut self, device: &str) -> Option<Ramp> {
            let dc = Dc::open(device)?;
            let mut ramp = [0u16; 768];
            unsafe { GetDeviceGammaRamp(dc.0, ramp.as_mut_ptr().cast()) }
                .as_bool()
                .then_some(ramp)
        }

        fn set(&mut self, device: &str, ramp: &Ramp) -> bool {
            let Some(dc) = Dc::open(device) else {
                return false;
            };
            unsafe { SetDeviceGammaRamp(dc.0, ramp.as_ptr().cast()) }.as_bool()
        }
    }
}

#[cfg(not(windows))]
mod sys {
    use super::{Backend, Monitor};
    use crate::color::Ramp;

    pub struct Gdi;

    impl Backend for Gdi {
        fn scan(&mut self) -> Vec<Monitor> {
            Vec::new()
        }
        fn get(&mut self, _: &str) -> Option<Ramp> {
            None
        }
        fn set(&mut self, _: &str, _: &Ramp) -> bool {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::rc::Rc;

    /// A pretend graphics driver: one ramp per connected display, and an
    /// optional limit on how far from linear it lets a ramp stray, like Windows
    /// without `GdiIcmGammaRange`.
    #[derive(Default)]
    struct Screens {
        monitors: Vec<Monitor>,
        ramps: HashMap<String, Ramp>,
        limit: Option<u16>,
    }

    /// The engine owns its backend, so the test keeps a second handle on the
    /// same screens to play the part of Windows, games and other programs.
    #[derive(Clone, Default)]
    struct Fake(Rc<RefCell<Screens>>);

    impl Fake {
        fn with(devices: &[&str]) -> Self {
            let fake = Self::default();
            for device in devices {
                fake.plug(device);
            }
            fake
        }

        fn plug(&self, device: &str) {
            let mut screens = self.0.borrow_mut();
            let primary = screens.monitors.is_empty();
            screens.monitors.push(Monitor {
                id: format!("path-{device}"),
                device: device.to_string(),
                name: Some(format!("Panel {device}")),
                internal: false,
                primary,
            });
            screens.ramps.insert(device.to_string(), identity());
        }

        fn unplug(&self, device: &str) {
            let mut screens = self.0.borrow_mut();
            screens.monitors.retain(|m| m.device != device);
            screens.ramps.remove(device);
        }

        /// Something other than the engine writes a ramp: a game, Windows, another tool.
        fn paint(&self, device: &str, ramp: Ramp) {
            self.0.borrow_mut().ramps.insert(device.to_string(), ramp);
        }

        fn ramp(&self, device: &str) -> Ramp {
            self.0.borrow().ramps[device]
        }
    }

    impl Backend for Fake {
        fn scan(&mut self) -> Vec<Monitor> {
            self.0.borrow().monitors.clone()
        }

        fn get(&mut self, device: &str) -> Option<Ramp> {
            self.0.borrow().ramps.get(device).copied()
        }

        fn set(&mut self, device: &str, ramp: &Ramp) -> bool {
            let mut screens = self.0.borrow_mut();
            if !screens.ramps.contains_key(device) {
                return false;
            }
            if screens
                .limit
                .is_some_and(|limit| distance(ramp, &identity()) > limit)
            {
                return false;
            }
            screens.ramps.insert(device.to_string(), *ramp);
            true
        }
    }

    fn distance(a: &Ramp, b: &Ramp) -> u16 {
        a.iter()
            .zip(b)
            .map(|(x, y)| x.abs_diff(*y))
            .max()
            .unwrap_or(0)
    }

    fn engine(fake: &Fake) -> Engine<Fake> {
        Engine::with_backend(fake.clone(), &[], &Settings::default()).unwrap()
    }

    fn warm() -> Settings {
        Settings {
            temperature: 0.6,
            ..Default::default()
        }
    }

    /// A calibration curve some other tool loaded before the program started.
    fn calibration() -> Ramp {
        Settings {
            gamma: 1.1,
            ..Default::default()
        }
        .ramp()
    }

    #[test]
    fn no_display_is_an_error() {
        let fake = Fake::default();
        assert!(Engine::with_backend(fake, &[], &Settings::default()).is_err());
    }

    #[test]
    fn applies_everywhere_and_puts_each_original_back() {
        let fake = Fake::with(&["A", "B"]);
        fake.paint("A", calibration());
        let mut engine = engine(&fake);

        assert_eq!(engine.apply(&warm()), Applied::Full);
        assert_eq!(fake.ramp("A"), warm().ramp());
        assert_eq!(fake.ramp("B"), warm().ramp());

        drop(engine);
        assert_eq!(fake.ramp("A"), calibration());
        assert_eq!(fake.ramp("B"), identity());
    }

    #[test]
    fn neutral_settings_restore_the_original() {
        let fake = Fake::with(&["A"]);
        let mut engine = engine(&fake);
        engine.apply(&warm());
        assert_eq!(engine.apply(&Settings::default()), Applied::Full);
        assert_eq!(fake.ramp("A"), identity());
    }

    #[test]
    fn refused_strength_is_walked_down() {
        let strong = Settings {
            contrast: 1.8,
            gamma: 2.2,
            ..Default::default()
        };
        let fake = Fake::with(&["A"]);
        fake.0.borrow_mut().limit = Some(distance(&strong.scaled(0.6).ramp(), &identity()));
        let mut engine = engine(&fake);

        let Applied::Limited(factor) = engine.apply(&strong) else {
            panic!("expected a limited apply");
        };
        assert!(factor < 1.0);
        assert_eq!(fake.ramp("A"), strong.scaled(factor).ramp());

        fake.0.borrow_mut().limit = Some(0);
        assert_eq!(engine.apply(&warm()), Applied::Rejected);
    }

    #[test]
    fn an_excluded_display_is_left_alone() {
        let fake = Fake::with(&["A", "B"]);
        fake.paint("B", calibration());
        let mut engine =
            Engine::with_backend(fake.clone(), &["path-B".into()], &Settings::default()).unwrap();

        engine.apply(&warm());
        assert_eq!(fake.ramp("A"), warm().ramp());
        assert_eq!(fake.ramp("B"), calibration());
        drop(engine);
        assert_eq!(fake.ramp("B"), calibration());
    }

    #[test]
    fn leaving_a_display_out_restores_it_and_the_last_one_stays() {
        let fake = Fake::with(&["A", "B"]);
        let mut engine = engine(&fake);
        engine.apply(&warm());

        assert!(engine.set_enabled(1, false));
        assert_eq!(fake.ramp("B"), identity());
        assert_eq!(engine.excluded(), ["path-B".to_string()]);

        assert!(
            !engine.set_enabled(0, false),
            "the last display was left out"
        );
        assert_eq!(fake.ramp("A"), warm().ramp());
        assert!(!engine.set_enabled(7, true), "no such display");
    }

    #[test]
    fn including_a_display_brings_the_effect_to_it() {
        let fake = Fake::with(&["A", "B"]);
        let mut engine =
            Engine::with_backend(fake.clone(), &["path-B".into()], &Settings::default()).unwrap();
        engine.apply(&warm());

        assert!(engine.set_enabled(1, true));
        assert_eq!(fake.ramp("B"), warm().ramp());
        assert!(engine.excluded().is_empty());
    }

    #[test]
    fn excluding_every_connected_display_brings_them_all_back() {
        let fake = Fake::with(&["A", "B"]);
        let engine = Engine::with_backend(
            fake,
            &["path-A".into(), "path-B".into(), "path-gone".into()],
            &Settings::default(),
        )
        .unwrap();
        assert!(engine.displays().all(|(_, on)| on));
        assert_eq!(engine.excluded(), ["path-gone".to_string()]);
    }

    #[test]
    fn watch_puts_back_a_ramp_something_reset() {
        let fake = Fake::with(&["A", "B"]);
        let mut engine = engine(&fake);
        engine.apply(&warm());

        assert_eq!(engine.watch(), Watch::default(), "nothing to do yet");

        fake.paint("A", identity());
        let report = engine.watch();
        assert_eq!(report.restored, 1);
        assert_eq!(fake.ramp("A"), warm().ramp());
        assert_eq!(fake.ramp("B"), warm().ramp());
    }

    #[test]
    fn watch_leaves_the_display_alone_while_the_effect_is_off() {
        let fake = Fake::with(&["A"]);
        let mut engine = engine(&fake);
        engine.apply(&warm());
        engine.reset();

        fake.paint("A", calibration());
        assert_eq!(engine.watch(), Watch::default());
        assert_eq!(fake.ramp("A"), calibration());
    }

    #[test]
    fn watch_gives_up_on_a_fight_and_a_deliberate_apply_takes_over() {
        let fake = Fake::with(&["A"]);
        let mut engine = engine(&fake);
        engine.apply(&warm());

        let mut gave_up = 0;
        for _ in 0..FIGHT_LIMIT {
            fake.paint("A", calibration());
            gave_up += engine.watch().gave_up;
        }
        assert_eq!(gave_up, 1);
        assert_eq!(fake.ramp("A"), calibration(), "the other program keeps it");

        assert_eq!(engine.watch(), Watch::default(), "no more fighting");
        assert_eq!(fake.ramp("A"), calibration());

        drop(engine);
        assert_eq!(
            fake.ramp("A"),
            calibration(),
            "not ours to restore once given up"
        );
    }

    #[test]
    fn strength_and_pause_report_what_is_on_screen() {
        let strong = Settings {
            contrast: 1.8,
            gamma: 2.2,
            ..Default::default()
        };
        let fake = Fake::with(&["A"]);
        let mut engine = engine(&fake);
        assert_eq!(engine.strength(), None);

        engine.apply(&warm());
        assert_eq!(engine.strength(), Some(1.0));

        fake.0.borrow_mut().limit = Some(distance(&strong.scaled(0.6).ramp(), &identity()));
        let Applied::Limited(factor) = engine.apply(&strong) else {
            panic!("expected a limited apply");
        };
        assert_eq!(engine.strength(), Some(factor));
        fake.0.borrow_mut().limit = None;

        for _ in 0..FIGHT_LIMIT {
            fake.paint("A", calibration());
            engine.watch();
        }
        assert!(engine.paused());
        engine.apply(&warm());
        assert!(!engine.paused());

        engine.apply(&Settings::default());
        assert_eq!(engine.strength(), None);
    }

    #[test]
    fn applying_again_after_giving_up_writes_at_once() {
        let fake = Fake::with(&["A"]);
        let mut engine = engine(&fake);
        engine.apply(&warm());
        for _ in 0..FIGHT_LIMIT {
            fake.paint("A", calibration());
            engine.watch();
        }

        assert_eq!(engine.apply(&warm()), Applied::Full);
        assert_eq!(fake.ramp("A"), warm().ramp());
    }

    #[test]
    fn watch_adopts_new_displays_and_drops_gone_ones() {
        let fake = Fake::with(&["A", "B"]);
        let mut engine = engine(&fake);
        engine.apply(&warm());

        fake.plug("C");
        assert!(engine.watch().displays_changed);
        assert_eq!(fake.ramp("C"), warm().ramp());

        fake.unplug("B");
        assert!(engine.watch().displays_changed);
        let devices: Vec<_> = engine.displays().map(|(m, _)| m.device.clone()).collect();
        assert_eq!(devices, ["A", "C"]);

        drop(engine);
        assert_eq!(fake.ramp("C"), identity());
    }

    #[test]
    fn a_display_that_drops_out_and_returns_keeps_its_first_original() {
        let fake = Fake::with(&["A"]);
        fake.paint("A", calibration());
        let mut engine = engine(&fake);
        engine.apply(&warm());

        // Enumeration loses the display for a moment while our ramp stays on it.
        let monitor = fake.0.borrow_mut().monitors.remove(0);
        fake.plug("B");
        engine.watch();
        fake.0.borrow_mut().monitors.push(monitor);
        engine.watch();

        drop(engine);
        assert_eq!(fake.ramp("A"), calibration());
    }

    #[test]
    fn an_empty_scan_is_ignored() {
        let fake = Fake::with(&["A"]);
        let mut engine = engine(&fake);
        engine.apply(&warm());

        fake.0.borrow_mut().monitors.clear();
        assert!(!engine.watch().displays_changed);
        assert_eq!(engine.displays().count(), 1);
    }

    #[test]
    fn a_ramp_left_by_a_run_that_died_is_not_taken_for_the_original() {
        let fake = Fake::with(&["A", "B"]);
        fake.paint("A", warm().scaled(0.85).ramp());
        fake.paint("B", calibration());

        let engine = Engine::with_backend(fake.clone(), &[], &warm()).unwrap();
        assert_eq!(
            fake.ramp("A"),
            identity(),
            "the leftover is cleared at once"
        );
        assert_eq!(fake.ramp("B"), calibration(), "a real original is kept");

        drop(engine);
        assert_eq!(fake.ramp("A"), identity());
        assert_eq!(fake.ramp("B"), calibration());
    }
}
