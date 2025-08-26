<div align="center">

![Voquill Logo](rust/icons/icon-256x256.png)

# Voquill

**Cross-platform push-to-talk dictation app with Whisper-powered speech recognition**

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Platform](https://img.shields.io/badge/platform-Windows%20%7C%20macOS%20%7C%20Linux-lightgrey)](https://github.com/jackbrumley/voquill)
[![Built with Tauri](https://img.shields.io/badge/built%20with-Tauri-24C8DB)](https://tauri.app/)

*Transform your voice into text with the power of AI*

</div>

---

## ✨ Current Features

🎤 **Global Push-to-Talk** - Hold a customizable key combination to record anywhere on your system  
🧠 **OpenAI Whisper Integration** - Cloud-based speech recognition with high accuracy  
🌐 **Cross-Platform Support** - Native support for Windows, macOS, and Linux  
⚡ **Direct Text Injection** - Transcribed text appears instantly in any focused text field  
📊 **Live Visual Feedback** - Unobtrusive overlay shows recording and transcription status  
⚙️ **Simple Configuration** - Minimal UI for hotkey and audio settings  
📝 **Transcription History** - View and manage your recent transcriptions  

## 🚧 Coming Soon

🔒 **Local Privacy Mode** - Optional local processing with Whisper.cpp for complete privacy  
🌍 **Multiple Whisper Providers** - Support for various Whisper API providers beyond OpenAI  

## 🚀 Quick Start

### Download

Ready-to-use binaries are available for supported platforms:

- **Windows**: Full native support with global hotkeys and text injection
- **macOS**: Full native support using Quartz Event Services  
- **Linux**: Supported on Wayland/GNOME & KDE with proper portal support

> 📦 **Coming Soon**: GitHub Releases with pre-built binaries

### Usage

1. **Launch Voquill** - The app will open with a configuration window
2. **Add API Key** - Enter your OpenAI API key in the settings
3. **Hold & Speak** - Press and hold `Ctrl + Space` (default) anywhere on your system while speaking
4. **Release & Watch** - Speech is transcribed and automatically typed into the focused text field
5. **See Status** - Visual overlay shows "Recording" → "Transcribing" → completion

## 🛠️ Technology

Voquill is built with modern, performant technologies:

- **[Tauri](https://tauri.app/)** - Secure, fast, and lightweight desktop framework
- **[Rust](https://www.rust-lang.org/)** - Systems programming language for the backend
- **[React](https://reactjs.org/)** - Modern UI framework for the frontend
- **[Whisper](https://openai.com/research/whisper)** - Advanced speech recognition model

## 🎯 Use Cases

- **Content Creation** - Dictate blog posts, articles, and documentation
- **Coding** - Voice-driven code comments and documentation
- **Accessibility** - Alternative input method for users with mobility challenges
- **Productivity** - Faster text input for emails, messages, and notes
- **Multilingual** - Supports multiple languages through Whisper

## 🔧 Configuration

Voquill offers simple configuration options:

- **API Key** - Required for speech transcription
- **API URL** - Configurable endpoint (supports OpenAI, OpenRouter, and other Whisper-compatible APIs)
- **Custom Hotkeys** - Set your preferred push-to-talk combination (default: `Ctrl + Space`)
- **Transcription History** - View and manage your recent voice recordings and transcriptions

### Configuration File Locations

Voquill stores its configuration in the following locations:

- **Linux**: `~/.config/voquill/config.json`
- **Windows**: `%APPDATA%\voquill\config.json`
- **macOS**: `~/Library/Application Support/voquill/config.json`

To reset your configuration, simply delete the config file and restart the application.

## 📚 Documentation

For detailed technical information and development guides:

- **[Build Instructions](docs/BUILD.md)** - How to build Voquill from source
- **[Architecture](docs/ARCHITECTURE.md)** - Technical specifications and design decisions
- **[Development Setup](rust/README.md)** - Rust/Tauri specific development guide

## 🤝 Contributing

We welcome contributions! Whether it's:

- 🐛 Bug reports and fixes
- ✨ Feature requests and implementations
- 📚 Documentation improvements
- 🌍 Translations and localization

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🙏 Acknowledgments

- **OpenAI** for the incredible Whisper model
- **Tauri Team** for the amazing cross-platform framework
- **Rust Community** for the robust ecosystem

---

<div align="center">

**Made with ❤️ for seamless voice-to-text experiences**

[Report Bug](https://github.com/jackbrumley/voquill/issues) • [Request Feature](https://github.com/jackbrumley/voquill/issues) • [Documentation](rust/README.md)

</div>
