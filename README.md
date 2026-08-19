<p align="center">
  <img src="src-tauri/icons/128x128.png" alt="Voquill Logo" width="96" height="96" />
</p>

<h1 align="center">FOSS Voquill - Private Push-to-Talk Dictation for Windows and Linux</h1>

<p align="center">Truly free, private system-wide push-to-talk dictation tool.</p>

<p align="center">
  <a href="https://www.gnu.org/licenses/agpl-3.0.html"><img src="https://img.shields.io/badge/License-AGPLv3-blue.svg" alt="AGPL v3 License" /></a>
  <a href="https://github.com/jackbrumley/voquill"><img src="https://img.shields.io/badge/Platform-Windows%20%7C%20Linux-4f46e5" alt="Platform" /></a>
  <a href="https://tauri.app/"><img src="https://img.shields.io/badge/Built%20With-Tauri-14b8a6" alt="Built with Tauri" /></a>
  <a href="https://flatpak.github.io/xdg-desktop-portal/"><img src="https://img.shields.io/badge/Wayland-XDG%20Portals-4f46e5" alt="Wayland via XDG Portals" /></a>
</p>

<p align="center">
  <a href="https://github.com/jackbrumley/voquill/releases/latest"><img src="https://img.shields.io/badge/Download-Latest%20Release-ef4444?style=for-the-badge" alt="Download Latest Release" /></a>
</p>

FOSS Voquill is offline dictation that works in any app on your system. Your voice never leaves your device, and there are no accounts, subscriptions, or cloud dependencies. On Linux, it uses XDG Portals for native Wayland support.

It exists because dictation should be a basic utility, not a locked service. If you want private, system-wide push-to-talk transcription on Linux or Windows, this is built for that workflow.

## Watch the Promo

