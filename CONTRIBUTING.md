# Contributing

Patches are welcome. The project is small on purpose, so the bar is less about process and
more about keeping it that way.

## Building

```bash
cargo build --release
cargo test
```

Rust 1.85 or newer, Windows. There is no C++ toolchain step, no Python and no
`node_modules`.

## Before opening a pull request

```bash
cargo fmt --all
cargo clippy --all-targets -- -D warnings
cargo test --all-targets
```

CI runs exactly these three on every push, so a green local run means a green pipeline.

## What the code expects of you

- **The colour maths is tested, so keep it tested.** Anything in `src/color.rs` that changes
  the curve needs a test that would fail without the change. The existing ones assert real
  properties — monotonicity, the neutral setting reproducing the identity ramp, contrast
  pivoting on mid grey — rather than golden values.
- **Never leave a ramp behind.** Gamma ramps survive the process that set them. Any new code
  path that writes one must also restore it, including on early exit.
- **No dead controls.** A gamma ramp is one curve per channel and cannot mix channels. If a
  feature needs channel mixing it does not belong on this path, and shipping a slider that
  silently does nothing is worse than not having it.
- **Comments explain constraints, not mechanics.** If the code says what it does, let it.

## Reporting a bug

Include your Windows build, whether HDR is on, the GPU, whether the game was borderless or
exclusive fullscreen, and what the status bar said. The status bar is the fastest way to
tell a rejected ramp apart from a ramp that applied and did nothing visible.

## Licence

Contributions are accepted under [GPL-3.0-or-later](LICENSE), the same terms as the project.
