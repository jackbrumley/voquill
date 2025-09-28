# Version Update Checklist

When releasing a new version of Voquill, the following files need to be updated with the new version number:

## Required Files to Update

### 1. `rust/Cargo.toml`
```toml
[package]
name = "voquill"
version = "X.Y.Z"  # ← Update this line
```
- **Purpose**: Main Rust package version
- **Impact**: Used by Cargo for dependency management and build metadata

### 2. `rust/tauri.conf.json`
```json
{
  "$schema": "https://schema.tauri.app/config/2.0.0",
  "productName": "Voquill",
  "version": "X.Y.Z",  // ← Update this line
  "identifier": "com.voquill.voquill",
```
- **Purpose**: Tauri application configuration
- **Impact**: Used for app bundles, installers, and update checking

### 3. `rust/ui/package.json`
```json
{
  "name": "ui",
  "private": true,
  "version": "X.Y.Z",  // ← Update this line
  "type": "module",
```
- **Purpose**: Frontend UI package version
- **Impact**: Keeps frontend version consistent with main app

### 4. `flatpak/com.voquill.voquill.yml`
```yaml
sources:
  - type: file
    url: https://github.com/jackbrumley/voquill/releases/download/vX.Y.Z/voquill-linux-x86_64  # ← Update version in URL
    sha256: PLACEHOLDER_SHA256
```
- **Purpose**: Flatpak manifest for Linux packaging
- **Impact**: Used for immutable Linux distributions (Bazzite, Fedora Silverblue)

### 5. `flatpak/com.voquill.voquill.metainfo.xml`
```xml
<releases>
  <release version="X.Y.Z" date="YYYY-MM-DD">  <!-- ← Add new release entry -->
    <description>
      <p>Description of changes in this version.</p>
    </description>
  </release>
```
- **Purpose**: AppStream metadata for Flatpak
- **Impact**: Provides version history and release notes for software centers

## Version Update Process

1. **Decide on new version number** following [Semantic Versioning](https://semver.org/):
   - `MAJOR.MINOR.PATCH` (e.g., `1.0.0`, `1.1.0`, `1.0.1`)
   - **MAJOR**: Breaking changes
   - **MINOR**: New features (backward compatible)
   - **PATCH**: Bug fixes (backward compatible)

2. **Update all five files** with the same version number

3. **Test the build** - See [BUILD.md](BUILD.md) for detailed build instructions

4. **Commit the version changes**:
   ```bash
   git add .
   git commit -m "Bump version to X.Y.Z"
   ```

5. **Create a git tag**:
   ```bash
   git tag vX.Y.Z
   git push origin vX.Y.Z
   ```
   A **git tag** is like a bookmark that marks a specific point in your project's history. Think of it as a permanent label that says "this is version 1.0.0" or "this is the release version."


## Notes

- All five files should always have the same version number
- The `rust/Cargo.lock` file will be automatically updated when you run the build and **should be committed** (ensures everyone uses the same dependency versions)
- Consider updating the changelog/release notes when bumping versions
- For pre-release versions, use suffixes like `1.0.0-beta.1` or `1.0.0-rc.1`

## Current Version: 1.0.0

Last updated: January 2025
