# Voquill - Project Overview and Design Decisions

## 📝 Summary

Voquill is a cross-platform voice dictation tool designed to offer seamless voice-to-text functionality using the OpenAI Whisper API, with plans to eventually support local/offline transcription. Written in Go, Voquill prioritises portability, minimal system dependencies, and a consistent user experience on both Windows and Linux. It captures short audio recordings on demand, sends them to Whisper for transcription, and emulates keystrokes to type the result directly into any active text field. Real-time status popups and a system tray menu enhance user feedback and ease of use.

---

## 📌 Key Features

* Cross-platform (Windows and Linux)
* Simple install-and-run design with minimal dependencies
* Whisper API integration for transcription
* System tray app with menu options
* Real-time status popups using Fyne
* Configurable typing speed and API key
* Auto-generated config file if missing
* Update checker that compares version to GitHub
* Plans for:

  * Configurable keyboard shortcut
  * Optional support for offline Whisper models

---

## 🛠️ Programming Language: Go (Golang)

Go was chosen for its:

* **Strong cross-platform capabilities**
* **Simple dependency management** (`go.mod`, `go get`)
* **Compiled binary output** (no interpreter needed)
* **Concurrency and performance**

This makes the application easy to distribute, deploy, and maintain with a single codebase and no runtime requirements for end users.

---

## 🎨 GUI and System Tray Design

### Tray Menu

Implemented using [`systray`](https://github.com/getlantern/systray) for cross-platform tray integration. It provides:

* "Edit Config" to open the config file in the default editor
* "Start Dictation" to trigger the record-transcribe-type cycle
* "Quit" to exit the app

### Status Popups

Implemented with [`fyne`](https://fyne.io/) to show lightweight, non-intrusive popup windows:

* "Recording..." displayed for duration + 1 second
* "Transcribing..." displayed while waiting for API result

Popups help give users real-time feedback without requiring an active window or dialog.

---

## 🎤 Audio Capture

### Library: `portaudio`

* Used via the `github.com/gordonklaus/portaudio` Go bindings
* Captures mono, 16kHz, 16-bit audio for 5 seconds
* Saves to a temporary `.wav` file for Whisper input

### Portability Notes

* **Linux:** Requires system-level `portaudio` to be installed (e.g., `sudo pacman -S portaudio`)
* **Windows:** Bundle `portaudio.dll` with the `.exe`

Alternatives like `alsa` or `pulse` were considered but lacked cross-platform support. Static linking of `portaudio` is possible but deferred for now due to complexity.

---

## ✍️ Typing Output

### Library: `robotgo`

* Simulates typing the returned transcript character-by-character
* Controlled with a user-configurable delay (e.g., 10ms per character)

While alternatives such as clipboard-paste were considered, character-by-character typing offers better integration in most input contexts.

---

## 📦 Configuration Management

* Config stored in:

  * `~/.config/voikey/config.ini` on Linux
  * `%LOCALAPPDATA%\voikey\config.ini` on Windows

* Auto-generated on first run with default values:

  ```ini
  WHISPER_API_KEY = your_api_key_here
  TYPING_SPEED_INTERVAL = 0.01
  ```

* Users are prompted to edit the config if no API key is present

---

## 🔁 Update Checker

* Fetches the latest version from:
  `https://raw.githubusercontent.com/jackbrumley/voquill/main/version.txt`

* Compares to `installedVersion` constant and logs update availability

* Simple, non-intrusive check at startup

---

## 📂 Assets

* Icons used for both system tray and GUI popups are stored in `assets/`:

  * `icon.ico`, `icon256x256.png`, `icon1024x1024.png`
* Tray icon is loaded and converted to PNG using Go's `image` and `png` packages

---

## 🔑 Revised Architecture Plan (January 2025)

### Two-Window System Design

**1. Main GUI Window (Management Interface):**
- Can be minimized/hidden most of the time
- Used for setup, settings, and history review
- Tabs: Status, History, Settings
- Built with Fyne for cross-platform consistency

**2. Temporary Status Popup (Active Feedback):**
- Small window that appears bottom-center of screen during operation
- Shows "Recording..." while hotkey is held
- Shows "Transcribing..." while processing
- Auto-disappears when complete
- Never steals focus or interrupts typing position

### Complete User Experience Flow

1. **Setup Phase:**
   - User opens main GUI, enters API key in Settings tab
   - Configures hotkey preference (default: Ctrl+Shift+Alt)
   - Minimizes or closes main GUI

2. **Active Dictation:**
   - User positions cursor anywhere in any application
   - Presses and holds global hotkey
   - **Status popup appears**: "Recording..." (bottom-center screen)
   - User speaks for desired duration
   - User releases hotkey
   - **Status popup changes**: "Transcribing..."
   - **Status popup disappears** when transcription is typed
   - Text appears at original cursor position

3. **History Management:**
   - User can reopen main GUI anytime
   - Check History tab for previous transcriptions
   - Copy any previous transcription to clipboard

### Key Requirements

- **Global hotkey is essential** - Must work system-wide like Windows+H
- **No focus stealing** - Recording must not change active application
- **System-wide typing** - Text appears wherever cursor is positioned
- **Cross-platform GUI** - Identical experience on Windows, GNOME, KDE
- **Chat-like history** - Previous transcriptions with copy functionality

### Technical Components

- **GUI Framework**: Fyne (already integrated)
- **Global Hotkeys**: robotgo or gohook for system-wide key detection
- **Audio Recording**: PortAudio (already working)
- **Transcription**: OpenAI Whisper API (already working)
- **Keystroke Simulation**: robotgo (already working)
- **History Storage**: JSON file in config directory

## 🔑 Next Steps and Planned Features

* ✅ **Status Popups** - Implemented (needs GUI restructure)
* 🔄 **Dual-Window System** - In Progress
* 🔄 **Global Hotkey Integration** - In Progress
* 🔄 **History Management** - Planned
* ⏸️ **Auto-launch on startup** - Deferred
* ⏳ **Offline Whisper support (e.g., Whisper.cpp)** - Future scope

---

## 📁 Directory Structure

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
├── packaging/             # Packaging and deployment files
│   └── linux/            # Linux-specific packaging
├── assets/               # Application assets
│   ├── icon.ico
│   ├── icon256x256.png
│   └── icon1024x1024.png
├── build.sh              # Build script for all platforms
├── DESIGN.md             # This file
└── README.md             # User documentation
```

---

## 🧪 Testing Requirements

* Ensure Go is installed and `go.mod` initialised:

  ```bash
  go mod init voquill
  go mod tidy
  ```
* Install PortAudio:

  * Linux: `sudo pacman -S portaudio`
  * Windows: bundle `portaudio.dll`
* Edit `config.ini` and insert valid API key
* Run with:

  ```bash
  go build -o voikey
  ./voikey
  ```

---

## 🌐 Repository

GitHub Repository: [https://github.com/jackbrumley/voquill](https://github.com/jackbrumley/voquill)

---

This document captures the design, structure, and decisions of the Voquill project for future continuation or contribution. Let me know when you’d like to resume development or begin testing!
