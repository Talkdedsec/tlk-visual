// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Talkdedsec

#![windows_subsystem = "windows"]

mod color;
mod engine;
mod presets;
mod preview;
mod profiles;
mod system;

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use color::Settings;
use engine::{Applied, Engine};
use preview::Scene;
use profiles::Store;
use slint::{ModelRc, VecModel};
use tray_icon::menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use tray_icon::{Icon, TrayIconBuilder};

slint::include_modules!();

const REPO_URL: &str = "https://github.com/Talkdedsec/tlk-visual";
const VERSION: &str = env!("CARGO_PKG_VERSION");

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

    fn remember(&mut self) {
        self.store.last = self.settings;
        self.store.auto_apply = self.auto_apply;
        self.store.tray_on_close = self.tray_on_close;
        self.store.save();
    }

    fn push(&mut self) -> String {
        if !self.auto_apply {
            return "Beklemede.".into();
        }
        self.force_apply()
    }

    fn force_apply(&mut self) -> String {
        let neutral = self.settings.is_neutral();
        let Some(engine) = self.engine.as_mut() else {
            return "Ekran sürücüsü gama tablosunu kabul etmiyor.".into();
        };
        match engine.apply(&self.settings) {
            Applied::Full if neutral => "Hazır.".into(),
            Applied::Full => "Uygulandı.".into(),
            Applied::Limited(f) => format!(
                "Windows tam gücü reddetti — %{} uygulandı.",
                (f * 100.0).round() as i32
            ),
            Applied::Rejected => "Windows bu ayarı reddetti.".into(),
        }
    }

    fn detail(&self) -> String {
        if !self.auto_apply {
            return "Otomatik uygulama kapalı — ekran dokunulmadı.".into();
        }
        let s = self.settings;
        let d = Settings::default();
        let mut parts = Vec::new();
        if (s.brightness - d.brightness).abs() > 1e-4 {
            parts.push(format!("Parlaklık {}", tr(s.brightness, 2, true)));
        }
        if (s.contrast - d.contrast).abs() > 1e-4 {
            parts.push(format!("Kontrast {}", tr(s.contrast, 2, false)));
        }
        if (s.gamma - d.gamma).abs() > 1e-4 {
            parts.push(format!("Gama {}", tr(s.gamma, 2, false)));
        }
        if s.temperature.abs() > 1e-4 {
            parts.push(format!("Sıcaklık {}", tr(s.temperature, 2, true)));
        }
        if s.night_vision.abs() > 1e-4 {
            parts.push(format!("Gece görüşü {}", tr(s.night_vision, 2, false)));
        }
        if parts.is_empty() {
            "Ekran dokunulmadı.".into()
        } else {
            parts.join(" · ")
        }
    }
}

fn tr(value: f32, decimals: usize, signed: bool) -> String {
    let text = if signed {
        format!("{value:+.decimals$}")
    } else {
        format!("{value:.decimals$}")
    };
    text.replace('.', ",")
}

/// Five points along the transfer curve, which is what a gamma ramp really is.
fn curve_readout(settings: &Settings) -> String {
    let stops = [0.0f32, 0.25, 0.5, 0.75, 1.0];
    let mut lines = vec!["in     R    G    B".to_string()];
    for stop in stops {
        lines.push(format!(
            "{:>3}  {:>3}  {:>3}  {:>3}",
            (stop * 255.0) as i32,
            (settings.channel(stop, 0) * 255.0).round() as i32,
            (settings.channel(stop, 1) * 255.0).round() as i32,
            (settings.channel(stop, 2) * 255.0).round() as i32,
        ));
    }
    lines.join("\n")
}

