# Manual Release Process

This document outlines the steps to manually create and publish a new release of Voquill.

## 1. Prepare the Release

Before building, ensure the version numbers are consistent across the project.

1. Update version in `src-tauri/Cargo.toml`
2. Update version in `src-tauri/tauri.conf.json`
3. Update version in `package.json`
4. Commit these changes:
   ```bash
   git add .
   git commit -m "Bump version to vX.Y.Z"
   ```

## 2. Build the Binaries

You will need to build the application on each target platform.

### Linux (Debian/Ubuntu/RPM/AppImage)
On any Linux machine with `podman` installed:
```bash
npm run release:linux
```

This builds inside an Ubuntu 22.04 container (`src-tauri/packaging/linux/Containerfile`).
Linux binaries link against the glibc of the machine that builds them, and the
AppImage also bundles that machine's libraries, so a build on a recent distro
(e.g. Fedora 44, glibc 2.43) will not start on older ones. The container pins the
floor at glibc 2.35 (Ubuntu 22.04 / Linux Mint 21 / Debian 12 and newer).
Bundles are written to `src-tauri/target/linux-release/bundle`, the only Linux
location `package-release.mjs` reads from. A host `npm run tauri:build` is for
local testing only and is never packaged for release.

The first run builds the image and compiles from scratch; later runs reuse the
`voquill-linux-release-cache` podman volume (Cargo registry, build target, npm cache).

### Windows (MSI/EXE)
On a Windows machine:
```bash
npm run tauri:build
```

### Package Release Artifacts

After building on each platform, run the packaging script to rename the build
outputs to the standard naming convention and generate checksums:

```bash
node scripts/package-release.mjs
```

Renamed artifacts and `.sha256` checksum files are written to `release-artifacts/`
in the project root. The script skips any stale artifacts whose embedded version
does not match the current version in `src-tauri/tauri.conf.json`.

Asset naming convention (automatic):
- `voquill-<version>-linux-x64.deb`
- `voquill-<version>-linux-x64.rpm`
- `voquill-<version>-linux-x64.AppImage`
- `voquill-<version>-windows-x64-setup.exe`
- `voquill-<version>-windows-x64.msi`

## 3. Create a GitHub Release

1. **Tag the commit**:
   ```bash
   git tag vX.Y.Z
   git push origin vX.Y.Z
   ```

2. **Draft the Release on GitHub**:
    - Go to the "Releases" section of the repository.
    - Click "Draft a new release".
    - Select the tag you just pushed.
    - Set the title (e.g., `Voquill vX.Y.Z`).
    - Describe the changes in the release notes.
    - Use the corresponding release notes file in `docs/release-notes/` (for example, `docs/release-notes/v1.3.1.md`).

### GitHub Asset Naming Convention

Release assets uploaded to GitHub must follow this naming scheme:

`voquill-<version>-<os>-<architecture>[ -setup ].<extension>`

Supported Voquill asset names:
- `voquill-<version>-linux-x64.AppImage`
- `voquill-<version>-linux-x64.deb`
- `voquill-<version>-linux-x64.rpm`
- `voquill-<version>-windows-x64-setup.exe`
- `voquill-<version>-windows-x64.msi`

Rules:
- Use lowercase `voquill`.
- Use SemVer for `<version>` (example: `1.2.6`).
- Use OS token values: `linux`, `windows`.
- Use architecture token value: `x64`.
- Use the optional `-setup` variant for the Windows installer executable.

Example for v1.2.6:
- `voquill-1.2.6-linux-x64.AppImage`
- `voquill-1.2.6-linux-x64.deb`
- `voquill-1.2.6-linux-x64.rpm`
- `voquill-1.2.6-windows-x64-setup.exe`
- `voquill-1.2.6-windows-x64.msi`

3. **Upload Assets from `release-artifacts/`**:
   Upload every file from the `release-artifacts/` directory (artifacts + `.sha256` files)
   to the GitHub release.
   The files are already named according to the convention above.

4. **Publish**:
   Review the release and click "Publish release".

## 4. Post-Release

Verify that the download links in the README point to the latest version and that the binaries work as expected on clean installations.
