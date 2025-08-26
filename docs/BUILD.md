# Voquill Build Script

Simple cross-platform build automation for Voquill using Node.js.

## Quick Start

```bash
# Standard release build - Creates optimized, small executable for distribution
node scripts/build.js

# Development build - Builds faster but creates larger file with debug info
node scripts/build.js --dev

# Clean build - Deletes all previous build files first, then builds fresh
node scripts/build.js --clean

# Clean development build - Fresh build with debug info (combines both above)
node scripts/build.js --dev --clean
```

### When to use each command:
- **Standard build**: Use this when you want the final app to share or use daily
- **Development build**: Use this when testing changes (builds much faster)
- **Clean build**: Use this when something seems broken or after major changes

## Requirements

- **Node.js** (any recent version)
- **Rust** with Cargo
- **Tauri CLI** (will be installed automatically if missing)
- **npm** (comes with Node.js)

### Platform-Specific Requirements

#### Linux (Ubuntu/Debian)
The script will check for and require these system packages:
- `libwebkit2gtk-4.1-dev`
- `libgtk-3-dev`
- `libayatana-appindicator3-dev`
- `librsvg2-dev`
- `build-essential`
- `curl`
- `wget`
- `file`
- `libssl-dev`
- `libasound2-dev`
- `libxdo-dev`

If any are missing, the script will show you the exact command to install them.

#### macOS
- Xcode command line tools: `xcode-select --install`

#### Windows
- Microsoft C++ Build Tools or Visual Studio with C++ support

The script will automatically check for these requirements and guide you if anything is missing.

## What It Does

1. **Checks Requirements** - Verifies Node.js and Rust are installed
2. **Cleans Build** - Removes previous frontend build artifacts
3. **Installs Dependencies** - Runs `npm install` in the UI directory
4. **Builds Frontend** - Runs `npm run build` to create the React build
5. **Builds Application** - Runs `cargo tauri build` to create the final executable
6. **Shows Results** - Displays the location and size of the built executable

## Options

- `--dev` or `-d` - Build in debug mode (faster compilation, larger binary)
- `--clean` or `-c` - Remove all build artifacts before building (including Rust target directory)
- `--help` or `-h` - Show usage information

## Output

The script will create:
- **Windows**: `rust/target/release/voquill.exe` (or `debug/voquill.exe` with `--dev`)
- **Linux/macOS**: `rust/target/release/voquill` (or `debug/voquill` with `--dev`)

## Troubleshooting

- **"This script must be run from the project root directory"**
  - Make sure you're running the script from the main project directory (where the `scripts/build.js` file is located)

- **"Node.js not found"**
  - Install Node.js from [nodejs.org](https://nodejs.org/)

- **"Rust not found"**
  - Install Rust from [rustup.rs](https://rustup.rs/)

- **Build fails during frontend step**
  - Try running `npm install` manually in the `rust/ui` directory first

- **Build fails during Tauri step**
  - Make sure you have all the system dependencies installed as described in `rust/README.md`

## Manual Build Process

If you prefer to run the steps manually:

```bash
# 1. Install frontend dependencies
cd rust/ui
npm install

# 2. Build frontend
npm run build

# 3. Build application
cd ..
cargo tauri build
```

## Development Workflow

For development, you might prefer:

```bash
# Run in development mode (no build needed)
cd rust
cargo tauri dev
```

This starts the app in development mode with hot reloading for both frontend and backend changes.
