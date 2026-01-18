<div align="center">

![Voquill Logo](src-tauri/icons/128x128.png)

# Voquill

**The original, free, and private system-wide push-to-talk dictation tool.**

[voquill.org](https://voquill.org)

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Platform](https://img.shields.io/badge/platform-Windows%20%7C%20Linux-lightgrey)](https://github.com/jackbrumley/voquill)
[![Built with Tauri](https://img.shields.io/badge/built%20with-Tauri-24C8DB)](https://tauri.app/)

*Accurate voice dictation that works in any application, anywhere on your system. 100% local, 100% private, and open source under the MIT license.*

</div>

---

## The Philosophy

Voquill was created with a simple premise: **voice dictation should be a basic utility, not a subscription service.** 

In an era of cloud-first AI, Voquill stands apart by prioritizing the user:
- **Zero Backend**: No remote servers, no cloud services, and no data collection.
- **Zero Accounts**: No login required. No tracking, no onboarding, and no "Free Tiers."
- **Zero Cost**: Truly free and open source under the MIT license.
- **Privacy First**: Your voice never leaves your computer. Transcription happens 100% on your device.

---

## Features

- **Private by Default** - High-performance offline transcription ensures total data sovereignty.
- **Global Push-to-Talk** - Hold a customizable shortcut to record and release to transcribe instantly.
- **Universal Input** - Transcribed text is injected directly into your active window as keystrokes.
- **Windows and Linux Support** - Native support for Windows and Linux (Wayland).
- **Minimalist Design** - An unobtrusive overlay provides status updates without getting in your way.
- **History Management** - Quickly access, copy, and manage your previous transcriptions.

## Getting Started

### Download

Ready-to-use binaries are available for supported platforms:

**[Download Latest Release](https://github.com/jackbrumley/voquill/releases/latest)**

- **Windows**: `.msi` installer or standalone `.exe`
- **Linux**: `.deb` package, `.rpm` package, or universal `.AppImage`

### Setup Guide

Ensure you have a working microphone set as your default audio device before starting.

#### Local Mode (Recommended)

Voquill is designed to work offline. Simply launch the app and:
1. Select your preferred model size from the **Config tab** when prompted.
2. Voquill will handle the one-time download and optimization.
3. Start dictating immediately.

#### Cloud Mode (Optional)

If you prefer using the OpenAI Whisper API for cloud-based transcription:
1. Enter your OpenAI API key in the settings.
2. Ensure your account has a balance.
3. Switch the transcription mode to "Cloud".

### How to Dictate

1. **Focus**: Click into any text field (Email, Word, Browser, Code Editor, etc.).
2. **Hold**: Press and hold `Ctrl + Space` (default).
3. **Speak**: Speak clearly while holding the keys.
4. **Release**: Let go of the keys. Your speech will appear as text at your cursor.

## Technology

Voquill is built for performance and security:

- **Tauri** - Lightweight desktop framework.
- **Rust** - High-performance systems backend.
- **Whisper.cpp** - Optimized on-device speech recognition.
- **Preact** - Clean and responsive interface.
- **Deno** - Modern JavaScript/TypeScript runtime and task runner.

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

---

### Project Provenance
*Voquill was founded in July 2025 by Jack Brumley as a minimalist, privacy-first alternative to cloud-based dictation services. It remains committed to the principle that utility tools should be free, open, and local.*

<div align="center">

[Report Bug](https://github.com/jackbrumley/voquill/issues) • [Request Feature](https://github.com/jackbrumley/voquill/issues)

</div>
