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
- **Windows and Linux Support** - Native support for Windows and Linux (Wayland + X11).
- **Minimalist Design** - An unobtrusive overlay provides status updates without getting in your way.
- **History Management** - Quickly access, copy, and manage your previous transcriptions.

---

## Getting Started

**Set Up (Local Recommended)**
- Open **Config**, choose a model size (start with **Distil-Small**).
- Click **Download** to install the model locally.
- Optional: enable **Turbo Mode** in Advanced settings if you have a dedicated GPU.
- Cloud mode optional: add your OpenAI API key and switch transcription mode to **Cloud**.

**Use It**
- Focus any text field (email, docs, browser, editor).
- Hold `Ctrl + Shift + Space` (default), speak, then release to transcribe.

**Audio Quick Check**
- Select the correct microphone.
- Adjust mic sensitivity so voice is clear without clipping.
- Use **Test Microphone** + playback to verify quality.

---

## How to Build It Yourself

1. **Open Terminal in the project folder**
   - On Windows: open PowerShell in the Voquill folder.
   - On Linux: open your Terminal in the Voquill folder.
2. **Install required project files**
   - Run: `npm install`
   - This downloads everything needed to build Voquill.
3. **Check your system is ready**
   - Run: `npm run deps:check`
   - If anything is missing, it will print the install commands.
4. **Build the full app**
   - Run: `npm run tauri:build`
   - This creates installable app packages.
5. **Find your built files**
   - Location: `src-tauri/target/release/bundle/`
   - This folder contains installer/package files (such as `.msi`, `.deb`, `.rpm`, `.AppImage`).
6. **Optional: Run in development mode**
   - Run: `npm run tauri:dev`
   - This is for live testing while developing.
   - It is not the final packaged release build.

---

## Platform Support, Downloads & Known Issues

| System | Display Server | Recommended Downloads | Status | Known Issues |
| :-- | :-- | :-- | :--: | :-- |
| Windows 10/11 | Native Windows desktop | `.msi`, setup `.exe`, portable `.exe` | ✅ | None |
| Ubuntu / Debian / Linux Mint | Wayland | `.deb`, `.AppImage` | ✅ | None |
| Fedora / RHEL-based distros | Wayland | `.rpm`, `.AppImage` | ✅ | None |
| KDE-based distros (Kubuntu, KDE Neon) | Wayland | distro package (`.deb`/`.rpm`), `.AppImage` | ✅ | None |
| Linux (general compatibility) | X11 | distro package (`.deb`/`.rpm`), `.AppImage` | ✅ | None |

`✅` = tested and supported, `⚠️` = supported with caveats, `❌` = not supported

**General Known Issues**
- Linux input is currently limited to standard English characters. Non-English characters and broader Unicode support are planned.
- Language selection acts as a transcription hint and may not always be followed by every model.

---

## Technology

Voquill is built for performance and security:

- **Tauri** - Lightweight desktop framework.
- **Rust** - High-performance systems backend.
- **Whisper.cpp** - Optimized on-device speech recognition.
- **Preact** - Clean and responsive interface.
- **Vite + npm** - Frontend tooling and script orchestration.

---

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

---

### Project Provenance
*FOSS Voquill is the original project, first published in July 2025. Built to give back to the open-source community. Truly free. No subscriptions or paid tiers.*

<div align="center">

[Report Bug](https://github.com/jackbrumley/voquill/issues) • [Request Feature](https://github.com/jackbrumley/voquill/issues)

</div>
