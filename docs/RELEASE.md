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
On a Linux machine:
```bash
npm run tauri:build
```
Artifacts will be in `src-tauri/target/release/bundle/deb/`, `src-tauri/target/release/bundle/rpm/`, and `src-tauri/target/release/bundle/appimage/`.

### Windows (MSI/EXE)
On a Windows machine:
```bash
npm run tauri:build
```
Artifacts will be in `src-tauri/target/release/bundle/msi/` and `src-tauri/target/release/`.

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
    - Use the corresponding release notes file in `docs/release-notes/` (for example, `docs/release-notes/v1.3.0.md`).

### GitHub Asset Naming Convention

Release assets uploaded to GitHub must follow this naming scheme:

`voquill-<version>-<os>-<architecture>[ -<variant> ].<extension>`

Supported Voquill asset names:
- `voquill-<version>-linux-x64`
- `voquill-<version>-linux-x64.AppImage`
- `voquill-<version>-linux-x64.deb`
- `voquill-<version>-linux-x64.rpm`
- `voquill-<version>-windows-x64-portable.exe`
- `voquill-<version>-windows-x64-setup.exe`
- `voquill-<version>-windows-x64.msi`

Rules:
- Use lowercase `voquill`.
- Use SemVer for `<version>` (example: `1.2.6`).
- Use OS token values: `linux`, `windows`.
- Use architecture token value: `x64`.
- Use the optional `<variant>` only when needed (example: `portable`, `setup`).

Example for v1.2.6:
- `voquill-1.2.6-linux-x64`
- `voquill-1.2.6-linux-x64.AppImage`
- `voquill-1.2.6-linux-x64.deb`
- `voquill-1.2.6-linux-x64.rpm`
- `voquill-1.2.6-windows-x64-portable.exe`
- `voquill-1.2.6-windows-x64-setup.exe`
- `voquill-1.2.6-windows-x64.msi`

3. **Upload Assets**:
   Manually upload assets using the naming convention above.

4. **Publish**:
   Review the release and click "Publish release".

## 4. Post-Release

Verify that the download links in the README point to the latest version and that the binaries work as expected on clean installations.
