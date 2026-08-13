<picture>
  <source media="(prefers-color-scheme: dark)" srcset="assets/banner-dark.png">
  <img src="assets/banner-light.png" width="100%" alt="Talkdedsec Visual — real-time screen colour engine for Windows. 5 controls, gamma ramp, 9.4 MB, no injection, no admin rights.">
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

A colour panel for your whole screen. Five sliders — brightness, contrast, gamma, temperature and
night vision — are compiled into a gamma ramp and written straight to the display adapter. Everything
the panel shows goes through it: games, video, the desktop, all of it.

It is not a game mod. Nothing is copied into a game folder, no process is opened, no library is
injected and no driver is installed. The ramp is the same knob a monitor calibration profile turns,
which is why it needs no administrator rights and costs exactly zero frame time — the correction
happens in the display pipeline, not in your game.

<br>

## The panel

<img src="assets/screenshot.png" width="100%" alt="The Talkdedsec Visual panel: preset rail, three control cards, live preview with a before/after split, and the profile rail">

Left rail holds the presets; the thumbnails are not screenshots but the same scene pushed through each
preset's actual curve, so what you see on the card is what the preset does. Centre column is the three
control cards. Under them sits a live preview with a draggable before/after split — you set the sliders
there and watch the result before it reaches the screen. **Hold to compare** drops the effect for as
long as the mouse is down.

Right rail saves profiles and prints the transfer curve: five points showing what each input level
becomes on the red, green and blue channels, updating as you drag.

<br>

## Controls

| Control | Range | Neutral | What it does |
|---|---|---|---|
| Brightness | −0.35 … +0.35 | 0.00 | Shifts the whole curve up or down |
| Contrast | 0.60 … 1.80 | 1.00 | Opens the gap between shadow and highlight, pivoting on mid grey |
| Gamma | 0.50 … 2.20 | 1.00 | Reweights the midtones while both ends stay pinned |
| Temperature | −1.00 … +1.00 | 0.00 | Warm or cool, by separating red from blue |
| Night vision | 0.00 … 1.00 | 0.00 | Lifts shadow detail out of the black without washing out highlights |

Every slider carries a tick at its neutral position, and every value box is editable — type `1.35`,
press <kbd>Enter</kbd>, done.

Saturation and hue are deliberately absent. A gamma ramp is one curve per channel and cannot mix
channels, so no honest implementation of them exists on this path. Shipping dead sliders would be
worse than leaving them out.

<br>

## How it works

Each of the 256 input levels is pushed through the five stages in a fixed order:

```
night vision → gamma → contrast → brightness → temperature
```

The result is a 256-entry table per channel, handed to `SetDeviceGammaRamp` on every attached display.
The maths lives in [`src/color.rs`](src/color.rs) and is held down by unit tests: the neutral setting
must reproduce the identity ramp exactly, every curve must stay monotonic, contrast must pivot on mid
grey, gamma must leave black and white untouched, and night vision must lift shadows at least ten
times more than highlights.

```bash
cargo test
```

### When Windows says no

Windows rejects gamma ramps that stray too far from linear unless `GdiIcmGammaRange` is widened, and
that registry switch needs administrator rights and a sign-out. Rather than fail, the engine walks the
setting down — 100%, 85%, 70% and so on — until the driver accepts one, then tells you in the status
bar exactly how much of your setting survived.

Gamma ramps also outlive the process that set them. The engine reads and stores the ramp of every
display at startup and puts it back on exit, and the restore also runs if the window is closed to the
tray or the effect is toggled off.

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

Delete it and the program starts fresh. Set `TALKDEDSEC_VISUAL_CONFIG` to a path of your own and it
becomes portable. Nothing else is written anywhere, and nothing is sent anywhere — the program opens
no sockets.

### Verify the download

SHA-256 for `talkdedsec-visual.exe`, release `v0.1.0`:

```text
63fc9bc06015f94bb1748d072bb51025788f7e080c70a3a8de1129169627bda3
```

```powershell
Get-FileHash .\talkdedsec-visual.exe -Algorithm SHA256
```

<br>

## Living in the tray

Closing the window sends it to the tray rather than quitting, so the effect stays on while you play.
The tray menu shows the window again, toggles the effect, or quits for real. If you would rather the
close button actually close, turn the option off in settings.

A global hotkey — <kbd>F6</kbd> through <kbd>F12</kbd>, <kbd>F9</kbd> by default — toggles the effect
without leaving the game. If another program already owns that key the panel says so instead of
failing silently.

Run at startup is a single registry value under `HKCU\...\CurrentVersion\Run`, added with `--tray` so
it comes up minimised; switching it off removes the value.

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
      "brightness": 0.04,
      "contrast": 1.05,
      "gamma": 1.35,
      "temperature": -0.1,
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

Rust 1.85 or newer is the only prerequisite. There is no C++ toolchain step, no Python, no
`node_modules`. The output is `target/release/talkdedsec-visual.exe` at roughly 9.4 MB.

| Path | What is in it |
|---|---|
| `src/color.rs` | The transfer curve and its tests |
| `src/engine.rs` | Gamma ramp I/O, the backoff ladder and restore-on-exit |
| `src/preview.rs` | The procedural preview scene |
| `src/presets.rs` | Built-in presets |
| `src/profiles.rs` | Profile store and JSON import/export |
| `src/system.rs` | Tray, global hotkey, run-at-startup |
| `ui/` | Slint interface: `main`, `widgets`, `icons`, `theme` |

The preview scene is generated, not photographed: sky gradient, treeline, terrain, a deliberately dark
pocket for night vision to work against, and a twelve-patch calibration strip. Nothing in this
repository is traced from anyone else's artwork.

<br>

## Known limits

- **Saturation and hue are not possible** on a gamma ramp. See above.
- **HDR displays** ignore gamma ramps on most drivers. Turn HDR off if nothing happens.
- **Exclusive fullscreen** hands the display pipeline to the game; some titles reset the ramp on entry.
  Borderless windowed is the reliable mode.
- **The ramp is global.** Every window on that display is affected, not just the game.
- **Windows clamps the range** by default, so extreme settings arrive softened. The status bar tells
  you when that happened.

<br>

## A note on games

This changes what the display does with the image, not what the game draws. That is a real
distinction, and it is why nothing here touches anti-cheat.

It is not, however, a promise. Some competitive titles disallow external visual settings that improve
visibility, and enforcement is their call rather than a technical question. Read the rules of whatever
you play and decide for yourself.

<br>

## Licence

[GPL-3.0-or-later](LICENSE) — © 2026 Talkdedsec

Take it, change it, ship it. Derivative work has to stay open too.
