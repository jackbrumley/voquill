# Voquill - Voice-to-Text Dictation App

A cross-platform voice-to-text application with GUI and global hotkey support, powered by OpenAI's Whisper API.

![Voice-to-Text Dictation](assets/icon256x256.png)

## What is Voquill?

Voquill is a **system-wide voice dictation tool** that converts your speech to text and automatically types it wherever your cursor is positioned. Unlike other dictation apps that only work in specific applications, Voquill uses **keyboard simulation** to work in **any text input field** - from email clients and word processors to web browsers, code editors, and chat applications.

**Key Advantage:** Because Voquill simulates actual keystrokes, it integrates seamlessly with any application that accepts text input, giving you true system-wide voice dictation capabilities.

Perfect for:
- Writing emails, documents, and messages in any application
- Taking notes during meetings in your preferred text editor
- Coding with voice input in IDEs and code editors
- Accessibility and hands-free computing across all programs
- Quick voice-to-text conversion anywhere on your system

## Features

- **Voice Recording**: Click-to-record with visual feedback
- **Real-time Transcription**: Powered by OpenAI Whisper API
- **Text Simulation**: Automatically types transcribed text at cursor position
- **History Management**: View and copy previous transcriptions
- **Cross-platform**: Works on Windows, Linux, and macOS
- **Desktop Integration**: Proper icon display in taskbar/dock

## How to Use

1. **Download**: Get the appropriate binary for your platform from releases
2. **Setup**: Run the application and enter your OpenAI API key in the Settings tab
3. **Record**: Click the microphone button to start/stop recording
4. **Transcribe**: Audio is automatically sent to OpenAI for transcription
5. **Type**: Transcribed text appears wherever your cursor was positioned
6. **History**: View previous transcriptions in the History tab with copy buttons

## System Requirements

- **Windows**: Windows 10/11
- **Linux**: Any modern distribution with audio support
- **macOS**: macOS 10.14 or later
- **Internet**: Required for OpenAI Whisper API
- **Microphone**: For voice recording
- **OpenAI API Key**: Required for transcription service

## Configuration

The application stores settings in:
- **Linux**: `~/.config/voquill/config.ini`
- **Windows**: `%LOCALAPPDATA%\voquill\config.ini`
- **macOS**: `~/Library/Application Support/voquill/config.ini`

Required settings:
- `WHISPER_API_KEY`: Your OpenAI API key
- `TYPING_SPEED_INTERVAL`: Delay between keystrokes (default: 0.01s)
- `HOTKEY`: Global hotkey combination (currently basic support)

---

## 🛠️ For Developers

### Development Environment

**Prerequisites:** These instructions assume you're developing on a Debian-based (Ubuntu) or Arch-based (Manjaro) Linux system.

### Development Environment Setup

**Important:** Due to complex CGO dependencies (PortAudio, OpenGL), cross-compilation between platforms is not reliable. Build on the target platform for best results.

#### Linux Development (Arch/Manjaro)
```bash
# Install dependencies
sudo pacman -S go portaudio gcc

# Clone and setup
git clone <repository-url>
cd voquill
cd src
go mod tidy

# Build Linux binary
go build -o ../bin/voquill-linux-amd64 .

# Run
../bin/voquill-linux-amd64
```

#### Linux Development (Debian/Ubuntu)
```bash
# Install dependencies
sudo apt update
sudo apt install golang-go libportaudio2 libportaudio-dev build-essential

# Clone and setup
git clone <repository-url>
cd voquill
cd src
go mod tidy

# Build Linux binary
go build -o ../bin/voquill-linux-amd64 .

# Run
../bin/voquill-linux-amd64
```

#### Windows Development
```cmd
# Prerequisites:
# 1. Install Go from https://golang.org/download/
# 2. Install TDM-GCC from https://jmeubank.github.io/tdm-gcc/
# 3. Install PortAudio:
#    - Download from http://www.portaudio.com/download.html
#    - Or use vcpkg: vcpkg install portaudio

# Clone and setup
git clone <repository-url>
cd voquill
cd src
go mod tidy

# Build Windows binary
go build -o ../bin/voquill-windows-amd64.exe .

# Run
../bin/voquill-windows-amd64.exe
```

### Build Commands Summary

**Linux (on Linux):**
```bash
cd src && go build -o ../bin/voquill-linux-amd64 .
```

**Windows (on Windows):**
```cmd
cd src && go build -o ../bin/voquill-windows-amd64.exe .
```

### Project Structure

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
│   ├── go.sum             # Go module checksums
│   └── version.txt        # Version information
├── bin/                   # Compiled binaries
│   ├── voquill-linux-amd64      # Linux binary
│   └── voquill-windows-amd64.exe # Windows binary
├── packaging/            # Packaging and deployment files
│   └── linux/           # Linux-specific packaging
│       └── voquill.desktop  # Desktop file for Linux
├── assets/               # Application assets
├── DESIGN.md             # Technical documentation
└── README.md             # This file
```

### Linux Desktop Integration

Install the desktop file for system integration:

**Arch Linux (Manjaro):**
```bash
cp packaging/linux/voquill.desktop ~/.local/share/applications/
update-desktop-database ~/.local/share/applications/
```

**Debian/Ubuntu:**
```bash
cp packaging/linux/voquill.desktop ~/.local/share/applications/
update-desktop-database ~/.local/share/applications/
```

### Development Notes

- **Platform-Specific Builds**: Build on the target platform for reliable results
- **CGO Required**: The application uses CGO for audio recording (PortAudio) and keyboard simulation
- **Cross-Platform Libraries**: Fyne for GUI, PortAudio for audio, keybd_event for keyboard simulation
- **Global Hotkeys**: Currently basic implementation, may need platform-specific improvements
- **Icon Embedding**: Icon is embedded as a Go resource for proper desktop integration
- **Modular Design**: Source code is organized into logical modules for maintainability

### Testing

Ensure your OpenAI API key is configured before testing:
1. Run the application with the appropriate binary for your platform
2. Go to Settings tab and enter your API key
3. Test recording with the microphone button
4. Verify transcription appears in your active text field

### Contributing

1. Follow the existing code structure and naming conventions
2. Test on the target platform (Linux builds on Linux, Windows builds on Windows)
3. Update documentation for any new features
4. Ensure build commands work correctly for both platforms
