# Cross-Platform Push-to-Talk Dictation App

## Overview
The goal is to build a cross-platform desktop application that allows the user to hold down a key combination, speak into their microphone, and have their speech transcribed into text, which is then typed directly into whichever text box currently has focus. The application should behave consistently across supported platforms without fallback modes. If a platform does not support the required features, it will simply not be supported.

## Core Requirements

### Functionality
1. **Global Hotkey (While-Held)**
   - Application must register a global keyboard shortcut (e.g., `Ctrl + Shift + Alt`).
   - Recording begins when the keys are pressed and held.
   - Recording ends when the keys are released.

2. **Microphone Capture**
   - Use the system’s default microphone for recording.
   - No prompts or reconfiguration required once permissions are granted.

3. **Transcription**
   - Audio is transcribed to text using a speech-to-text engine (local or remote).
   - Transcription occurs immediately after recording stops.

4. **Text Injection**
   - Transcribed text must be typed directly into the currently focused text field in any application.

5. **Visual Feedback**
   - A small pop-up overlay at the bottom middle of the screen showing:
     - **“Recording”** while keys are held.
     - **“Transcribing”** after release until text is injected.

6. **Configuration UI**
   - A small application window for configuring settings:
     - Hotkey combination.
     - Audio input device (if multiple available).
     - Speech-to-text engine/API configuration.
   - Simple and minimal interface.

## Supported Platforms
- **Windows** (fully supported)
- **Linux (Wayland/GNOME & KDE)** — only supported if the compositor provides the necessary global shortcut and text injection APIs.

If a platform does not provide the required support, it is not supported by the application.

## Recommended Technology Stack

1. **Core Language:** **Rust**
   - Strong cross-platform capabilities.
   - Mature bindings for Windows system APIs.
   - First-class support for Wayland portals (via `ashpd`).

2. **Desktop Framework:** **Tauri**
   - Lightweight, cross-platform GUI framework.
   - Provides system tray and pop-up windows.
   - Uses Rust core logic with a WebView-based configuration UI (built with **Preact** and **Deno**).

3. **Speech-to-Text Engine:**
   - Local: [Whisper.cpp](https://github.com/ggerganov/whisper.cpp) for offline.
   - Or Remote: Configurable API for cloud-based transcription.

4. **Audio:**
   - **CPAL** or **PortAudio** bindings in Rust for capturing audio.

5. **Global Hotkeys:**
   - Windows: Native `RegisterHotKey` or low-level keyboard hook.
   - Linux: `org.freedesktop.portal.GlobalShortcuts`.

6. **Text Injection:**
   - Windows: `SendInput` API.
   - Linux: `org.freedesktop.portal.RemoteDesktop`/`VirtualKeyboard`.

7. **Overlay Pop-Up:**
   - Implemented as a Tauri window with transparent background and always-on-top flag.
   - Displays recording/transcribing state.

---

## Summary
This project delivers a **single consistent application** across Windows and Linux (where supported). It provides while-held push-to-talk dictation, live transcription, direct text injection, and clear on-screen feedback. If a system cannot meet the requirements (e.g., Linux without proper portal support), it will not be supported.