fn sync(ui: &MainWindow, app: &App, status: String) {
    let s = app.settings;
    ui.set_brightness(s.brightness);
    ui.set_contrast(s.contrast);
    ui.set_gamma(s.gamma);
    ui.set_temperature(s.temperature);
    ui.set_night_vision(s.night_vision);

    ui.set_brightness_text(tr(s.brightness, 2, true).into());
    ui.set_contrast_text(tr(s.contrast, 2, false).into());
    ui.set_gamma_text(tr(s.gamma, 2, false).into());
    ui.set_temperature_text(tr(s.temperature, 2, true).into());
    ui.set_night_vision_text(tr(s.night_vision, 2, false).into());

    ui.set_preview_after(app.scene.render(&s));
    ui.set_matrix_text(curve_readout(&s).into());
    ui.set_engine_active(!s.is_neutral() && app.auto_apply && app.engine.is_some());
    ui.set_status_text(status.into());
    ui.set_status_detail(app.detail().into());
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
    let ui = MainWindow::new()?;
    let app = Rc::new(RefCell::new(App::new()));

    let startup = match Engine::new() {
        Ok(engine) => {
            app.borrow_mut().engine = Some(engine);
            "Hazır.".to_string()
        }
        Err(e) => format!("Hata: {e}"),
    };

    let thumbs = Scene::thumbnail(160, 92);
    ui.set_presets(ModelRc::from(Rc::new(VecModel::from(presets::ui_models(
        &thumbs,
    )))));

    let profile_model = Rc::new(VecModel::<slint::SharedString>::default());
    profile_model.set_vec(app.borrow().store.names());
    ui.set_profiles(ModelRc::from(profile_model.clone()));

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
                sync(&ui, &app, "Sayı okunamadı.".into());
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
                "Orijinal görüntü.".to_string()
            } else {
                app.push()
            };
            ui.set_status_text(status.into());
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
            sync(&ui, &app, "Sıfırlandı.".into());
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
                "Beklemede.".into()
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
                format!("\"{}\" kaydedildi.", name.trim())
            } else {
                "Profil adı boş olamaz.".to_string()
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
            let status = format!("\"{}\" yüklendi.", profile.name);
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
                sync(&ui, &app, format!("\"{name}\" silindi."));
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
                sync(&ui, &app, "Dışa aktarılacak profil yok.".into());
                return;
            }
            let Some(path) = rfd::FileDialog::new()
                .set_title("Profilleri dışa aktar")
                .set_file_name("talkdedsec-visual-profiles.json")
                .add_filter("JSON", &["json"])
                .save_file()
            else {
                return;
            };
            let status = match app.store.export_to(&path) {
                Ok(count) => format!("{count} profil dışa aktarıldı."),
                Err(e) => format!("Dışa aktarma hatası: {e}"),
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
                .set_title("Profilleri içe aktar")
                .add_filter("JSON", &["json"])
                .pick_file()
            else {
                return;
            };
            let mut app = app.borrow_mut();
            let status = match app.store.import_from(&path) {
                Ok(count) => {
                    model.set_vec(app.store.names());
                    format!("{count} profil içe aktarıldı.")
                }
                Err(e) => format!("İçe aktarma hatası: {e}"),
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
                    "Windows açılışında başlayacak.".to_string()
                } else {
                    "Otomatik başlatma kapatıldı.".to_string()
                }
            } else {
                "Kayıt defterine yazılamadı.".to_string()
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
            app.remember();
            ui.set_tray_on_close(on);
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
                format!("Kısayol {label} olarak ayarlandı.")
            } else {
                format!("{label} başka bir program tarafından kullanılıyor.")
            };
            sync(&ui, &app, status);
        }
    });

    let tray_menu = Menu::new();
    let tray_show = MenuItem::new("Pencereyi göster", true, None);
    let tray_toggle = MenuItem::new("Filtreyi aç / kapa", true, None);
    let tray_quit = MenuItem::new("Çıkış", true, None);
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

    let hidden = std::env::args().any(|a| a == "--tray");
    if !hidden {
        ui.show()?;
    }
    slint::run_event_loop_until_quit()?;

    app.borrow_mut().remember();
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
        "Beklemede.".to_string()
    };
    app.remember();
    ui.set_auto_apply(app.auto_apply);
    sync(ui, &app, status);
}
