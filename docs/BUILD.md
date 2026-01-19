# Voquill Build Guide

Voquill uses Deno as a task runner to simplify the build process across different platforms.

## Quick Start

The main build command will check for dependencies, build the frontend, and then build the Tauri application.

```bash
# Standard release build
deno task build

# Development build (faster, includes debug info)
deno task build --dev
```

### When to use each command:
- **deno task build**: Use this for the final app you intend to share or use daily. It produces optimized, small executables.
- **deno task dev**: Use this for active development. It provides hot-reloading for both the frontend and backend.

## Requirements

- **Deno** (latest version)
- **Rust** with Cargo
- **Node.js** and **npm** (for frontend dependencies)

### Platform-Specific Requirements

#### Linux (Ubuntu/Debian)
The build script checks for these packages:
- `libpulse-dev`
- `libgtk-layer-shell-dev`
- `cmake`
- `pkg-config`
- `libclang-dev`
- `build-essential`

Additional Tauri requirements:
- `libwebkit2gtk-4.1-dev`
- `libgtk-3-dev`
- `libayatana-appindicator3-dev`
- `librsvg2-dev`

## Known Runtime Warnings

### libayatana-appindicator Deprecation (Linux)

When running Voquill on Linux, you may see a warning in the terminal:
`libayatana-appindicator-WARNING: libayatana-appindicator is deprecated. Please use libayatana-appindicator-glib in newly written code.`

**Status:** This is a cosmetic warning that affects all Tauri v2 applications using tray icons on Linux. It does not affect functionality.

**Cause:** Tauri's tray implementation currently depends on the older `libayatana-appindicator3` library. The upstream project has released a newer `-glib` variant, but the Rust ecosystem bindings haven't migrated yet. No action is required from users or developers.

## What the Build Script Does

1. **Checks Dependencies** - Verifies required Linux system libraries are present.
2. **Builds Frontend** - Runs `deno task build:ui` (which runs type checks and Vite build).
3. **Builds Application** - Runs `cargo tauri build` to create the final executable and installers.

## Output

After a successful build, you can find the artifacts in:
- **Linux**: `src-tauri/target/release/bundle/` (contains `.deb`, `.rpm`, `.AppImage`)
- **Windows**: `src-tauri/target/release/bundle/` (contains `.msi`, `.exe`)

## Troubleshooting

- **Missing dependencies on Linux**: If the build fails with missing library errors, follow the instructions provided by the script to install the necessary `apt` packages.
- **Frontend build issues**: If the UI fails to build, try clearing `node_modules` and running the build again.
- **Rust compilation errors**: Ensure your Rust toolchain is up to date with `rustup update`.
