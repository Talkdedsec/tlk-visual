# winget manifests

Kept here so the manifest that goes to `microsoft/winget-pkgs` is versioned next to the
release it describes. These files are not read by anything at build time.

Publishing a version:

1. Copy `<version>/` into a fork of `microsoft/winget-pkgs` under
   `manifests/t/Talkdedsec/Visual/<version>/`.
2. `winget validate --manifest <folder>` and, on a machine you can dirty,
   `winget install --manifest <folder>`.
3. Open the pull request. Their CI does the rest.

For a new release, copy the previous version folder, change `PackageVersion`, the
`InstallerUrl`, the `InstallerSha256` from the release's `.sha256` file, and
`ReleaseDate`.
