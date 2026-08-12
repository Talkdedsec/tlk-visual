<picture>
  <source media="(prefers-color-scheme: dark)" srcset="assets/banner-dark.png">
  <img src="assets/banner-light.png" width="100%" alt="Talkdedsec Visual — real-time screen colour engine for Windows. 5 controls, one 5×5 matrix, 9.4 MB, no injection, no admin rights.">
</picture>

<p align="center">
  <a href="https://github.com/Talkdedsec1/talkdedsec-visual/releases/latest"><b>download</b></a>
  &nbsp;·&nbsp;
  <a href="#build-from-source"><b>build</b></a>
  &nbsp;·&nbsp;
  <a href="#how-it-works"><b>how it works</b></a>
  &nbsp;·&nbsp;
  <a href="README.tr.md"><b>Türkçe</b></a>
</p>

<br>

## What this is

A colour panel for your whole screen. Five sliders — brightness, contrast, saturation, hue and night
vision — feed one 5×5 colour matrix, and that matrix is handed to Windows itself. Every pixel the
desktop composites goes through it: games, video, the browser, everything.

It is not a game mod. Nothing is copied into a game folder, no process is opened, no library is
injected and no driver is installed. The program asks Windows to tint the display and Windows does it,
the same way the built-in colour filters in Settings work. That is also why it runs without
administrator rights and costs nothing measurable in frame time.

<br>

## The panel

<img src="assets/screenshot.png" width="100%" alt="The Talkdedsec Visual panel: preset rail, three control cards, live preview with a before/after split, and the profile rail">

Left rail holds the presets; the thumbnails are not screenshots but the same scene rendered through
each preset's actual matrix, so what you see on the card is what the preset does. Centre column is the
three control cards. Under them sits a live preview with a draggable before/after split — you set the
sliders there and watch the result before it ever touches the screen. **Hold to compare** drops the
filter for as long as the mouse is down.

Right rail saves profiles and shows the live matrix: the twelve coefficients the engine is applying at
that moment, updating as you drag.

<br>

## Controls

| Control | Range | Neutral | What it does |
|---|---|---|---|
| Brightness | −0.50 … +0.50 | 0.00 | Lifts or drops the black level |
| Contrast | 0.50 … 2.00 | 1.00 | Opens the gap between shadow and highlight, pivoting on mid grey |
| Saturation | 0.00 … 3.00 | 1.00 | Colour intensity, from grey to oversaturated |
| Hue | −180° … +180° | 0° | Rotates the entire palette |
| Night vision | 0.00 … 1.00 | 0.00 | Lifts shadow detail and biases it green instead of clipping to black |

Every slider carries a tick at its neutral position, and every value box is editable — type `1.52`,
press <kbd>Enter</kbd>, done.

<br>

## How it works

The five controls compose into a single affine matrix and are applied in one pass:

```
contrast → saturation → hue → night vision → brightness
```

Composition is plain matrix multiplication, so the order is fixed and the result is one
`MagSetFullscreenColorEffect` call per change. The matrix maths lives in
[`src/color.rs`](src/color.rs) and is covered by unit tests — zero saturation must equal Rec.709
luminance, contrast must pivot exactly on mid grey, a 360° hue rotation must return the identity, and
composing the five must equal applying them one at a time.

```bash
cargo test
```

<br>

## Install

Download `talkdedsec-visual.exe` from
[Releases](https://github.com/Talkdedsec1/talkdedsec-visual/releases/latest) and run it. One file, no
installer, no .NET, no WebView2, no runtime of any kind. Windows SmartScreen will warn you the first
time because the binary is not code-signed yet; verify the checksum below before choosing
**More info → Run anyway**.

Settings, profiles and the last slider positions live in one file:

```
%APPDATA%\Talkdedsec\Visual\config.json
```

Delete it and the program starts fresh. Nothing else is written anywhere, and nothing is sent
anywhere — the program opens no sockets.

### Verify the download

SHA-256 for `talkdedsec-visual.exe`, release `v0.1.0`:

```text
a0b2fa66984b46d5a7f3003a8d2e10d38f3cfe6d8881e57b5cca40f2a89bc925
```

```powershell
Get-FileHash .\talkdedsec-visual.exe -Algorithm SHA256
```

<br>

## Living in the tray

Closing the window sends it to the tray rather than quitting, so the filter stays on while you play.
The tray menu shows the window again, toggles the filter, or quits for real. If you would rather the
close button actually close, turn the option off in settings.

A global hotkey — <kbd>F6</kbd> through <kbd>F12</kbd>, <kbd>F9</kbd> by default — toggles the filter
without leaving the game. If another program already owns the key the panel tells you instead of
silently failing.

Run at startup is a single registry value under `HKCU\...\CurrentVersion\Run`; switching it off
removes the value.

<br>

## Profiles

Name the current slider positions and they are saved. Saving under a name that already exists
overwrites it, so repeated saves do not pile up duplicates. Profiles export to plain JSON and import
back, which is also how you move them between machines:

```json
[
  {
    "name": "night",
    "settings": {
      "brightness": 0.08,
      "contrast": 1.1,
      "saturation": 0.6,
      "hue": 0.0,
      "night_vision": 0.85
    }
  }
]
```

<br>

## Build from source

```bash
git clone https://github.com/Talkdedsec1/talkdedsec-visual
cd talkdedsec-visual
cargo build --release
```

Rust 1.85 or newer is the only prerequisite. There is no C++ toolchain step, no Python, no `node_modules`.
The output is `target/release/talkdedsec-visual.exe` at roughly 9.4 MB.

| Path | What is in it |
|---|---|
| `src/color.rs` | The 5×5 matrix maths and its tests |
| `src/engine.rs` | The Magnification API binding |
| `src/preview.rs` | The procedural preview scene |
| `src/presets.rs` | Built-in presets |
| `src/profiles.rs` | Profile store and JSON import/export |
| `src/system.rs` | Tray, global hotkey, run-at-startup |
| `ui/` | Slint interface: `main`, `widgets`, `icons`, `theme` |

The preview scene in `preview.rs` is generated, not photographed: sky gradient, treeline, terrain,
a deliberately dark pocket for night vision to work against, and a twelve-patch calibration strip.
Nothing in this repository is traced from anyone else's artwork.

<br>

## A note on games

This changes what the display sends to your eyes, not what the game sends to the display. That is a
real distinction, and it is why nothing here trips anti-cheat.

It is not, however, a promise. Some competitive titles disallow external visual settings that improve
visibility, and enforcement is their call rather than a technical question. Read the rules of whatever
you play and decide for yourself.

<br>

## Licence

[GPL-3.0-or-later](LICENSE) — © 2026 Talkdedsec

Take it, change it, ship it. Derivative work has to stay open too.
