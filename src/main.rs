// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Talkdedsec

#![windows_subsystem = "windows"]

mod color;
mod engine;
mod i18n;
mod presets;
mod preview;
mod profiles;
mod system;

use std::cell::RefCell;
use std::fmt::Write;
use std::rc::Rc;
use std::time::Duration;

use color::Settings;
use engine::{Applied, Engine, Monitor};
use i18n::{fill, number, t, Lang};
use preview::Scene;
use profiles::Store;
use slint::{ModelRc, VecModel};
use tray_icon::menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use tray_icon::{Icon, TrayIconBuilder};

slint::include_modules!();

const REPO_URL: &str = "https://github.com/Talkdedsec/tlk-visual";
const VERSION: &str = env!("CARGO_PKG_VERSION");
/// Logical height under which the left rail drops its source-code card so the
/// presets and profiles still fit.
const COMPACT_BELOW: f32 = 760.0;

struct App {
    settings: Settings,
    auto_apply: bool,
    engine: Option<Engine>,
    scene: Scene,
    store: Store,
    tray_on_close: bool,
}

impl App {
    fn new() -> Self {
        let store = Store::load();
        Self {
            settings: store.last,
            auto_apply: store.auto_apply,
            tray_on_close: store.tray_on_close,
            engine: None,
            scene: Scene::new(1040, 362),
            store,
        }
    }

    /// Writes the store when anything in it is out of date. Runs every second, so
    /// shutting Windows down with the window in the tray keeps the last change.
    fn persist(&mut self) {
        let excluded = self.engine.as_ref().map_or_else(
            || self.store.excluded_displays.clone(),
            |e| e.excluded().to_vec(),
        );
        if self.store.last == self.settings
            && self.store.auto_apply == self.auto_apply
            && self.store.tray_on_close == self.tray_on_close
            && self.store.excluded_displays == excluded
        {
            return;
        }
        self.store.last = self.settings;
        self.store.auto_apply = self.auto_apply;
        self.store.tray_on_close = self.tray_on_close;
        self.store.excluded_displays = excluded;
        self.store.save();
    }

    fn push(&mut self) -> String {
        if !self.auto_apply {
            return t("Waiting.").into();
        }
        self.force_apply()
    }

    fn force_apply(&mut self) -> String {
        let neutral = self.settings.is_neutral();
        let Some(engine) = self.engine.as_mut() else {
            return t("The display driver does not accept a gamma ramp.").into();
        };
        match engine.apply(&self.settings) {
            Applied::Full if neutral => t("Ready.").into(),
            Applied::Full => t("Applied.").into(),
            Applied::Limited(f) => fill(
                t("Windows refused full strength — {}% applied."),
                &[&((f * 100.0).round() as i32)],
            ),
            Applied::Rejected => t("Windows refused this setting.").into(),
        }
    }

    fn detail(&self) -> String {
        if !self.auto_apply {
            return t("Auto-apply is off — display untouched.").into();
        }
        let s = self.settings;
        let d = Settings::default();
        let mut parts = Vec::new();
        if (s.brightness - d.brightness).abs() > 1e-4 {
            parts.push(fill(t("Brightness {}"), &[&number(s.brightness, 2, true)]));
        }
        if (s.contrast - d.contrast).abs() > 1e-4 {
            parts.push(fill(t("Contrast {}"), &[&number(s.contrast, 2, false)]));
        }
        if (s.gamma - d.gamma).abs() > 1e-4 {
            parts.push(fill(t("Gamma {}"), &[&number(s.gamma, 2, false)]));
        }
        if s.temperature.abs() > 1e-4 {
            parts.push(fill(
                t("Temperature {}"),
                &[&number(s.temperature, 2, true)],
            ));
        }
        if s.night_vision.abs() > 1e-4 {
            parts.push(fill(
                t("Night vision {}"),
                &[&number(s.night_vision, 2, false)],
            ));
        }
        if parts.is_empty() {
            t("Display untouched.").into()
        } else {
            parts.join(" · ")
        }
    }
}

