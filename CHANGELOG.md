# Changelog

All notable changes to this project are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the project uses
[semantic versioning](https://semver.org/spec/v2.0.0.html).

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

[0.1.0]: https://github.com/Talkdedsec1/tlk-visual/releases/tag/v0.1.0