[![Watch the Voquill promo video](https://img.youtube.com/vi/yKKyPUwEpDg/maxresdefault.jpg)](https://youtu.be/yKKyPUwEpDg)

Prefer direct link: https://youtu.be/yKKyPUwEpDg

## Install

Download the latest release here:

- **[Download Latest Release](https://github.com/jackbrumley/voquill/releases/latest)**

Release package options:

- Windows (Most users): Setup EXE (no admin)
- Windows (IT/Admin): System MSI
- Linux (Debian/Ubuntu): `.deb`
- Linux (Fedora/RHEL): `.rpm`
- Linux (Portable): `.AppImage`

Setup EXE installs per-user (no admin). MSI is a system-wide install intended for IT/admin deployment.

## Getting Started

1. **Launch Voquill**
   - On launch, you will see an initial setup screen.
2. **Configure your setup**
   - Set your hotkey, select your model, approve permissions, and choose your microphone.
   - Adjust mic sensitivity so voice is clear without clipping.
   - Use **Test Microphone** with playback to verify quality.
3. **Start dictating**
   - Focus any text field (email, docs, browser, editor).
   - Hold your hotkey `Ctrl + Shift + Space` (default), speak, then release to transcribe.

## Features

- **Private by Default** — Offline transcription via Whisper.cpp with Vulkan GPU acceleration. Your voice never leaves your device. No accounts, no cloud, no subscriptions.
- **Push-to-Talk** — Hold-to-Talk and Toggle modes. Customizable hotkey with capture UI. Press the hotkey during transcription to cancel.
- **Two Output Methods** — Typewriter (direct keystroke simulation via Wayland Portal / X11 XTest / Windows SendInput) or Clipboard with optional auto-paste via Ctrl+V.
- **Custom Dictionary** — Add names, jargon, or technical terms to improve transcription accuracy.
- **Regex Filler Word Removal** — Strips "uh", "umm", "hmm" instantly. No LLM needed. Customizable word list.
- **LLM Post-Processing** — Local (llama-server GGUF models) or cloud API (OpenAI-compatible). Multiple saved prompts. GPU acceleration with graceful fallback.
- **Speaker Diarization** — Detect and label different speakers in recordings and live dictation. Configurable clustering threshold.
- **Audio Noise Reduction** — Reduce background noise via spectral gating before transcription. Configurable strength slider.
- **Audio File Import** — Drag-and-drop or file picker. Supports WAV, MP3, M4A, OGG, FLAC. Full diarization and post-processing pipeline.
- **History with Full-Text Search** — SQLite-backed with FTS5 search. Configurable limit with auto-prune.
- **Full Wayland Support** — Uses XDG Portals for global shortcuts, input emulation, microphone access, and overlay. Works on GNOME, KDE, and other Wayland compositors. No external tools needed.
- **Windows and Linux Support** — Native on both platforms with display server auto-detection.
- **Minimalist Overlay** — Transparent status overlay during recording and transcription. Platform-native positioning (Layer Shell on Wayland).
- **CLI** — `--start-hidden` to launch to system tray. More flags coming.
- **Engine Flexibility** — Local Whisper (CPU/GPU), Parakeet/sherpa-onnx, or any OpenAI-compatible API.

> See the full feature breakdown with planned items at [docs/FEATURES.md](docs/FEATURES.md).

---

## The Philosophy

FOSS Voquill was created with a simple premise: **voice dictation should be a basic utility, not a subscription service.**

In an era of cloud-first AI, FOSS Voquill stands apart by putting privacy and freedom first:
- **No Backend**: No servers, no cloud, no data collection.
- **No Accounts**: No logins, no tracking, no onboarding.
- **Truly Free**: No subscriptions, no paid tiers. Free to use and build on.
- **Privacy First**: Your voice stays on your device. Transcription runs locally.

## Screenshots

![Voquill status overlay during transcription](docs/screenshots/screenshot-status.png)
![Voquill engine configuration and model selection](docs/screenshots/screenshot-config1.png)
![Voquill settings for shortcuts and behavior](docs/screenshots/screenshot-config2.png)
![Voquill local dictation history view](docs/screenshots/screenshot-history.png)

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
4. **Build release packages**
   - Run: `npm run tauri:build`
   - This builds production packages for your current platform.
5. **Find your built files**
   - Linux location: `src-tauri/target/release/bundle/`
   - Windows location: `C:\vb\release\bundle\`
   - This folder contains installer/package files (such as `.msi`, `.deb`, `.rpm`, `.AppImage`).
6. **Optional: Run in development mode**
   - Run: `npm run tauri:dev`
   - This is for live testing while developing.
   - It is not the final packaged release build.

On Windows, all Tauri/Cargo build artifacts are written to `C:\vb` to avoid long-path build failures (the Vulkan shader build nests deeply enough to exceed the 260-character limit under longer roots).

---

## Known Issues

- Language selection acts as a transcription hint and may not be reliably applied by all engines.
- Unicode and non-English keyboard input is best-effort; the Typewriter mode normalizes common Unicode characters (curly quotes, em-dashes, ellipses) to ASCII equivalents.
- AppImage is a cross-distro fallback, not the primary support target; desktop/portal integration can vary compared to distro-native `.deb`/`.rpm` installs.
- Fedora AppImage builds can fail due to linuxdeploy strip incompatibility with RELR system libraries.

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

This project is licensed under the GNU Affero General Public License v3.0 — see the [LICENSE](LICENSE) file for details.

### Why AGPLv3?

Voquill is my small way of giving back to the open source community.

I chose AGPLv3 to make sure it stays genuinely open for everyone. You are completely free to use, learn from, and build on this code — the only ask is that if you make improvements, those changes are shared back with the community rather than locked away.

---

### Project Provenance
*FOSS Voquill is the original project, first published in July 2025. Built to give back to the open-source community. Truly free and Open Source*

<div align="center">

[Report Bug](https://github.com/jackbrumley/voquill/issues) • [Request Feature](https://github.com/jackbrumley/voquill/issues)

</div>