/// One channel of the transfer curve, which is what a gamma ramp really is, as
/// path commands in a 255 × 255 box with y pointing down. 65 points are enough
/// for a line a few hundred pixels wide.
fn curve_path(settings: &Settings, channel: usize) -> String {
    let mut path = String::with_capacity(800);
    for step in 0..=64usize {
        let level = (step * 4).min(255) as f32;
        let out = 255.0 - settings.channel(level / 255.0, channel) * 255.0;
        let verb = if step == 0 { 'M' } else { 'L' };
        let _ = write!(path, "{verb}{level:.0} {out:.1} ");
    }
    path.truncate(path.trim_end().len());
    path
}

/// The two figures under the curve: how much of the setting Windows let through,
/// and on how many displays.
fn engine_figures(app: &App) -> (String, bool, String) {
    let Some(engine) = app.engine.as_ref() else {
        return ("—".into(), false, "—".into());
    };
    let total = engine.displays().count();
    let on = engine.displays().filter(|(_, on)| *on).count();
    let (strength, warn) = if engine.paused() {
        (t("Paused").to_string(), true)
    } else {
        match engine.strength() {
            None if !app.auto_apply => (t("Off").to_string(), false),
            None => ("—".to_string(), false),
            Some(f) => (fill(t("{}%"), &[&((f * 100.0).round() as i32)]), f < 1.0),
        }
    };
    (strength, warn, format!("{on} / {total}"))
}

fn show_engine(ui: &MainWindow, app: &App) {
    let (strength, warn, displays) = engine_figures(app);
    ui.set_strength_text(strength.into());
    ui.set_strength_warn(warn);
    ui.set_displays_text(displays.into());
}

/// The sliders stop where the engine clamps, so neither end has a dead zone.
fn share_ranges(ui: &MainWindow) {
    let ranges = ui.global::<Ranges>();
    ranges.set_brightness_min(Settings::BRIGHTNESS_RANGE.0);
    ranges.set_brightness_max(Settings::BRIGHTNESS_RANGE.1);
    ranges.set_contrast_min(Settings::CONTRAST_RANGE.0);
    ranges.set_contrast_max(Settings::CONTRAST_RANGE.1);
    ranges.set_gamma_min(Settings::GAMMA_RANGE.0);
    ranges.set_gamma_max(Settings::GAMMA_RANGE.1);
    ranges.set_temperature_min(Settings::TEMPERATURE_RANGE.0);
    ranges.set_temperature_max(Settings::TEMPERATURE_RANGE.1);
    ranges.set_night_vision_min(Settings::NIGHT_VISION_RANGE.0);
    ranges.set_night_vision_max(Settings::NIGHT_VISION_RANGE.1);
}

/// Numbered in the order the settings show them, main display first.
fn display_label(index: usize, monitor: &Monitor) -> String {
    let name = match &monitor.name {
        Some(name) => name.as_str(),
        None if monitor.internal => t("Built-in display"),
        None => t("Display"),
    };
    format!("{} · {name}", index + 1)
}

fn display_rows(engine: Option<&Engine>) -> Vec<DisplayEntry> {
    engine
        .map(|engine| {
            engine
                .displays()
                .enumerate()
                .map(|(index, (monitor, on))| DisplayEntry {
                    name: display_label(index, monitor).into(),
                    on,
                })
                .collect()
        })
        .unwrap_or_default()
}

fn sync(ui: &MainWindow, app: &App, status: String) {
    let s = app.settings;
    ui.set_brightness(s.brightness);
    ui.set_contrast(s.contrast);
    ui.set_gamma(s.gamma);
    ui.set_temperature(s.temperature);
    ui.set_night_vision(s.night_vision);

    ui.set_brightness_text(number(s.brightness, 2, true).into());
    ui.set_contrast_text(number(s.contrast, 2, false).into());
    ui.set_gamma_text(number(s.gamma, 2, false).into());
    ui.set_temperature_text(number(s.temperature, 2, true).into());
    ui.set_night_vision_text(number(s.night_vision, 2, false).into());

    ui.set_preview_after(app.scene.render(&s));
    ui.set_curve_red(curve_path(&s, 0).into());
    ui.set_curve_green(curve_path(&s, 1).into());
    ui.set_curve_blue(curve_path(&s, 2).into());
    // Only temperature pulls the channels apart; until then one line says it all.
    ui.set_curve_mono(s.clamped().temperature.abs() < 1e-4);
    ui.set_selected_profile(
        app.store
            .profiles
            .iter()
            .position(|p| p.settings == s)
            .map_or(-1, |i| i as i32),
    );
    ui.set_engine_active(!s.is_neutral() && app.auto_apply && app.engine.is_some());
    ui.set_status_text(status.into());
    ui.set_status_detail(app.detail().into());
    show_engine(ui, app);
}

