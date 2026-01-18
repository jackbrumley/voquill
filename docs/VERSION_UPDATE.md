# Version Update Checklist

When releasing a new version of Voquill, the following files need to be updated with the new version number:

## Required Files to Update

### 1. `src-tauri/Cargo.toml`
```toml
[package]
name = "voquill"
version = "X.Y.Z"  # ← Update this line
```
- **Purpose**: Main Rust package version
- **Impact**: Used by Cargo for build metadata

### 2. `src-tauri/tauri.conf.json`
```json
{
  "version": "X.Y.Z",  // ← Update this line
```
- **Purpose**: Tauri application configuration
- **Impact**: Used for app bundles and installers

### 3. `package.json` (Root)
```json
{
  "version": "X.Y.Z",  // ← Update this line
```
- **Purpose**: Root project version
- **Impact**: Keeps project versioning consistent

## Version Update Process

1. **Decide on new version number** following [Semantic Versioning](https://semver.org/):
   - `MAJOR.MINOR.PATCH` (e.g., `1.0.0`, `1.1.0`, `1.0.1`)

2. **Update all three files** with the same version number

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

## Notes

- All versioned files should always have the same version number.
- `src-tauri/Cargo.lock` will be automatically updated when you run the build and should be committed.
- Consider updating a `CHANGELOG.md` or the release notes when bumping versions.
