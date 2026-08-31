## What this changes

<!-- What it does and why. If it fixes an issue, link it. -->

## Ran before opening

- [ ] `cargo fmt --all`
- [ ] `cargo clippy --all-targets -- -D warnings`
- [ ] `cargo test --all-targets`

## If it touches the ramp

- [ ] The curve change has a test that fails without it
- [ ] Every path that writes a ramp restores it, early exits included
