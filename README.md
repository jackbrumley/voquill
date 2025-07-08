# Voquill - Voice-to-Text Dictation App

A cross-platform voice-to-text application with GUI and global hotkey support, powered by OpenAI's Whisper API.

## Quick Start Guide (Developer)

### Prerequisites
- Go 1.19 or later
- PortAudio development libraries
- OpenAI API key

### Install Dependencies

**Arch Linux (Manjaro):**
```bash
sudo pacman -S go portaudio
```

**Debian/Ubuntu:**
```bash
sudo apt update
sudo apt install golang-go libportaudio2 libportaudio-dev
```

**Windows:**
- Install Go from https://golang.org/download/
- Install TDM-GCC or MinGW-w64
- Download PortAudio and place in your PATH

### Run the Application

1. Clone and navigate to the project:
```bash
git clone <repository-url>
cd voquill
```

2. Install Go dependencies:
```bash
cd src
go mod tidy
cd ..
```

3. Build and run:
```bash
./build.sh
./bin/voquill
```

4. Configure your OpenAI API key in the Settings tab or edit the config file directly.

## Building for Different Operating Systems

### Easy Build (Recommended)
Use the provided build script:
```bash
./build.sh
```

This will create binaries in the `bin/` directory for your current platform and cross-compile for other platforms if the required tools are available.

### Manual Build Commands

**Build for Current Platform:**
```bash
cd src
go build -o ../bin/voquill .
cd ..
```

**Cross-Platform Builds:**

**For Windows (from Linux/macOS):**
```bash
cd src
CGO_ENABLED=1 GOOS=windows GOARCH=amd64 CC=x86_64-w64-mingw32-gcc go build -o ../bin/voquill.exe .
cd ..
```

**For Linux (from other platforms):**
```bash
cd src
CGO_ENABLED=1 GOOS=linux GOARCH=amd64 go build -o ../bin/voquill-linux .
cd ..
```

**For macOS (from other platforms):**
```bash
cd src
CGO_ENABLED=1 GOOS=darwin GOARCH=amd64 go build -o ../bin/voquill-macos .
cd ..
```

### Platform-Specific Build Instructions

#### Arch Linux (Manjaro)
```bash
# Install dependencies
sudo pacman -S go portaudio gcc

# For cross-compilation to Windows (optional)
sudo pacman -S mingw-w64-gcc

# Build
./build.sh

# Install desktop file (optional)
cp packaging/linux/voquill.desktop ~/.local/share/applications/
update-desktop-database ~/.local/share/applications/
```

#### Debian/Ubuntu
```bash
# Install dependencies
sudo apt update
sudo apt install golang-go libportaudio2 libportaudio-dev build-essential

# For cross-compilation to Windows (optional)
sudo apt install gcc-mingw-w64

# Build
./build.sh

# Install desktop file (optional)
cp packaging/linux/voquill.desktop ~/.local/share/applications/
update-desktop-database ~/.local/share/applications/
```

#### Windows
```bash
# Prerequisites: Install Go, TDM-GCC/MinGW-w64, and PortAudio
# Then build:
cd src
go build -o ../bin/voquill.exe .
cd ..
```

## Project Structure

```
voquill/
├── src/                    # Source code directory
│   ├── main.go            # Application entry point
│   ├── types.go           # Type definitions and constants
│   ├── config.go          # Configuration management
│   ├── history.go         # Transcription history
│   ├── audio.go           # Audio recording functionality
│   ├── transcription.go   # OpenAI Whisper integration
│   ├── keyboard.go        # Keyboard simulation
│   ├── gui.go             # GUI components
│   ├── core.go            # Core application logic
│   ├── icon.go            # Embedded icon resource
│   ├── go.mod             # Go module definition
│   └── go.sum             # Go module checksums
├── bin/                   # Compiled binaries
│   ├── voquill           # Linux/macOS binary
│   ├── voquill.exe       # Windows binary
│   └── voquill-*         # Platform-specific binaries
├── packaging/            # Packaging and deployment files
│   └── linux/           # Linux-specific packaging
│       └── voquill.desktop  # Desktop file for Linux
├── assets/               # Application assets
├── build.sh              # Build script for all platforms
└── README.md             # This file
```

## Features

- **Voice Recording**: Click-to-record with visual feedback
- **Real-time Transcription**: Powered by OpenAI Whisper API
- **Text Simulation**: Automatically types transcribed text
- **History Management**: View and copy previous transcriptions with text wrapping
- **Cross-platform**: Works on Windows, Linux, and macOS
- **Desktop Integration**: Proper icon display in taskbar/dock

## Configuration

The application stores configuration in:
- **Linux**: `~/.config/voquill/config.ini`
- **Windows**: `%LOCALAPPDATA%\voquill\config.ini`

Required settings:
- `WHISPER_API_KEY`: Your OpenAI API key
- `TYPING_SPEED_INTERVAL`: Delay between keystrokes (default: 0.01s)
- `HOTKEY`: Global hotkey combination (currently basic support)

## Usage

1. **Setup**: Enter your OpenAI API key in the Settings tab
2. **Record**: Click the microphone button to start/stop recording
3. **Transcribe**: Audio is automatically sent to OpenAI for transcription
4. **Type**: Transcribed text is automatically typed at cursor position
5. **History**: View previous transcriptions in the History tab with copy buttons

## Development Notes

- The application uses CGO for audio recording (PortAudio) and keyboard simulation
- Global hotkey detection is currently basic and may need platform-specific improvements
- The GUI is built with Fyne for cross-platform compatibility
- Icon is embedded as a Go resource for proper desktop integration
- Build artifacts are organized in the `bin/` directory
- Source code is organized in the `src/` directory with modular files
- Packaging files are organized in the `packaging/` directory by platform
