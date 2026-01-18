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
deno task build
```
Artifacts will be in `src-tauri/target/release/bundle/deb/`, `src-tauri/target/release/bundle/rpm/`, and `src-tauri/target/release/bundle/appimage/`.

### Windows (MSI/EXE)
On a Windows machine:
```bash
deno task build
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

3. **Upload Assets**:
   Manually upload the following files from your build outputs:
   - `voquill_X.Y.Z_amd64.deb`
   - `voquill_X.Y.Z_amd64.rpm`
   - `voquill_X.Y.Z_amd64.AppImage`
   - `Voquill_X.Y.Z_x64_en-US.msi`
   - `voquill.exe` (Standalone Windows binary)

4. **Publish**:
   Review the release and click "Publish release".

## 4. Post-Release

Verify that the download links in the README point to the latest version and that the binaries work as expected on clean installations.
