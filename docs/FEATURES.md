# Features

Voquill is a **private, offline-first, system-wide dictation app** for Linux and Windows.

---

## Current

### Dictation

- **Push-to-talk** — Hold-to-Talk and Toggle modes. Press the hotkey during transcription to cancel.
- **Two output methods** — Typewriter (simulates keystrokes via Wayland Portal / X11 XTest / Windows SendInput) or Clipboard (copy to clipboard).
- **Append trailing space** — Keep your cursor positioned for the next word.
- **Auto-submit Enter** — Automatically press Enter after dictation finishes.

### Transcription

- **Local Whisper** — whisper.cpp with Vulkan GPU acceleration. Graceful fallback to CPU with UI feedback.
- **Cloud API** — OpenAI-compatible endpoints (any provider, any model).
- **Parakeet / sherpa-onnx** — NVIDIA Parakeet models for CPU-optimized transcription.
- **GPU status** — Tested/available/detail surfaced in the UI.
- **Engine-specific settings** — Per-engine configuration panel.

### Accuracy

- **Custom dictionary** — Add names, jargon, or technical terms as a Whisper prompt hint.
- **Regex filler word removal** — Built-in, deterministic, no LLM needed. Strips "uh", "umm", "hmm", etc. instantly. Configurable custom filler words.
- **LLM post-processing** — Local (llama-server GGUF models) or cloud API (OpenAI-compatible). Customizable system prompt. GPU acceleration with fallback.
- **Multiple prompts** — Create, name, select, and delete post-processing prompts. Switch between them in settings.

### Audio

- **Noise reduction** — Spectral gating via Python runner (noisereduce). Configurable strength. Runs before transcription.
- **File import** — Drag-and-drop or file picker. Supports WAV, MP3, M4A, OGG, FLAC via symphonia (pure Rust decoding).
- **Mic test** — Record, playback, and adjust input sensitivity with live metering.
- **Device selection** — Choose your microphone. Post-roll (ms) to avoid cut-off.
- **Recording logs** — Save raw WAVs for debugging.

### Speaker Diarization

- **Multi-speaker detection** — sherpa-onnx via Python runner. Detects and labels different voices.
- **Separate toggles** — Independent enable for live recordings and file imports.
- **Configurable cluster threshold** — Adjust how aggressively voices are merged.
- **Per-segment transcription** — Each speaker segment transcribed independently, labeled in output and history.

### Platform

- **Wayland-native (XDG Portals)** — GlobalShortcuts, RemoteDesktop input emulation, Camera microphone, Layer Shell overlay. No external tools needed.
- **X11** — XTest keyboard simulation, native global shortcuts via tauri-plugin.
- **Windows** — SendInput keyboard simulation, native global shortcuts.

### Customization

- **Global hotkey** — Configurable modifier+key combo. Capture UI (press your combination).
- **Hotkey modes** — Hold to Talk or Toggle.
- **Typing speed** — Configurable character delay and key hold duration.
- **Overlay position** — Vertical offset from screen bottom (Wayland Layer Shell: compositor-managed).

### History

- **SQLite with FTS5** — Full-text search across all transcriptions.
- **Segment storage** — Speaker-labeled segments saved with diarized recordings.
- **Configurable limit** — Auto-prune oldest entries when limit is exceeded (default 500).

### System Integration

- **System tray** — Left-click shows window, right-click menu (Open / Quit). Minimize-to-tray.
- **Autostart** — Launch on login via tauri-plugin-autostart.
- **Update checking** — GitHub API release check.
- **CLI** — `--start-hidden` to launch to tray without showing the main window.

### Diagnostics

- **Diagnostics page** — System health, GPU status, portal capabilities, session logs.
- **Session logging** — Rotating session logs with timestamps.
- **Recording logs** — Save raw audio for debugging.
- **Factory reset** — Clear models, logs, history, and settings.

### Language

- **12 options** — Auto-detect + English variants (AU, GB, US) + French, Spanish, German, Italian, Portuguese, Dutch, Japanese, Chinese.
- **Spelling hints** — Region-specific spelling prompts per English variant.

---

## Planned

- **Cancel (Escape) shortcut** — Dedicated cancel hotkey, dynamically registered during recording.
- **Recording retention** — Auto-cleanup of old debug WAV files.
- **Paste delays** — Configurable delay before and after text insertion.
- **Model unload timeout** — Auto-unload transcription model after inactivity.
- **Language identification** — Auto-detect audio language via Python runner.
- **Multi-state tray icons** — Different icons for Idle, Recording, Transcribing.
- **Model submenu** — Switch models from the system tray.
- **Portable mode** — Store all data alongside the executable.
- **Log level configuration** — Adjust verbosity from settings.
- **Audio feedback** — Recording start/stop sounds.
- **Streaming transcription** — Live preview in overlay (requires streaming-compatible models).
- **Multi-engine architectures** — Additional engine backends (SenseVoice, Moonshine, etc.).