/// Rust-side text and Slint's bundled catalog switch together. Slint only knows
/// its catalogs once a component exists, so this runs after the window is made.
fn use_language(ui: &MainWindow, lang: Lang) {
    i18n::set(lang);
    let _ = slint::select_bundled_translation(lang.code());
    ui.set_language(lang.code().into());
}

/// The tray menu lives outside Slint, so it is relabelled by hand.
fn label_tray(show: &MenuItem, toggle: &MenuItem, quit: &MenuItem) {
    show.set_text(t("Show window"));
    toggle.set_text(t("Turn the filter on or off"));
    quit.set_text(t("Quit"));
}

/// ShellExecuteW rather than `cmd /C start`, so no console window ever flashes.
fn open_url(url: &str) {
    #[cfg(windows)]
    unsafe {
        use windows::core::HSTRING;
        use windows::Win32::UI::Shell::ShellExecuteW;
        use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

        let verb = HSTRING::from("open");
        let target = HSTRING::from(url);
        let _ = ShellExecuteW(None, &verb, &target, None, None, SW_SHOWNORMAL);
    }
    #[cfg(not(windows))]
    let _ = url;
}

fn main() -> Result<(), slint::PlatformError> {
    let system::Instance::First(summons) = system::claim() else {
        return Ok(());
    };

    let ui = MainWindow::new()?;
    let app = Rc::new(RefCell::new(App::new()));

    let lang = app
        .borrow()
        .store
        .language
        .as_deref()
        .and_then(Lang::from_code)
        .unwrap_or_else(Lang::system);
    use_language(&ui, lang);

    let engine = {
        let app = app.borrow();
        Engine::new(&app.store.excluded_displays, &app.store.last)
    };
    let startup = match engine {
        Ok(engine) => {
            app.borrow_mut().engine = Some(engine);
            t("Ready.").to_string()
        }
        Err(_) => t("The display driver does not accept a gamma ramp.").to_string(),
    };

    let display_model = Rc::new(VecModel::from(display_rows(app.borrow().engine.as_ref())));
    ui.set_displays(ModelRc::from(display_model.clone()));

    let thumbs = Scene::thumbnail(160, 92);
    let preset_model = Rc::new(VecModel::from(presets::ui_models(&thumbs)));
    ui.set_presets(ModelRc::from(preset_model.clone()));

    let profile_model = Rc::new(VecModel::<slint::SharedString>::default());
    profile_model.set_vec(app.borrow().store.names());
    ui.set_profiles(ModelRc::from(profile_model.clone()));

    share_ranges(&ui);
    ui.set_preview_before(app.borrow().scene.render(&Settings::default()));
    ui.set_version(VERSION.into());
    ui.set_hotkeys(ModelRc::from(Rc::new(VecModel::from(
        system::HOTKEYS
            .iter()
            .map(|(name, _)| slint::SharedString::from(*name))
            .collect::<Vec<_>>(),
    ))));
    ui.set_autostart(system::autostart_enabled());
    ui.set_tray_on_close(app.borrow().tray_on_close);
    ui.set_hotkey_index(system::hotkey_index(&app.borrow().store.hotkey) as i32);
    ui.set_auto_apply(app.borrow().auto_apply);
    ui.set_selected_preset(presets::index_of(&app.borrow().settings));

    {
        let mut app_mut = app.borrow_mut();
        let status = if app_mut.settings.is_neutral() {
            startup
        } else {
            app_mut.push()
        };
        sync(&ui, &app_mut, status);
    }

    ui.on_changed({
        let ui = ui.as_weak();
        let app = app.clone();
        move |field, value| {
            let ui = ui.unwrap();
            let mut app = app.borrow_mut();
            match field.as_str() {
                "brightness" => app.settings.brightness = value,
                "contrast" => app.settings.contrast = value,
                "gamma" => app.settings.gamma = value,
                "temperature" => app.settings.temperature = value,
                "night_vision" => app.settings.night_vision = value,
                _ => return,
            }
            app.settings = app.settings.clamped();
            ui.set_selected_preset(presets::index_of(&app.settings));
            let status = app.push();
            sync(&ui, &app, status);
        }
    });

    ui.on_committed({
        let ui = ui.as_weak();
        let app = app.clone();
        move |field, text| {
            let ui = ui.unwrap();
            let Ok(value) = text.replace(',', ".").trim().parse::<f32>() else {
                let app = app.borrow();
                sync(&ui, &app, t("Could not read the number.").into());
                return;
            };
            let mut app = app.borrow_mut();
            match field.as_str() {
                "brightness" => app.settings.brightness = value,
                "contrast" => app.settings.contrast = value,
                "gamma" => app.settings.gamma = value,
                "temperature" => app.settings.temperature = value,
                "night_vision" => app.settings.night_vision = value,
                _ => return,
            }
            app.settings = app.settings.clamped();
            ui.set_selected_preset(presets::index_of(&app.settings));
            let status = app.push();
            sync(&ui, &app, status);
        }
    });

    ui.on_compare({
        let ui = ui.as_weak();
        let app = app.clone();
        move |holding| {
            let ui = ui.unwrap();
            let mut app = app.borrow_mut();
            ui.set_comparing(holding);
            let status = if holding {
                if let Some(engine) = app.engine.as_mut() {
                    engine.reset();
                }
                t("Original image.").to_string()
            } else {
                app.push()
            };
            ui.set_status_text(status.into());
            show_engine(&ui, &app);
        }
    });

    ui.on_reset_block({
        let ui = ui.as_weak();
        let app = app.clone();
        move |block| {
            let ui = ui.unwrap();
            let mut app = app.borrow_mut();
            let d = Settings::default();
            match block.as_str() {
                "light" => {
                    app.settings.brightness = d.brightness;
                    app.settings.contrast = d.contrast;
                }
                "color" => {
                    app.settings.gamma = d.gamma;
                    app.settings.temperature = d.temperature;
                }
                "vision" => app.settings.night_vision = d.night_vision,
                _ => return,
            }
            ui.set_selected_preset(presets::index_of(&app.settings));
            let status = app.push();
            sync(&ui, &app, status);
        }
    });

    ui.on_reset_all({
        let ui = ui.as_weak();
        let app = app.clone();
        move || {
            let ui = ui.unwrap();
            let mut app = app.borrow_mut();
            app.settings = Settings::default();
            if let Some(engine) = app.engine.as_mut() {
                engine.reset();
            }
            ui.set_selected_preset(0);
            sync(&ui, &app, t("Back to defaults.").into());
        }
    });

    ui.on_apply_now({
        let ui = ui.as_weak();
        let app = app.clone();
        move || {
            let ui = ui.unwrap();
            let mut app = app.borrow_mut();
            let status = app.force_apply();
            sync(&ui, &app, status);
        }
    });

    ui.on_toggle_auto({
        let ui = ui.as_weak();
        let app = app.clone();
        move |on| {
            let ui = ui.unwrap();
            let mut app = app.borrow_mut();
            app.auto_apply = on;
            let status = if on {
                app.push()
            } else {
                if let Some(engine) = app.engine.as_mut() {
                    engine.reset();
                }
                t("Waiting.").into()
            };
            sync(&ui, &app, status);
        }
    });

    ui.on_pick_preset({
        let ui = ui.as_weak();
        let app = app.clone();
        move |index| {
            let ui = ui.unwrap();
            let Some(preset) = presets::at(index as usize) else {
                return;
            };
            let mut app = app.borrow_mut();
            app.settings = preset.settings;
            ui.set_selected_preset(index);
            let status = app.push();
            sync(&ui, &app, status);
        }
    });

    ui.on_save_profile({
        let ui = ui.as_weak();
        let app = app.clone();
        let model = profile_model.clone();
        move || {
            let ui = ui.unwrap();
            let name = ui.get_profile_name().to_string();
            let mut app = app.borrow_mut();
            let settings = app.settings;
            let status = if app.store.upsert(&name, settings) {
                model.set_vec(app.store.names());
                ui.set_profile_name("".into());
                fill(t("\"{}\" saved."), &[&name.trim()])
            } else {
                t("A profile needs a name.").to_string()
            };
            sync(&ui, &app, status);
        }
    });

    ui.on_pick_profile({
        let ui = ui.as_weak();
        let app = app.clone();
        move |index| {
            let ui = ui.unwrap();
            let mut app = app.borrow_mut();
            let Some(profile) = app.store.profiles.get(index as usize).cloned() else {
                return;
            };
            app.settings = profile.settings;
            ui.set_selected_preset(presets::index_of(&app.settings));
            app.push();
            let status = fill(t("\"{}\" loaded."), &[&profile.name]);
            sync(&ui, &app, status);
        }
    });

    ui.on_delete_profile({
        let ui = ui.as_weak();
        let app = app.clone();
        let model = profile_model.clone();
        move |index| {
            let ui = ui.unwrap();
            let mut app = app.borrow_mut();
            let name = app
                .store
                .profiles
                .get(index as usize)
                .map(|p| p.name.clone())
                .unwrap_or_default();
            if app.store.remove(index as usize) {
                model.set_vec(app.store.names());
                sync(&ui, &app, fill(t("\"{}\" deleted."), &[&name]));
            }
        }
    });

    ui.on_export_profiles({
        let ui = ui.as_weak();
        let app = app.clone();
        move || {
            let ui = ui.unwrap();
            let app = app.borrow();
            if app.store.profiles.is_empty() {
                sync(&ui, &app, t("There are no profiles to export.").into());
                return;
            }
            let Some(path) = rfd::FileDialog::new()
                .set_title(t("Export profiles"))
                .set_file_name("talkdedsec-visual-profiles.json")
                .add_filter("JSON", &["json"])
                .save_file()
            else {
                return;
            };
            let status = match app.store.export_to(&path) {
                Ok(1) => t("1 profile exported.").to_string(),
                Ok(count) => fill(t("{} profiles exported."), &[&count]),
                Err(e) => fill(t("Export failed: {}"), &[&e]),
            };
            sync(&ui, &app, status);
        }
    });

    ui.on_import_profiles({
        let ui = ui.as_weak();
        let app = app.clone();
        let model = profile_model.clone();
        move || {
            let ui = ui.unwrap();
            let Some(path) = rfd::FileDialog::new()
                .set_title(t("Import profiles"))
                .add_filter("JSON", &["json"])
                .pick_file()
            else {
                return;
            };
            let mut app = app.borrow_mut();
            let status = match app.store.import_from(&path) {
                Ok(count) => {
                    model.set_vec(app.store.names());
                    if count == 1 {
                        t("1 profile imported.").to_string()
                    } else {
                        fill(t("{} profiles imported."), &[&count])
                    }
                }
                Err(e) => fill(t("Import failed: {}"), &[&e]),
            };
            sync(&ui, &app, status);
        }
    });

    ui.on_open_settings({
        let ui = ui.as_weak();
        move || ui.unwrap().set_settings_open(true)
    });

    ui.on_close_settings({
        let ui = ui.as_weak();
        move || ui.unwrap().set_settings_open(false)
    });

    ui.on_set_autostart({
        let ui = ui.as_weak();
        let app = app.clone();
        move |on| {
            let ui = ui.unwrap();
            let applied = system::set_autostart(on);
            ui.set_autostart(if applied {
                on
            } else {
                system::autostart_enabled()
            });
            let app = app.borrow();
            let status = if applied {
                if on {
                    t("Will start with Windows.").to_string()
                } else {
                    t("Start with Windows is off.").to_string()
                }
            } else {
                t("Could not write to the registry.").to_string()
            };
            sync(&ui, &app, status);
        }
    });

    ui.on_set_tray_close({
        let ui = ui.as_weak();
        let app = app.clone();
        move |on| {
            let ui = ui.unwrap();
            let mut app = app.borrow_mut();
            app.tray_on_close = on;
            app.persist();
            ui.set_tray_on_close(on);
        }
    });

    ui.on_set_display({
        let ui = ui.as_weak();
        let app = app.clone();
        let model = display_model.clone();
        move |index, on| {
            let ui = ui.unwrap();
            let mut app = app.borrow_mut();
            let Some(engine) = app.engine.as_mut() else {
                return;
            };
            let index = index as usize;
            let accepted = engine.set_enabled(index, on);
            let label = engine
                .displays()
                .nth(index)
                .map(|(monitor, _)| display_label(index, monitor))
                .unwrap_or_default();
            model.set_vec(display_rows(app.engine.as_ref()));
            let status = if !accepted {
                t("At least one display has to stay selected.").to_string()
            } else if on {
                fill(t("{} is included."), &[&label])
            } else {
                fill(t("{} is left out."), &[&label])
            };
            app.persist();
            sync(&ui, &app, status);
        }
    });

    ui.on_open_source(|| open_url(REPO_URL));

    ui.on_move_window({
        let ui = ui.as_weak();
        move |dx, dy| {
            let ui = ui.unwrap();
            let window = ui.window();
            let scale = window.scale_factor();
            let pos = window.position();
            window.set_position(slint::PhysicalPosition::new(
                pos.x + (dx * scale).round() as i32,
                pos.y + (dy * scale).round() as i32,
            ));
        }
    });

    ui.on_resize_window({
        let ui = ui.as_weak();
        move |edge, dx, dy| {
            let ui = ui.unwrap();
            let window = ui.window();
            let scale = window.scale_factor();
            let position = window.position();
            let size = window.size();

            let min_w = (1160.0 * scale) as i32;
            let min_h = (680.0 * scale) as i32;
            let step_x = (dx * scale).round() as i32;
            let step_y = (dy * scale).round() as i32;

            let (mut x, mut y) = (position.x, position.y);
            let (mut w, mut h) = (size.width as i32, size.height as i32);

            if edge & 1 != 0 {
                let next = (w - step_x).max(min_w);
                x += w - next;
                w = next;
            }
            if edge & 2 != 0 {
                w = (w + step_x).max(min_w);
            }
            if edge & 4 != 0 {
                let next = (h - step_y).max(min_h);
                y += h - next;
                h = next;
            }
            if edge & 8 != 0 {
                h = (h + step_y).max(min_h);
            }

            window.set_position(slint::PhysicalPosition::new(x, y));
            window.set_size(slint::PhysicalSize::new(w as u32, h as u32));
        }
    });

    ui.on_minimize({
        let ui = ui.as_weak();
        move || ui.unwrap().window().set_minimized(true)
    });

    ui.on_quit({
        let ui = ui.as_weak();
        let app = app.clone();
        move || {
            let ui = ui.unwrap();
            if app.borrow().tray_on_close {
                let _ = ui.hide();
            } else {
                let _ = slint::quit_event_loop();
            }
        }
    });

    let mut hotkeys = system::Hotkeys::new();
    if let Some(manager) = hotkeys.as_mut() {
        let label = app.borrow().store.hotkey.clone();
        if !manager.bind(&label) {
            ui.set_hotkey_index(-1);
        }
    }
    let hotkeys = Rc::new(RefCell::new(hotkeys));

    ui.on_set_hotkey({
        let ui = ui.as_weak();
        let app = app.clone();
        let hotkeys = hotkeys.clone();
        move |index| {
            let ui = ui.unwrap();
            let Some((label, _)) = system::HOTKEYS.get(index as usize) else {
                return;
            };
            let mut hotkeys = hotkeys.borrow_mut();
            let bound = hotkeys.as_mut().is_some_and(|m| m.bind(label));
            let mut app = app.borrow_mut();
            let status = if bound {
                app.store.hotkey = (*label).to_string();
                app.store.save();
                ui.set_hotkey_index(index);
                fill(t("Shortcut set to {}."), &[label])
            } else {
                fill(t("{} is taken by another program."), &[label])
            };
            sync(&ui, &app, status);
        }
    });

    let tray_menu = Menu::new();
    let tray_show = MenuItem::new("", true, None);
    let tray_toggle = MenuItem::new("", true, None);
    let tray_quit = MenuItem::new("", true, None);
    label_tray(&tray_show, &tray_toggle, &tray_quit);
    let _ = tray_menu.append_items(&[
        &tray_show,
        &tray_toggle,
        &PredefinedMenuItem::separator(),
        &tray_quit,
    ]);

    let _tray = Icon::from_rgba(include_bytes!("../assets/tray.rgba").to_vec(), 32, 32)
        .ok()
        .and_then(|icon| {
            TrayIconBuilder::new()
                .with_menu(Box::new(tray_menu))
                .with_tooltip("Talkdedsec Visual")
                .with_icon(icon)
                .build()
                .ok()
        });

    ui.on_set_language({
        let ui = ui.as_weak();
        let app = app.clone();
        let tray = (tray_show.clone(), tray_toggle.clone(), tray_quit.clone());
        let displays = display_model.clone();
        move |code| {
            let ui = ui.unwrap();
            let Some(lang) = Lang::from_code(&code) else {
                return;
            };
            use_language(&ui, lang);
            presets::relabel(&preset_model);
            label_tray(&tray.0, &tray.1, &tray.2);
            let mut app = app.borrow_mut();
            displays.set_vec(display_rows(app.engine.as_ref()));
            app.store.language = Some(lang.code().to_string());
            app.store.save();
            sync(&ui, &app, t("Language changed.").into());
        }
    });

    let show_id = tray_show.id().clone();
    let toggle_id = tray_toggle.id().clone();
    let quit_id = tray_quit.id().clone();

    let pump = slint::Timer::default();
    pump.start(slint::TimerMode::Repeated, Duration::from_millis(120), {
        let ui = ui.as_weak();
        let app = app.clone();
        let hotkeys = hotkeys.clone();
        move || {
            let Some(ui) = ui.upgrade() else {
                return;
            };

            if summons.raised() {
                let _ = ui.show();
                ui.window().set_minimized(false);
            }

            let window = ui.window();
            let compact = (window.size().height as f32) / window.scale_factor() < COMPACT_BELOW;
            if ui.get_compact() != compact {
                ui.set_compact(compact);
            }

            let hotkey_id = hotkeys.borrow().as_ref().and_then(|m| m.id());
            while let Ok(event) = global_hotkey::GlobalHotKeyEvent::receiver().try_recv() {
                if event.state != global_hotkey::HotKeyState::Pressed {
                    continue;
                }
                if hotkey_id.is_some_and(|id| id == event.id) {
                    toggle_filter(&ui, &app);
                }
            }

            while let Ok(event) = MenuEvent::receiver().try_recv() {
                if event.id == show_id {
                    let _ = ui.show();
                    ui.window().set_minimized(false);
                } else if event.id == toggle_id {
                    toggle_filter(&ui, &app);
                } else if event.id == quit_id {
                    let _ = slint::quit_event_loop();
                }
            }
        }
    });

    let watch = slint::Timer::default();
    watch.start(slint::TimerMode::Repeated, Duration::from_secs(1), {
        let ui = ui.as_weak();
        let app = app.clone();
        let displays = display_model.clone();
        move || {
            let Some(ui) = ui.upgrade() else {
                return;
            };
            let mut app = app.borrow_mut();
            app.persist();
            let Some(engine) = app.engine.as_mut() else {
                return;
            };
            let report = engine.watch();
            if report.displays_changed {
                displays.set_vec(display_rows(app.engine.as_ref()));
            }
            if report != engine::Watch::default() {
                show_engine(&ui, &app);
            }
            let status = if report.gave_up > 0 {
                t("Another program keeps changing the colours — paused until you change a setting.")
            } else if report.restored > 0 {
                t("The display was reset — applied again.")
            } else if report.displays_changed {
                t("Display list updated.")
            } else {
                return;
            };
            ui.set_status_text(status.into());
        }
    });

    let hidden = std::env::args().any(|a| a == "--tray");
    if !hidden {
        ui.show()?;
    }
    slint::run_event_loop_until_quit()?;

    let mut app = app.borrow_mut();
    app.persist();
    // Dropping the engine puts every display it touched back the way it was.
    app.engine = None;
    Ok(())
}

