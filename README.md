# Voquill

A cross-platform voice dictation tool that converts speech to text using OpenAI's Whisper API and types it directly into any application.

## Quick Start (Developer)

### Prerequisites
- Go 1.24+ installed
- OpenAI API key
- System audio dependencies (see OS-specific sections below)

### Run from Source
```bash
# Clone and navigate to project
git clone https://github.com/jackbrumley/voquill.git
cd voquill

# Install dependencies
go mod tidy

# Run the application
go run main.go
```

On first run, the app will create a config file. Edit it with your OpenAI API key:
- **Linux**: `~/.config/voikey/config.ini`
- **Windows**: `%LOCALAPPDATA%\voikey\config.ini`

### Usage
1. Look for the Voquill icon in your system tray
2. Right-click the tray icon and select "Start Dictation"
3. Speak for 5 seconds when the "Recording..." popup appears
4. The transcribed text will be typed into the active application

## Build Instructions

### Arch Linux / Manjaro

#### Install Dependencies
```bash
# Install Go
sudo pacman -S go

# Install PortAudio
sudo pacman -S portaudio

# Install X11 development libraries (for robotgo)
sudo pacman -S libx11 libxtst libxinerama libxrandr libxss
```

#### Build
```bash
# Build binary
go build -o voquill main.go

# Make executable and run
chmod +x voquill
./voquill
```

#### Create Desktop Entry (Optional)
```bash
# Create desktop file
cat > ~/.local/share/applications/voquill.desktop << EOF
[Desktop Entry]
Name=Voquill
Comment=Voice dictation tool
Exec=/path/to/voquill/voquill
Icon=/path/to/voquill/assets/icon256x256.png
Type=Application
Categories=Utility;
StartupNotify=false
EOF

# Update desktop database
update-desktop-database ~/.local/share/applications/
```

### Ubuntu / Debian

#### Install Dependencies
```bash
# Install Go (if not already installed)
sudo apt update
sudo apt install golang-go

# Install PortAudio
sudo apt install libportaudio2 libportaudio-dev

# Install X11 development libraries (for robotgo)
sudo apt install libx11-dev libxtst-dev libxinerama-dev libxrandr-dev libxss-dev

# Install additional dependencies for robotgo
sudo apt install gcc libc6-dev
```

#### Build
```bash
# Build binary
go build -o voquill main.go

# Make executable and run
chmod +x voquill
./voquill
```

#### Install System-wide (Optional)
```bash
# Copy binary to system path
sudo cp voquill /usr/local/bin/

# Copy icon
sudo mkdir -p /usr/local/share/pixmaps
sudo cp assets/icon256x256.png /usr/local/share/pixmaps/voquill.png

# Create desktop file
sudo tee /usr/share/applications/voquill.desktop > /dev/null << EOF
[Desktop Entry]
Name=Voquill
Comment=Voice dictation tool
Exec=voquill
Icon=voquill
Type=Application
Categories=Utility;
StartupNotify=false
EOF
```

### Windows

#### Install Dependencies
1. **Install Go**: Download from [golang.org](https://golang.org/dl/)
2. **Install Git**: Download from [git-scm.com](https://git-scm.com/)
3. **Install TDM-GCC** (for CGO compilation): Download from [tdm-gcc.tdragon.net](https://jmeubank.github.io/tdm-gcc/)

#### Build
```cmd
REM Clone repository
git clone https://github.com/jackbrumley/voquill.git
cd voquill

REM Install dependencies
go mod tidy

REM Build executable
go build -o voquill.exe main.go
```

#### Create Portable Package
```cmd
REM Create distribution folder
mkdir dist
copy voquill.exe dist\
xcopy assets dist\assets\ /E /I

REM The executable is now ready to run from dist\
```

#### Build with Icon (Optional)
```cmd
REM Install rsrc tool for embedding icon
go install github.com/akavel/rsrc@latest

REM Generate resource file (if you have a .ico file)
rsrc -ico assets\icon.ico -o rsrc.syso

REM Build with embedded icon
go build -o voquill.exe main.go

REM Clean up
del rsrc.syso
```

## Configuration

Edit the config file with your settings:

```ini
WHISPER_API_KEY = sk-your-openai-api-key-here
TYPING_SPEED_INTERVAL = 0.01
```

- `WHISPER_API_KEY`: Your OpenAI API key (required)
- `TYPING_SPEED_INTERVAL`: Delay between keystrokes in seconds (0.01 = 10ms)

## Troubleshooting

### Linux Issues
- **Audio not working**: Ensure your user is in the `audio` group: `sudo usermod -a -G audio $USER`
- **Permission denied**: Make sure the binary is executable: `chmod +x voquill`
- **Missing libraries**: Install development packages for X11 and audio

### Windows Issues
- **CGO errors**: Ensure TDM-GCC is installed and in your PATH
- **Audio not working**: Check Windows audio permissions and microphone access
- **Antivirus blocking**: Add exception for the executable

### General Issues
- **API errors**: Verify your OpenAI API key is valid and has credits
- **No system tray**: Some desktop environments may not support system tray icons
- **Typing not working**: Ensure the target application has focus and accepts text input

## Features

- ✅ Cross-platform (Windows, Linux)
- ✅ System tray integration
- ✅ OpenAI Whisper API transcription
- ✅ Configurable typing speed
- ✅ Real-time status popups
- ✅ Auto-update checker
- 🔄 Planned: Global hotkey support
- 🔄 Planned: Offline Whisper support

## License

This project is open source. See the repository for license details.

## Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Test on multiple platforms
5. Submit a pull request

## Repository

GitHub: [https://github.com/jackbrumley/voquill](https://github.com/jackbrumley/voquill)
