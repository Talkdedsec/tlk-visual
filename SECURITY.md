# Security policy

## What this program touches

Worth stating plainly, because it is a short list:

- **Writes** the gamma ramp of every attached display, and restores the captured original on
  exit.
- **Writes** one file: `%APPDATA%\Talkdedsec\Visual\config.json`, or the path in
  `TALKDEDSEC_VISUAL_CONFIG`.
- **Writes** one registry value, `HKCU\Software\Microsoft\Windows\CurrentVersion\Run`, and
  only while "run at startup" is on.
- **Opens no sockets.** There is no update check, no telemetry and no network code of any
  kind in the binary.
- **Never requests elevation.** If something asks you for administrator rights in the name of
  this program, it is not this program.

## Verifying a download

Every release lists the SHA-256 of the executable, and the release workflow attaches a
`.sha256` file next to it.

```powershell
Get-FileHash .\talkdedsec-visual.exe -Algorithm SHA256
```

The binary is not code-signed, so SmartScreen warns on first run. Check the digest before
choosing **More info → Run anyway**. Builds are reproducible from source with
`cargo build --release`.

## Supported versions

The latest release is the supported one.

## Reporting a vulnerability

Report privately through
[GitHub Security Advisories](https://github.com/Talkdedsec1/talkdedsec-visual/security/advisories/new)
rather than a public issue. Include the version, your Windows build and the steps to
reproduce. Expect a first reply within a week.
