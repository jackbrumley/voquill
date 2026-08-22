# Features

Voquill is a **private, offline-first, system-wide dictation app** for Linux and Windows.

---

## Current

### Dictation

- **Push-to-talk & Toggle** — Hold-to-Talk and Toggle modes. Press the hotkey during transcription to cancel.
- **Two output methods** — Typewriter (simulates keystrokes via Wayland Portal / X11 XTest / Windows SendInput) or Clipboard with auto-paste (Shift+Insert, Ctrl+V, Ctrl+Shift+V) and automatic clipboard save/restore.
- **Configurable paste shortcut & delays** — Universal Shift+Insert default, Ctrl+V, or Ctrl+Shift+V with configurable pre- and post-paste delays.
- **Append trailing space** — Keep your cursor positioned for the next word.
- **Auto-submit Enter** — Automatically press Enter after dictation finishes.

### Transcription

- **Local Whisper** — whisper.cpp with Vulkan GPU acceleration. Graceful fallback to CPU with UI feedback.
- **NVIDIA Parakeet** — CPU-optimized 25-language transcription with background model warm-up.
- **Cloud API** — OpenAI-compatible endpoints (any provider, any model: OpenRouter, Groq, Ollama, vLLM).
- **GPU status** — Tested/available/detail surfaced directly in settings.
- **Engine-specific settings** — Per-engine configuration panel.

### Accuracy & Post-Processing

- **Custom dictionary** — Add names, jargon, or technical terms as vocabulary prompt hints.
- **Regex filler word removal** — Built-in, deterministic, no LLM needed. Strips "uh", "umm", "hmm", etc. instantly. Configurable custom filler words.
- **LLM post-processing** — Local (llama-server GGUF models: Qwen 2.5 1.5B, Llama 3.2 1B) or cloud API (OpenAI-compatible). Customizable system prompt. GPU acceleration with fallback.
- **Multiple prompts** — Create, name, select, and delete post-processing presets. Switch between them in settings.
- **Original vs Cleaned comparison** — Retains both raw and post-processed transcripts for side-by-side review.

### Audio & Devices

- **Audio file transcription** — Drag-and-drop or file picker. Supports WAV, MP3, M4A, OGG, FLAC via symphonia (pure Rust decoding). Full diarization and post-processing pipeline.
- **Noise reduction** — Spectral gating via Python runner (noisereduce). Configurable strength. Runs before transcription.
- **Microphone & Playback device selection** — Choose your input microphone and output playback device independently.
- **Mic test & live metering** — Record, playback, and adjust input sensitivity with live volume metering.
- **Extended recordings** — Configurable maximum duration up to 180 minutes (3 hours). Post-roll (ms) to avoid cut-off.
- **Debug recordings** — Optional capture of raw WAV audio for troubleshooting with automated FIFO retention.

### Speaker Diarization

- **Multi-speaker detection** — sherpa-onnx via Python runner. Detects and labels different voices (`Speaker 1`, `Speaker 2`).
- **Separate toggles** — Independent enable for live recordings and file imports.
- **Configurable cluster threshold** — Adjust how aggressively voices are merged.
- **Per-segment transcription** — Each speaker segment transcribed independently, labeled in output and history.

### Platform & Integration

- **Wayland-native (XDG Portals)** — GlobalShortcuts, RemoteDesktop input emulation, Camera microphone, Layer Shell overlay. No external tools needed.
- **X11** — XTest keyboard simulation, native global shortcuts via tauri-plugin.
- **Windows** — SendInput keyboard simulation, native global shortcuts, CoreAudio endpoints.
- **Minimalist overlay** — Transparent status overlay during recording and transcription. Platform-native positioning (Layer Shell on Wayland).
- **System tray & Startup** — Minimize-to-tray, launch on login, and `--start-hidden` CLI flag.

### History

- **SQLite with FTS5** — Full-text search across all transcriptions and error diagnostics.
- **In-app audio playback** — Listen to recorded audio directly from history cards.
- **Original / Cleaned toggle** — View and copy either raw transcription or AI-cleaned text.
- **Status diagnostics** — Visual badges for `Failed`, `Empty`, and `Cancelled` attempts with detailed error reasons.
- **Individual item deletion** — Delete specific records or clear all.
- **Segment storage** — Speaker-labeled segments saved with diarized recordings.
- **Configurable limit & pruning** — Auto-prune oldest database entries and associated audio files when limit is reached.

### Diagnostics & Settings

- **Diagnostics page** — System health, GPU status, portal capabilities, and rotating session logs.
- **Log level configuration** — Adjust verbosity (error, warn, info, debug, trace) directly from settings.
- **Factory reset** — Clear models, logs, history, and settings back to defaults.

### Language

- **12 options** — Auto-detect + English variants (AU, GB, US) + French, Spanish, German, Italian, Portuguese, Dutch, Japanese, Chinese.
- **Spelling hints** — Region-specific spelling prompts per English variant.

---

## Planned

- **Cancel (Escape) shortcut** — Dedicated cancel hotkey, dynamically registered during recording.
- **Model unload timeout** — Auto-unload transcription model after inactivity.
- **Language identification** — Auto-detect audio language via Python runner.
- **Multi-state tray icons** — Different icons for Idle, Recording, Transcribing.
- **Model submenu** — Switch models from the system tray.
- **Portable mode** — Store all data alongside the executable.
- **Audio feedback** — Recording start/stop sounds.
- **Streaming transcription** — Live preview in overlay (requires streaming-compatible models).
- **Multi-engine architectures** — Additional engine backends (SenseVoice, Moonshine, etc.).
