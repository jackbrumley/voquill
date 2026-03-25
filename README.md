<div align="center">

![Voquill Logo](src-tauri/icons/128x128.png)

# FOSS Voquill

**Truly free, private system-wide push-to-talk dictation tool.**

[voquill.org](https://voquill.org)

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Platform](https://img.shields.io/badge/platform-Windows%20%7C%20Linux-lightgrey)](https://github.com/jackbrumley/voquill)
[![Built with Tauri](https://img.shields.io/badge/built%20with-Tauri-24C8DB)](https://tauri.app/)

*FOSS dictation that works in any application, anywhere on your system. 100% local, 100% private, and truly free.*

**[Download Latest Release](https://github.com/jackbrumley/voquill/releases/latest)**

</div>

---

## The Philosophy

FOSS Voquill was created with a simple premise: **voice dictation should be a basic utility, not a subscription service.** 

In an era of cloud-first AI, FOSS Voquill stands apart by putting privacy and freedom first:
- **No Backend**: No servers, no cloud, no data collection.
- **No Accounts**: No logins, no tracking, no onboarding.
- **Truly Free**: No subscriptions, no paid tiers. Free to use and build on.
- **Privacy First**: Your voice stays on your device. Transcription runs locally.

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

- **Windows**: setup `.exe`, portable `.exe`, or `.msi` installer
- **Linux**: `linux-x64` tarball, `.AppImage`, `.deb`, or `.rpm`

### Setup Guide

Ensure you have a working microphone set as your default audio device before starting (see **Audio Optimization** below).

#### Local Mode (Recommended)

Voquill is designed to work offline. Simply launch the app and:
1. Select your preferred model size from the **Config tab** (the **Distil-Small** model is recommended for most users).
2. Click the **Download** button to fetch the model to your device.
3. Start dictating immediately.

**Turbo Mode (GPU)**: If you have a dedicated graphics card (AMD or NVIDIA), enable Turbo Mode in Advanced settings. With Turbo Mode, the **Small** model is a great accuracy upgrade while still feeling fast.

#### Cloud Mode (Optional)

If you prefer using the OpenAI Whisper API for cloud-based transcription:
1. Enter your OpenAI API key in the settings.
2. Ensure your account has a balance.
3. Switch the transcription mode to "Cloud".

#### Audio Optimization

To ensure the highest transcription accuracy, take a moment to calibrate your audio in the **Config > Audio** section:
1. **Select Microphone**: Ensure the correct device is selected.
2. **Adjust Sensitivity**: Use the **Mic Sensitivity** slider so your voice is clear but not clipping.
3. **Test & Playback**: Use the **Test Microphone** button to record and listen to a snippet to verify clarity.

### How to Dictate

1. **Focus**: Click into any text field (Email, Word, Browser, Code Editor, etc.).
2. **Hold**: Press and hold `Ctrl + Shift + Space` (default).
3. **Speak**: Speak clearly while holding the keys.
4. **Release**: Let go of the keys. Your speech will appear as text at your cursor.

## Known Issues

- **Linux Input Support**: Current Linux input is limited to standard English characters. Support for non-English characters and Unicode symbols is planned for a future update.
- **Language Selection**: Choosing a language in settings provides a "hint" to the AI, but transcription may not always reliably follow the selected language depending on the model used.

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
*FOSS Voquill is the original project, first published in July 2025. Built to give back to the open-source community. Truly free. No subscriptions or paid tiers.*

<div align="center">

[Report Bug](https://github.com/jackbrumley/voquill/issues) • [Request Feature](https://github.com/jackbrumley/voquill/issues)

</div>
