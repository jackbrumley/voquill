# Voquill - Rust/Tauri Implementation

Cross-platform push-to-talk dictation app using OpenAI Whisper, built with Rust and Tauri.

## Quick Start

### Prerequisites

#### Linux (Ubuntu/Debian/Pop!_OS)

Before building, install the required system dependencies:

```bash
sudo apt update
sudo apt install -y \
    libwebkit2gtk-4.1-dev \
    libgtk-3-dev \
    libayatana-appindicator3-dev \
    librsvg2-dev \
    build-essential \
    curl \
    wget \
    file \
    libssl-dev \
    libgtk-3-dev \
    libayatana-appindicator3-dev \
    librsvg2-dev \
    libasound2-dev \
    libxdo-dev
```

#### macOS

Install Xcode command line tools:
```bash
xcode-select --install
```

#### Windows

Install Microsoft C++ Build Tools or Visual Studio with C++ support.

### Install Rust

If you don't have Rust installed:
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env
```

### Install Tauri CLI

Install the Tauri CLI tool which is required for building and running Tauri applications:
```bash
cargo install tauri-cli
```

### Install Node.js

The frontend requires Node.js. Install it from [nodejs.org](https://nodejs.org/) or using a package manager:

```bash
# Ubuntu/Debian
sudo apt install nodejs npm

# macOS with Homebrew
brew install node

# Or use nvm (recommended)
curl -o- https://raw.githubusercontent.com/nvm-sh/nvm/v0.39.0/install.sh | bash
nvm install node
```

### Build and Run

1. **Clone the repository** (if not already done):
   ```bash
   git clone https://github.com/jackbrumley/voquill
   cd voquill/rust
   ```

2. **Install frontend dependencies**:
   ```bash
   cd ui
   npm install
   cd ..
   ```

3. **Build the application**:
   ```bash
   cargo build
   ```

4. **Run in development mode**:
   ```bash
   cargo tauri dev
   ```

5. **Build for production**:
   ```bash
   cargo tauri build
   ```

## Troubleshooting

### Linux Build Issues

If you encounter the error:
```
The system library `javascriptcoregtk-4.1` required by crate `javascriptcore-rs-sys` was not found.
```

This means you're missing the WebKit GTK development libraries. Install them with:
```bash
sudo apt install libwebkit2gtk-4.1-dev
```

### Common Issues

- **Missing pkg-config**: Install with `sudo apt install pkg-config` on Linux
- **Permission errors**: Make sure you have write permissions to the project directory
- **Node.js version**: Ensure you're using Node.js 16 or later

## Project Structure

- `src/` - Rust backend code
- `ui/` - React frontend
- `icons/` - Application icons
- `tauri.conf.json` - Tauri configuration

## Features

- Cross-platform desktop application
- Push-to-talk voice recording
- OpenAI Whisper integration for transcription
- System tray integration
- Global hotkey support
- Overlay window for status display

## Development

### Backend (Rust)
The Rust backend handles:
- Audio recording and processing
- Global hotkey management
- File system operations
- API communication

### Frontend (React)
The React frontend provides:
- User interface
- Settings management
- History display
- Status overlay

## License

MIT License - see LICENSE file for details.