fn toggle_filter(ui: &MainWindow, app: &Rc<RefCell<App>>) {
    let mut app = app.borrow_mut();
    app.auto_apply = !app.auto_apply;
    let status = if app.auto_apply {
        app.push()
    } else {
        if let Some(engine) = app.engine.as_mut() {
            engine.reset();
        }
        t("Waiting.").to_string()
    };
    app.persist();
    ui.set_auto_apply(app.auto_apply);
    sync(ui, &app, status);
}

#[cfg(test)]
mod tests {
    use super::*;
    use slint::Model;

    /// Builds the real window on a headless platform: proves the Turkish catalog is
    /// compiled in and that Slint, Rust and the preset list switch together.
    #[test]
    fn the_window_switches_language() {
        i_slint_backend_testing::init_no_event_loop();
        let ui = MainWindow::new().unwrap();

        use_language(&ui, Lang::En);
        let model = VecModel::from(presets::ui_models(&Scene::thumbnail(16, 9)));
        assert_eq!(model.row_data(0).unwrap().name, "Clear");

        use_language(&ui, Lang::Tr);
        presets::relabel(&model);
        assert_eq!(ui.get_language(), "tr");
        assert_eq!(ui.get_status_text(), "Hazır.");
        assert_eq!(ui.get_status_detail(), "Ekran dokunulmadı.");
        assert_eq!(t("Ready."), "Hazır.");
        assert_eq!(model.row_data(0).unwrap().name, "Berrak");
        assert_eq!(model.row_data(0).unwrap().hint, "biraz daha net");

        use_language(&ui, Lang::En);
        presets::relabel(&model);
        assert_eq!(ui.get_language(), "en");
        assert_eq!(ui.get_status_text(), "Ready.");
        assert_eq!(t("Ready."), "Ready.");
        assert_eq!(model.row_data(0).unwrap().name, "Clear");

        // The same window proves the sliders end exactly where the engine clamps.
        share_ranges(&ui);
        let r = ui.global::<Ranges>();
        let pairs = [
            (
                (r.get_brightness_min(), r.get_brightness_max()),
                Settings::BRIGHTNESS_RANGE,
            ),
            (
                (r.get_contrast_min(), r.get_contrast_max()),
                Settings::CONTRAST_RANGE,
            ),
            (
                (r.get_gamma_min(), r.get_gamma_max()),
                Settings::GAMMA_RANGE,
            ),
            (
                (r.get_temperature_min(), r.get_temperature_max()),
                Settings::TEMPERATURE_RANGE,
            ),
            (
                (r.get_night_vision_min(), r.get_night_vision_max()),
                Settings::NIGHT_VISION_RANGE,
            ),
        ];
        for (slider, engine) in pairs {
            assert_eq!(slider, engine);
        }
    }

    /// The y of the point at `step` (level `step * 4`) in a curve path.
    fn height_at(path: &str, step: usize) -> f32 {
        let point = path
            .split(['M', 'L'])
            .filter(|p| !p.is_empty())
            .nth(step)
            .unwrap();
        point.split_whitespace().nth(1).unwrap().parse().unwrap()
    }

    #[test]
    fn the_neutral_curve_is_the_diagonal() {
        let path = curve_path(&Settings::default(), 0);
        assert!(path.starts_with("M0 255.0 L4 251.0"), "{path}");
        assert!(path.ends_with("L255 0.0"), "{path}");
        assert_eq!(path.matches('L').count(), 64);
        for step in 0..=64 {
            let level = (step * 4).min(255) as f32;
            assert!((height_at(&path, step) - (255.0 - level)).abs() <= 0.1);
        }
    }

    #[test]
    fn a_warm_curve_puts_red_above_blue() {
        let warm = Settings {
            temperature: 0.8,
            ..Default::default()
        };
        // Smaller y is higher on screen.
        let red = height_at(&curve_path(&warm, 0), 32);
        let blue = height_at(&curve_path(&warm, 2), 32);
        assert!(red < blue, "red {red} blue {blue}");
    }
}
