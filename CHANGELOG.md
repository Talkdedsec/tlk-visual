# Changelog

All notable changes to this project are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the project uses
[semantic versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.0] — 2026-10-06

### Added

- English interface. The panel, tray menu, file dialogs and status messages are written in
  English and translated to Turkish; both languages are compiled into the executable.
- Language setting with English and Türkçe. A first run follows the Windows display language;
  an explicit choice switches the window immediately and is kept in `config.json`.
- Display selection in settings. Every connected display is listed by the name in its EDID,
  a laptop panel as *Built-in display*; one left out gets its original ramp back and is not
  touched again. The choice follows the panel, not the port number, and survives a reboot.
- Displays plugged in while the program runs are picked up within a second.
- The effect comes back on its own when something resets the ramp — a game entering exclusive
  fullscreen, a panel waking from sleep, a resolution change. If something keeps rewriting it,
  five times in twenty seconds, the engine steps aside on that display and says so; changing any
  setting takes it back.
- Only one copy runs. Starting the program again brings the open window forward.
- The transfer curve is drawn: one line while the channels agree, red, green and blue once
  temperature separates them, against the diagonal of an untouched display.
- Two live figures under the curve: the strength Windows let through, amber when it clamped or the
  engine stepped aside, and how many displays the effect is on.
- The mouse wheel steps a slider; a double-click puts it back on neutral.
- The global shortcut is shown next to the auto-apply switch it flips.
- The profile matching the sliders is highlighted.
- Screen reader and UI Automation names and roles for every control: sliders with value, range and
  stepping, switches with their state, icon buttons with what they do.
- <kbd>Esc</kbd> closes the settings.

### Changed

- Slider readouts use the decimal separator of the chosen language: `1.25` in English,
  `1,25` in Turkish. Typed values are accepted with either.
- Settings are written within a second of a change rather than only on exit, so shutting Windows
  down with the window in the tray keeps them.
- The engine writes only to the displays it found by name and restores only the ones it changed.
- Deleting a profile takes two clicks; the first turns the bin into *Delete?*.
- The settings dialog is as tall as what it holds instead of a fixed height.
- The engine card shows the curve and figures in place of the static explanation, which lives in
  the README and on the site.
- Below 760 pixels of height the live preview and the source-code card step aside.
- New screenshots.

### Fixed

- A ramp left on screen by a run that died before restoring it was taken for the original at the
  next start and restored forever after. It is now recognised and cleared.
- A `config.json` saved with a byte order mark — Notepad, PowerShell 5 — was read as empty, and the
  next save replaced every profile with defaults. The mark is now ignored, and a config that still
  cannot be read is copied to `config.json.unreadable` before anything is written over it.
- The brightness and contrast sliders ran past the range the engine accepts, so the knob snapped
  back from both ends. The engine's own limits now set every slider's range.
- With more than three or four profiles the left rail outgrew the window and pushed the status bar
  and the import and export buttons off screen. The profile list now scrolls.
- At the minimum window height the layout overflowed even with no profiles at all.

## [0.1.0] — 2026-08-13

First public release.

### Added

- Five display controls — brightness, contrast, gamma, temperature and night vision —
  compiled into a per-channel gamma ramp and written to every attached display.
- Live preview with a draggable before/after split, drawn from a procedural scene rather
  than a screenshot, plus a hold-to-compare button that drops the effect while held.
- Six built-in presets whose thumbnails are rendered through each preset's real curve.
- Named profiles with JSON import and export, stored in
  `%APPDATA%\Talkdedsec\Visual\config.json`, or anywhere `TALKDEDSEC_VISUAL_CONFIG` points.
- Configurable global hotkey (F6–F12) that toggles the effect from inside a game.
- System tray with show, toggle and quit; closing the window minimises there by default.
- Run at startup, written as a single `HKCU\...\CurrentVersion\Run` value with `--tray`.
- Transfer curve readout showing what each input level becomes per channel.
- Resizable frameless window with grips on every edge and corner.

### Engine notes

- Windows clamps gamma ramps that stray too far from linear. The engine walks the strength
  down — 100%, 85%, 70% and so on — until the driver accepts one, and reports the accepted
  fraction in the status bar instead of failing silently.
- Gamma ramps outlive the process that wrote them, so the original ramp of every display is
  captured at startup and restored on exit, on toggle-off and on close-to-tray.

### Known limits

- Saturation and hue cannot be expressed as a per-channel curve and are therefore absent.
- HDR displays ignore gamma ramps on most drivers.
- Exclusive fullscreen hands the display pipeline to the game; borderless windowed is the
  reliable mode.

[Unreleased]: https://github.com/Talkdedsec/tlk-visual/compare/v0.2.0...HEAD
[0.2.0]: https://github.com/Talkdedsec/tlk-visual/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/Talkdedsec/tlk-visual/releases/tag/v0.1.0
