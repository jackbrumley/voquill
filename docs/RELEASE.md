# Release Process

This document explains how to create releases for Voquill using the automated GitHub Actions workflow.

## Overview

The GitHub Actions workflow automatically builds Voquill for all supported platforms (Windows, macOS, and Linux) and creates a GitHub release with all the binaries.

## Supported Platforms

- **Windows x86_64**: `.exe` executable and `.msi` installer
- **macOS x86_64**: Binary executable and `.dmg` installer
- **Linux x86_64**: Binary executable, `.deb` package, and `.AppImage`

## How to Create a Release

### Method 1: Git Tag (Recommended)

1. **Ensure your code is ready for release**:
   ```bash
   # Make sure all changes are committed
   git add .
   git commit -m "Prepare for release v1.0.0"
   git push origin main
   ```

2. **Create and push a version tag**:
   ```bash
   # Create a tag (replace 1.0.0 with your version)
   git tag v1.0.0
   
   # Push the tag to GitHub
   git push origin v1.0.0
   ```

3. **Wait for the build to complete**:
   - Go to your GitHub repository
   - Click on "Actions" tab
   - Watch the "Release" workflow run
   - This typically takes 10-15 minutes

4. **Check the release**:
   - Go to the "Releases" section of your GitHub repository
   - Your new release should appear with all platform binaries attached

### Method 2: Manual Trigger

1. Go to your GitHub repository
2. Click on "Actions" tab
3. Select "Release" workflow
4. Click "Run workflow"
5. Choose the branch and click "Run workflow"

## Version Numbering

Use semantic versioning (semver) for your tags:
- `v1.0.0` - Major release
- `v1.0.1` - Patch release
- `v1.1.0` - Minor release
- `v2.0.0` - Major breaking changes

## What Gets Built

For each platform, the workflow creates:

### Windows
- `voquill.exe` - Standalone executable
- `voquill_1.0.0_x64_en-US.msi` - Windows installer

### macOS
- `voquill` - Standalone executable
- `Voquill_1.0.0_x64.dmg` - macOS disk image

### Linux
- `voquill` - Standalone executable
- `voquill_1.0.0_amd64.deb` - Debian/Ubuntu package
- `voquill_1.0.0_amd64.AppImage` - Universal Linux package

## Distribution

Users can download the appropriate file for their platform:

- **Windows users**: Download the `.msi` installer for easy installation, or the `.exe` for portable use
- **macOS users**: Download the `.dmg` file and drag to Applications folder
- **Linux users**: 
  - Debian/Ubuntu: Download and install the `.deb` package
  - Other distributions: Use the `.AppImage` file (no installation required)
  - Advanced users: Use the standalone binary

## Troubleshooting

### Build Fails
1. Check the Actions tab for error details
2. Common issues:
   - Syntax errors in code
   - Missing dependencies
   - Version conflicts

### Missing Binaries
- Some platforms might fail while others succeed
- Check individual job logs in the Actions tab
- The release will still be created with available binaries

### Permission Issues
- Ensure your GitHub repository has Actions enabled
- Check that the workflow has write permissions to create releases

## Local Testing

Before creating a release, test your build locally:

```bash
# Test the build process
node scripts/build.js

# Test in development mode
cd rust
cargo tauri dev
```

## Updating the Workflow

The workflow file is located at `.github/workflows/release.yml`. Modify it if you need to:
- Add new target platforms
- Change build configurations
- Update dependencies
- Modify artifact names

After updating the workflow, commit and push the changes before creating your next release.
