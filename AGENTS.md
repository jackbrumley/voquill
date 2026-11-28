# Voquill Agent Manifesto & Guidelines

This document serves as a constitution for all agentic coding entities (and humans) operating within the Voquill repository. Integrity, cleanliness, and architectural soundness are our primary metrics of success.

---

## The Voquill Philosophy

### 1. Integrity Over Expediency
We do not value "quick hacks" that work today but create technical debt for tomorrow. If a feature or fix cannot be implemented cleanly, it should not be implemented until a proper architectural solution is found. 
- **No Shortcuts:** "Temporary" workarounds are forbidden. If a platform (like Wayland) restricts an action, we find the compliant API (like XDG Portals) instead of forcing a legacy hack.
- **No Half-Efforts:** Features must be substantially complete and polished. This includes proper error handling, logging, and UI feedback.
- **Clean Over Functional:** We would rather have a clean, well-organized codebase that is missing a feature than a messy one that has it.

### 2. Neatness, Tidiness, and OCD-Standard Code
Code is for humans to read, and only secondarily for machines to execute.
- **Semantic Clarity:** Variable names must be descriptive and intentional. Avoid abbreviations like `amt` for `amount` or `idx` for `index`.
- **Single Responsibility:** Functions and modules must do one thing and do it well. Large functions should be decomposed into logical units.
- **Formatting:** Strict adherence to `cargo fmt` and `npm run typecheck`.
- **Proactive Cleanup:** If you see messy code, redundant nesting, or illogical organization, you are expected to suggest a cleanup or fix it immediately (after confirming with the user).

### 3. Linux Display Server Support
Linux support targets both Wayland and X11, with clear platform boundaries.
- **Wayland Path:** Use **XDG Portals** (via `ashpd`) for hardware access (Microphone, Shortcuts, Input Emulation).
- **X11 Path:** Use native X11-compatible backends for shortcuts/input while keeping behavior aligned with Wayland as closely as possible.
- **Compositor Awareness:** Recognize that Wayland compositors (GNOME, KDE, Hyprland) have strict security models; keep those integrations explicit and future-proof.
- **Primary Delivery:** Prefer distro-native Linux packages (`.deb` / `.rpm`) where possible, and treat AppImage as the cross-distro fallback.

### 4. Root Cause First
We solve problems at their origin. If data is messy, redundant, or incorrect, do not "clean it up" at the consumer level (e.g., in the UI or intermediate wrappers). Trace the data back to its absolute source of truth and fix the generation/fetching logic there. A workaround is technical debt; a root-cause fix is engineering.

### 5. Lean, Durable Architecture (No Bloat)
We design for long-term maintainability as a solo-developed project. Architecture must remain clean and scalable without over-engineering.
- **Capability-Driven, Not Distro-Driven:** Organize by platform and protocol capabilities, not by distro names. Prefer runtime capability detection over hardcoded Fedora/GNOME/KDE branching.
- **One Owner Per Concern:** Session lifecycle, portal API integration, state transitions, and UI mapping should each have a clear single owner.
- **No Abstraction Without Payoff:** New modules or traits must reduce duplication, simplify reasoning, or improve reliability. Avoid "future-proof" layers that are unused.
- **Small, Localized Change Surface:** Future platform changes (portal updates, new compositor behavior) should require minor edits in capability/adapter modules, not architectural rewrites.
- **State Machines Over Ad-Hoc Flags:** For non-trivial flows (permissions, hotkeys, portal sessions), prefer explicit state transitions over scattered booleans.

### 8. Python Runner Architecture (Pure Execution Runtime)

The `python-runner/` directory at the project root is a self-contained Python FastAPI server that handles audio processing tasks requiring Python ML libraries. It is bundled as a Tauri resource and extracted to the config directory at runtime.

**Core Philosophy & Architecture Boundaries:**
- **Rust is Core / Python is Pure Execution:** Rust is the authoritative core of Voquill. Rust owns all network I/O, model downloads, archive extraction, streaming progress emission to Preact, process lifecycle management, and UI state synchronization. The Python runner is **strictly an execution runtime** for specialized ML inference (Piper TTS, sherpa-onnx diarization, spectral enhancement).
- **Zero Autonomous Python Downloads:** Python modules must **never** perform blocking asset downloads (e.g. via `urllib.request` or `requests`) inside request handlers. All neural models, weights, and runtime files must be downloaded, verified, and placed on disk by Rust before invoking Python endpoints. Downloading within Python causes severe UI disconnects, breaks progress indicators, and triggers HTTP client timeouts.
- **Native Rust Extraction:** Model archives (`.tar.bz2`, `.tar.gz`, `.zip`) must be acquired and extracted natively in Rust via `archive::extract_archive` directly to the runner storage directory, streaming live `model-download-progress` / `tts-model-download-progress` events to the UI.
- **Fast Inference:** Python endpoints assume local model assets are already present on disk, performing immediate inference (<100ms for TTS) and returning structured Pydantic responses.

**How it works:**
1. **Bundling:** `python-runner/**/*` is listed in `tauri.conf.json` `resources`. At build time, all Python source files are bundled with the app.
2. **Extraction:** On first use (or when the version in `python-runner/.version` differs from the expected `RUNNER_VERSION` in `src-tauri/src/python_runner/mod.rs`), Rust copies the bundled files to `~/.config/voquill-app/python-runner/`.
3. **Portable Python:** Rust downloads `python-build-standalone` (~25MB) — a relocatable Python build — to `python-runner/python/` if not present. No system Python required.
4. **Venv + Dependencies:** Rust creates a Python venv from the portable Python at `python-runner/venv/` and `pip install`s requirements from `python-runner/requirements/*.txt`.
5. **Lifecycle:** Rust spawns `uvicorn server:app` as a sidecar process (same pattern as llama-server). The server exposes:
   - `GET /health` — health check
   - `GET /capabilities` — discover available endpoints
   - `POST /diarize` — speaker diarization
   - `POST /tts/render` — render speech from a fully resolved request (voice files, speaker, speed, effects) built by Rust's `tts` module

**Capability-Based Design:**
- Each feature (diarization, TTS, enhancement) is a separate module under `python-runner/`
- Each module has a `run()` function with a standard signature
- The Rust side discovers endpoints via `GET /capabilities`
- Adding a new capability requires only a new Python module + requirements file — no Rust changes

**Tier 2 → Tier 3 Diarization Pathway:**
- **Current (Tier 2):** `diarization/provider_sherpa.py` uses `sherpa-onnx` Python package (no PyTorch, ~50MB)
- **Future (Tier 3):** Create `diarization/provider_pyannote.py` with the same `run()` signature using `pyannote-audio` + `torch`. The Rust side never changes — same `/diarize` endpoint, same response schema.
- **Upgrade trigger:** A new config field (`diarization_provider: "sherpa" | "pyannote"`) would select the backend.

**Key files:**
| File | Role |
|------|------|
| `python-runner/server.py` | FastAPI app, capability discovery, routing |
| `python-runner/tts/provider_sherpa.py` | Piper TTS rendering only (no voice catalog, settings or downloads) |
| `python-runner/diarization/provider_sherpa.py` | Tier 2 diarization (sherpa-onnx) |
| `python-runner/diarization/schemas.py` | Pydantic models (shared across providers) |
| `python-runner/requirements/base.txt` | Core deps (fastapi, uvicorn) |
| `python-runner/requirements/diarization-sherpa.txt` | Tier 2 deps (sherpa-onnx, soundfile) |
| `src-tauri/src/python_runner/mod.rs` | Rust lifecycle: extract, venv, spawn, health |
| `src-tauri/src/python_runner/tts_models.rs` | Rust base-voice catalog, file locations, download, and archive extraction |
| `src-tauri/src/tts/` | Voice decisions: persona catalog, Voice Studio presets (`voice_presets.json`), resolving a voice ID into a render request |
| `src-tauri/src/diarization/mod.rs` | Rust types: Segment, DiarizationResult, DiarizationService trait |
| `src-tauri/src/diarization/provider_python.rs` | (future) Wraps PythonRunner for DiarizationService trait |

**Version management:**
- `python-runner/.version` tracks the extracted source version
- `RUNNER_VERSION` constant in `python_runner/mod.rs` is the expected version
- On mismatch, Rust re-extracts from the app bundle (handles app updates)
- This ensures fresh installs AND upgrades always get the latest Python source
- Bump both whenever any file under `python-runner/` changes, or installed copies keep the old code
- `python-runner/.python-version` is the only pin of the portable Python version (read by Rust and by CI)

**Startup and verification:**
- Every capability in `_discover_capabilities` must load, or `server.py` exits with an error naming each one that failed; Rust reports that exit with the runner's output. A capability is never skipped silently.
- `scripts/check-python-runner.mjs` starts the runner as the app does and calls every endpoint; CI's `python-runner` job runs it on Linux, Windows and macOS. Add a new endpoint to its `ENDPOINT_CHECKS`.

### 6. Platform Adaptation Pattern
When implementing platform-sensitive features, follow this structure:
1. **Platform Boundary First:** Keep OS/display boundaries (`linux/wayland`, `linux/x11`, `windows`) as top-level separations.
2. **Provider Layer Second:** Within a platform, isolate backend/provider behavior (e.g., portal capabilities and session handling).
3. **Quirks Last:** Only add DE/provider-specific quirk modules when a real incompatibility is confirmed and cannot be solved generically.

This pattern keeps the codebase clean as new distros, compositor versions, or portal changes appear.

### 7. Architecture-First, No Band-Aids
If implementing a feature or fix requires an architectural change, the architectural change must be made first. Do not implement features in a way that circumvents the current architecture because it is easier or faster. A correct feature on a correct architecture is the only acceptable outcome. If a full-stack refactor is required, do the full-stack refactor. This is non-negotiable. A feature jammed in with a shim or workaround is not a feature -- it is technical debt that will need to be undone later at greater cost.

### 9. Single Source of Truth & Zero UI Domain Drift (Anti-Duplication)
UI visualizers, indicators, meters, and status badges must NEVER re-implement, simulate, or approximate backend domain algorithms with ad-hoc frontend logic.
- **Backend Authority:** If the backend owns an acoustic, mathematical, or stateful decision (e.g., Voice Activation Detection, envelope smoothing, hysteresis, hangover timers, key emulation, readiness gating, or text cleanup), the backend is the sole source of truth.
- **No Parallel Business Logic:** Never compute an independent `is_triggered`, `is_ready`, or `is_valid` state in TypeScript/Preact if that concept is decided by the backend. The backend must compute and emit the authoritative state (e.g., emitting `{ volume: f32, is_triggered: bool }`), and the UI must purely render that payload.
- **Zero Semantic Drift:** If a visual gauge in Settings represents a background capability (such as Voice Activation Threshold), what the user sees in the UI must be powered by the exact same engine running in production. Any divergence between what the UI displays and how the backend executes is a critical architectural defect.

### 10. Supported OS APIs Only (No Interception)
Voquill uses only the APIs each operating system provides for hotkeys, input, audio and permissions, and asks the OS for permission the documented way. It never hooks or intercepts underneath the OS, even when that would unlock a feature.
- **Supported Paths:** Windows `RegisterHotKey` (through `tauri-plugin-global-shortcut`), XDG Portals on Wayland, native X11 backends, and macOS TCC prompts (Accessibility, Input Monitoring, Microphone).
- **Forbidden:** System-wide keyboard or mouse hooks (`WH_KEYBOARD_LL`, `WH_MOUSE_LL`), swallowing input the OS reserves (Alt+Space, Win+Space and other shell shortcuts), exclusive device access, and any workaround that defeats an OS permission or security boundary.
- **Why:** Supported APIs keep working across OS updates; interception breaks with them, or breaks the OS features it overrides. A feature that needs interception is declined (see pull request 20, reserved Windows hotkeys), not shipped behind a hack.

---

## Pre-Submit Verification (Mandatory)

Before marking any task as complete, run each check as a separate command (never chained with `&&` -- if one silently fails, the chain hides it):

1. `cargo fmt` (Formatting)
2. `cargo check` (Compilation check -- zero warnings required)
3. `cargo clippy --all-targets` (Static analysis, tests included -- zero warnings required)
4. `cargo test` (Rust unit tests)
5. `npm run lint` (Frontend lint -- pre-existing warnings are acceptable, new code must not introduce additional warnings)
6. `npm run typecheck` (TypeScript integrity)
7. `npm run check:structure` (File size budget -- see below)

All seven must pass without warnings. Treat compiler warnings as errors.

---

## Architecture & Patterns

### 1. Backend (Rust)
- **Async Flow:** Use `tokio` or `tauri::async_runtime` for all I/O, network, and audio operations. Never block the main thread.
- **Error Handling:** Use `anyhow` for internal propagation to maintain context.
- **Command Safety:** Return `Result<T, String>` for all `#[tauri::command]` functions. The error string is what the frontend `Promise.reject` receives.
- **Command Allow-List:** Every command registered in `main.rs` must be listed in `src-tauri/permissions/app.toml` (`commands.allow`), and nothing else may be. A registered but unlisted command is rejected at runtime by Tauri's ACL with no compile error; the sync test in `app::commands` catches it in `cargo test` and CI.
- **State Management:** Use `AppState` (managed by Tauri) to hold shared resources like `Config`, `AudioStream`, or `RecordingState`.
- **Dictation Session Lifecycle:** `SessionState` (`Idle` / `Recording` / `Transcribing` / `Typing`) in `AppState` is the authoritative session guard; a new session may only start from `Idle`. Each session carries a cancel token in `AppState.active_session` so a cancelled pipeline can never clobber a newer session's state or status.
- **Hotkey Gestures:** All press/release semantics (hold-to-talk, toggle mode, press-to-cancel) live in `app/hotkey_handler.rs`. Platform backends (the Wayland portal loop, the X11/Windows plugin handler in `main.rs`) only feed it events; never implement gesture logic inside a backend.
- **Window & Tray Lifecycle:** `app/tray` owns the tray icon, tray availability (`is_tray_available`: always on Windows/macOS, StatusNotifier watcher on Linux), and every show/close/minimize/quit path. The tray behaves the same everywhere (left click opens the window, right click shows Open Voquill / Quit, defined once by `TrayMenuItem`); only the backend differs: Tauri's tray on Windows/macOS (`tray/system_tray.rs`), a `ksni` StatusNotifierItem on Linux (`tray/status_notifier.rs`, because Tauri's libappindicator tray reports no clicks). All close requests (title-bar button, Alt+F4, taskbar close) route through `tray::close_main_window`, which honors `config.close_to_tray` and falls back to the taskbar (logged and toasted) when no tray exists, so the app is never left running unreachable. Every "bring Voquill forward" trigger (tray click, tray menu, second launch, macOS Dock click) goes through `tray::show_main_window`.
- **Single Instance:** One Voquill per user login session (`tauri-plugin-single-instance`, registered as the first plugin). A second launch hands its arguments to the running instance (`tray::handle_second_launch`, which shows the window unless the launch was `--start-hidden`) and exits during plugin initialization. `main()` therefore does only process-local setup (environment variables); everything that touches the user's files, session log or devices (storage migration, session log rotation, panic hook, config load, audio) runs in `bootstrap::run_setup`, after the guard.
- **Crash Logging:** `session_log::install_panic_hook` records every panic (message, location, thread) in `session.log` before the default hook runs, so an unexpected exit always leaves its reason behind.
- **Modularity:** Keep hardware-specific logic isolated in modules (e.g., `audio.rs`, `typing.rs`, `hotkey.rs`).
- **Platform Capabilities:** Display backends declare what they support through the platform traits (`InputSimulation::supported_paste_shortcuts`, `GlobalShortcutEngine::supports_fn_modifier`), exposed to the UI by `get_input_capabilities` (`useInputCapabilities`). The frontend never sniffs the OS (user agent, `navigator.platform`); it offers only what the backend reports.
- **Typed Text (Typewriter Mode):** Text must arrive exactly as transcribed on any keyboard layout. Windows sends characters with `SendInput` + `KEYEVENTF_UNICODE` and macOS with `CGEventKeyboardSetUnicodeString`, so neither depends on the layout; only Enter and Tab are real key presses. Linux backends type keysyms from `platform/linux/keysym.rs` (the single char→keysym owner); X11 types only characters reachable with Shift alone in the active keymap and pastes the whole text through the clipboard (logged) when any character is not.
- **Permission Requests:** `request_audio_permission` / `request_input_permission` are one-line commands that call the active backend's `PermissionManager`; each backend owns what its platform gates (Wayland: Camera portal and RemoteDesktop session; macOS: CoreAudio prompt, Accessibility, and Input Monitoring for Fn hotkeys; X11/Windows: nothing to request). Never add per-OS `cfg` branches to these commands.
- **macOS Backend:** `platform/macos` owns Quartz input simulation, microphone (`AVCaptureDevice` authorization status, so readiness is right while the microphone is closed), Accessibility and Input Monitoring queries, and Fn event taps. Fn registration validates the entire combination and awaits listener initialization before reporting readiness; the Fn event tap runs only while the hotkey uses Fn (switching away stops it); gesture behavior stays in `app/hotkey_handler.rs`. Whisper uses Vulkan through the bundled MoltenVK driver; development requires the LunarG SDK environment documented in `docs/INSTALL.md`.
- **Sidecar & Archive Ownership:** All release-archive extraction goes through `archive.rs` (zip / tar.gz / tar.bz2; handles both flat and single-root-wrapped archives). All sidecar binary acquisition, port selection, and process spawning go through `sidecar.rs` (llama-server, sherpa-onnx, parakeet.cpp, the Python runner). Free ports are found by bind-probing, never by `connect()`, which takes ~2 s per free port on Windows. Provider modules keep only their platform→archive release knowledge and readiness probing.
- **Service Lifecycle:** `EngineFactory` (transcription) and `PostProcessFactory` (post-processing) are both stateful owners in `AppState`: Whisper models are cached/preloaded via `WhisperEngineCache`, the Parakeet sidecar is kept running by `parakeet::ParakeetSidecarCache` (fingerprinted by backend + model path + thread count, checked to be alive before reuse, stopped by `EngineFactory::unload_all`, which `save_config` calls whenever the transcription engine, model or engine settings change, so a loaded engine is always the configured one), and the local post-process llama-server sidecar is cached and reused across dictations (fingerprinted by engine + model; the system prompt is request-scoped and never restarts the server). Warm-up is fired at startup and re-armed by `save_config` (on engine/model/enable changes) and by `download_model` completion, via `app::bootstrap::spawn_engine_preload` / `spawn_post_process_warmup`; the Python runner is pre-warmed the same way (`spawn_python_runner_prewarm`) whenever `python_runner_needed` (file diarization or noise reduction is enabled); disabling local post-processing (or switching to the API provider) unloads the sidecar immediately via `PostProcessFactory::invalidate_local`. GPU engine variants follow the single `"(GPU)"` naming convention owned by `engine_factory::engine_uses_gpu`; GPU start failures fall back to CPU with the reason logged and surfaced via `get_gpu_status` / `get_post_process_gpu_status` (refetched by the frontend on the `post-process-gpu-status-changed` event). Transcription preloads are serialized by a lock inside `EngineFactory::preload`, so fire-and-forget warm-ups never double-load alongside the awaited `preload_transcription_engine` command (used by initial setup to deterministically verify GPU acceleration).
- **Parakeet Backends:** `parakeet/` serves two engines through one `ParakeetService`: "Parakeet" runs int8 ONNX models on the CPU in the sherpa-onnx WebSocket server (`parakeet/sherpa_onnx.rs`), "Parakeet (GPU)" runs GGUF models on the graphics card in the parakeet.cpp HTTP server (`parakeet/parakeet_cpp.rs`, pinned release, Vulkan on Windows/Linux, Metal on Apple Silicon). Each backend module owns only its release archive, arguments and wire protocol; process start, readiness, splitting and caching are shared. parakeet.cpp silently falls back to the CPU when no GPU starts, so the device it reports at startup is checked and a fallback is surfaced through `get_gpu_status`.
- **Engine Identity:** `engines.rs` (`ModelEngine`) is the single owner of every local engine: its name (stored in settings and shown in the UI), whether it uses the GPU, its family and model category. Nothing matches engine names as text. Settings keep the name string and parse it through `Config::transcription_engine` / `post_process_model_engine`, so an unknown name is a clear error where it is used, never an unreadable settings file. The frontend gets engines from `get_available_engines` (`EngineInfo`: name, uses_gpu, category) and models from `get_available_models`, which includes each model's `downloaded` flag; models are always identified by engine and size together (`src/models.ts`), never by size alone. Switching the transcription engine goes only through `updateConfig('local_engine')`, which keeps the model if the new engine has it and otherwise picks the engine's recommended one.
- **Speech Engine Residency:** `EngineFactory` is the single owner of whether the local speech engine is in memory; `engine_residency.rs` turns that into one published state (`Unloaded` / `Loading` / `Loaded`), exposed by `get_engine_residency` and pushed with `engine-residency-changed` (`useEngineResidency` on the frontend, shown by the home screen's `EngineResidencyChip`). Every load goes through `EngineFactory::preload` (which `create_service` also calls when the engine is not loaded), so progress is always reported. Starting a recording with the engine unloaded begins loading it at once; if it is still loading when capture ends, the status is `WakingUp` ("Waking up…") until it is ready. `config.engine_idle_unload_minutes` (0 = never) drives the idle unload in `bootstrap::spawn_engine_idle_unload`, which never unloads during a dictation or while voice macros are on (they listen through the engine). Voice macros never wake an unloaded engine (any nearby sound would undo the unload): the macro listener skips speech while the engine is `Unloaded`, so macros pause until a dictation or the home screen wakes it; the home screen asks for confirmation before unloading while voice macros are on.
- **Audio Devices:** `audio/device.rs` owns listing and resolving input/output devices. On Linux, devices are PulseAudio sources/sinks (IDs `pulse:<name>`, also served by PipeWire through pipewire-pulse) opened through cpal's `pulseaudio` host, which targets the exact device by index; cpal's ALSA host is used only when no Pulse server is running. `audio/pulse_server.rs` owns every PulseAudio connection (one shared cpal host, one endpoint client, one control connection for other applications' playback streams), replaced only after it fails: the `pulseaudio` crate never closes a dropped healthy connection, so never call `cpal::default_host()` or `cpal::host_from_id(HostId::PulseAudio)` elsewhere on Linux (a per-query host exhausts pipewire-pulse's 64-client limit and cuts audio for the whole desktop). Never route audio by setting `PULSE_SOURCE` / `PULSE_SINK` or any other process environment variable. A capture endpoint's identity includes the PulseAudio source index (`audio/pulse_server.rs`), because a re-plugged device keeps its name but gets a new index, and PipeWire leaves a stream whose device vanished on the fallback microphone instead of moving it back. `stream_buffer_size` requests short periods from PulseAudio, whose server-chosen defaults delay capture by ~1s and report playback finished before it is heard.
- **Capture Engine Lifecycle:** `app/capture.rs` is the only code that creates, replaces or tears down the capture engine, and the only code that decides whether the microphone is open. It is open while something needs it: a standing need from the settings (`keeps_microphone_open`: `config.microphone_when_idle` is `KeepOpen` (the default is `Release`), or voice macros are on) or an active `CaptureHold`. Dictation, the mic test, the spoken macro tester and the macOS permission request each take a hold with `capture::hold` and drop it the moment capture ends, however it ended; the last hold to go closes the engine when the settings do not keep it open, so webcam lights turn off and headset bases can switch to a phone. Voquill opens every stream in shared mode (WASAPI shared, PulseAudio/PipeWire, CoreAudio without hog mode); a device that reacts to Voquill is reacting to the stream being open, not to exclusive use. Dictation shows "Recording" only once its hold has opened the microphone. Startup, settings saves, factory reset and the watchdog call `sync_with_settings`; a changed input selection is checked first by `apply_input_change`, which opens the new device only when the microphone should be open and otherwise only resolves it. Opening the engine goes through `ensure_engine`, which rebuilds the engine when it is missing, its stream reported a fatal error, its endpoint identity no longer matches what the configured device resolves to now (selection changed, "System Default" moved, device re-plugged), or the sensitivity changed. An unresolvable device tears the engine down rather than letting audio come from a fallback microphone. A watchdog runs the same check every few seconds while nothing is recording. Each change re-attaches the voice macro listener and emits `audio-devices-changed`, which the UI answers by re-probing devices and readiness. `save_config` applies input changes under the mic-test lock, and the frontend saves device selections immediately (no debounce).
- **Other Audio While Recording:** `audio/other_audio` is the single owner of lowering or muting other applications' audio during a recording (`config.other_audio_while_recording`: Keep / Lower / Mute). The recording flow adjusts it beside capture and restores it as soon as capture ends, however it ended; `AppState::cleanup` restores it on exit. Restoring only undoes Voquill's own change (a stream the user muted or adjusted is left alone) and Voquill's own streams are never touched. Backends: PulseAudio sink inputs through `pulse_server`'s control connection (Linux), WASAPI audio sessions (Windows); macOS has no per-application volume API, so `get_audio_capabilities` reports it unsupported and the UI hides the setting.
- **Playback:** `audio::AudioPlayer` (in `AppState`) owns every output stream; play through `AppState::play_audio`, the single place that applies `config.playback_device`. Each `PlaybackSlot` plays one sound at a time (`Preview`: mic test and history recordings; `MacroFeedback`: voice macro chimes and TTS), and a sound's `on_end` runs exactly once whether it finished, was stopped or was replaced. The webview never plays audio itself: History playback goes through `play_history_recording`, so it honours the selected playback device.
- **Voice Macro Hearing:** `audio::speech_segmenter` is the single owner of how live audio becomes spoken phrases (30 ms VAD frames, ~210 ms lead-in, 0.3 s minimum, 4 s maximum), and `voice_macro::recognition::recognize_phrase` of how a phrase becomes a transcript and macro match. The background listener, the spoken macro tester (which plays back the exact clip the listener would receive) and the mic test meter all use them; never re-implement framing or phrase rules elsewhere.
- **Long Audio:** Each transcription provider owns its per-request limit and splits longer audio with `audio::chunking::split_at_pauses`, which cuts at the quietest moment near the limit (Parakeet: 60 s pieces, below sherpa-onnx's 300 s `--max-utterance-length`; API: 600 s pieces, below the 25 MB upload limit; Whisper has no limit). Callers always pass the whole recording.
- **Speech Level:** `audio::level` is the single owner of loudness: it measures the speech (loud 20 ms frames, not the pauses), boosts quiet microphones up to +30 dB, keeps loud ones below clipping and leaves silence alone. `finalize_captured_audio_for_whisper` applies it to every recording before trimming, and the mic test plays back that same prepared audio and reports `is_too_quiet` (decided by `level::measure`) with `mic-test-playback-started`; the UI only shows the warning.
- **Audio File Decoding:** Imported audio files (m4a/mp3/flac/ogg/wav) are decoded with `symphonia` (pure Rust, no external binaries) in `audio/decode.rs`; `audio/conversion.rs` owns normalization to 16kHz mono WAV for whisper. WAV inputs keep the original hound fast path.

### 2. Frontend (Preact, lives in `src/`)
- **Strict TypeScript:** No `any`. Explicit interfaces for all data structures (API responses, State slices).
- **Domain Hooks (Required):** State, effects, and handlers for a single domain must be co-located in a dedicated hook file (e.g., `useConfig.ts`, `useAudioSetup.ts`). Do not scatter related state across multiple hooks or leave it in the parent component. A hook should own its state, its effects, and its Tauri `invoke` calls. The parent component only wires hooks together and renders.
- **Signals-Only State Management (Strict Prohibition on useState):** `useState` is FORBIDDEN. All reactive state MUST use `useSignal` or `signal` from `@preact/signals`. `useCallback` is FORBIDDEN -- signals make it unnecessary. `useMemo` MUST be replaced with `useComputed`. `useEffect` MUST use `useSignalEffect` when reacting to state changes. Mount-only effects (event listeners, one-time probes) may use `useEffect` with an empty dependency array. `useRef` is FORBIDDEN for mutable state values -- use `useSignal` instead. `useRef` is ONLY permitted for DOM element references. Any new code introducing `useState`, `useCallback`, `useMemo`, or `useRef` for state will be rejected.
- **Styles (Current Convention):** Prefer component-local inline style objects with design tokens for layout, spacing, and color. Use global CSS (`index.css`) for resets, root-level variables, and truly global concerns only.
- **Style Consistency:** When touching existing UI, follow the style approach already used in that component/file. Do not introduce a separate styling pattern unless there is a clear architectural reason.
- **Tauri Core:** Use `@tauri-apps/api` for communication with the backend.
- **Readiness Gate:** `app::readiness` (Rust) is the single source of truth for "can the app be used right now": permissions (`platform_setup_status`), a real input device (the synthetic "System Default" entry is marked `is_system_default`), the configured device being present, transcription readiness (Local: the configured engine+model is downloaded; API: a non-placeholder key) and hotkey registration. `decide` is the pure, unit-tested rule; `evaluate` gathers the inputs and caches the verdict; `publish` re-evaluates and emits `app-readiness-changed`, and is called wherever an input changes (settings saves, model downloads, permission requests, capture-engine changes, Wayland input session changes, and `set_hotkey_error` / `set_hotkey_binding_state`, the single writers of hotkey state). The frontend only displays it (`useAppReadiness`, refreshed on window focus). `start_recording` uses the cached verdict and re-evaluates a cached "not ready" before refusing (then it shows the window); voice macros use the cached verdict. `useInitialRoute` owns all redirects: a one-shot launch decision plus a navigation guard rewriting any Home navigation to `#/setup` while unready. Never compute readiness in the frontend.

- **Initial Setup:** `InitialSetupPage` is a sequence of steps (`src/pages/setup/`: permissions → microphone → speech engine → try it). `useSetupSteps` derives each step's state from the backend readiness verdict only; it never decides readiness itself. The speech engine choices come from `get_setup_recommendation` (`setup_recommendation.rs`), which picks them from the hardware (`hardware_detection.rs`: graphics devices and system memory as ggml reports them, the same Vulkan/MoltenVK view the engines use, plus processor threads) and the configured language, in tiers: a suitable graphics card gets Parakeet on the GPU, a modest computer gets the light Whisper model first, everything else gets Parakeet on the processor. Each recommendation carries a plain `explanation` (what was found on this computer) and per-option `speed` / `accuracy` ratings (1-4), so people on different hardware understand why their engine differs; setup, Settings → Transcription and the Home engine line show them and never re-derive them. The built-in default engine and model come from the same rules (`setup_recommendation::baseline_choice`). The setup screen only renders them, in plain language. **Showing setup again after updates:** `SETUP_REVISION` (`setup_review.rs`) is bumped deliberately, never per version, when setup has something new for existing users. `config.completed_setup_revision` is backend-owned (`save_config` keeps the backend's value) and written by `complete_setup_review` when setup is finished or skipped. On launch a ready app asks `get_setup_review`; whenever the revision is behind, the review is due for every user, whichever version they updated from and whatever engine they run (cloud users included, a failed hardware check included), and the backend brings the main window forward even after a hidden autostart. Local users start from the recommended (first) choice; `already_recommended` tells the screen whether they already run it, and "Keep my current setup" appears only when the recommendation differs. Nothing records the revision except `complete_setup_review` (finishing or keeping), so a review closed unanswered opens again next launch.

### 3. Storage Locations & App Identity
- **Single Owner:** `src-tauri/src/paths.rs` owns every on-disk location. Never call `dirs::config_dir()` / `dirs::home_dir()` directly from feature code -- use the `paths::` helpers (`app_root`, `ensure_app_root`, `settings_file`, `history_db`, `models_dir`, `python_runner_dir`, `debug_dir`).
- **Unified Root:** Everything Voquill persists (settings.json, history.db, models/, python-runner/, debug/) lives under `~/.config/voquill-app` on every platform. Linux honors `XDG_CONFIG_HOME` via `dirs::config_dir()`; Windows/macOS resolve `dirs::home_dir()/.config`, deliberately keeping multi-GB models out of roaming `%APPDATA%`. The folder is `voquill-app`, not `voquill` (see Variations).
- **Config Defaults:** `Config::default()` in Rust (`config/defaults.rs`) is the only source of default settings. Brand-new settings (first run, factory reset) are then adjusted to the computer by `setup_recommendation::apply_first_run_defaults` (system English variant as the language, post-processing engine from the hardware); existing settings are never changed. `app::default_config_script` hands it to every webview as `window.__VOQUILL_DEFAULT_CONFIG__` before page scripts run; `src/defaultConfig.ts` reads it for `useConfig`'s pre-load placeholder (never persisted). Never write default values in the frontend. Settings values carry their unit in the name and are stored in that unit (e.g. `typing_interval_ms`); renamed or re-unitized fields are converted in `config::storage::migrate_legacy_fields`.
- **Config Persistence:** `src-tauri/src/config/storage.rs` owns reading and writing `settings.json`. Saves are atomic (temp file + rename). A file that exists but cannot be read aborts startup with a native dialog rather than risk overwriting it; a file that cannot be parsed is moved to `settings.json.invalid-<timestamp>` and the user is told via a dialog. On the frontend, `useConfig` must never auto-save before the on-disk config has loaded -- persisting its in-memory defaults would overwrite the user's settings.
- **Legacy Migration:** `bootstrap::initialize_storage_and_logging` runs the storage migrations before any other storage access. `paths::migrate_legacy_location()` moves the pre-1.4.3 directory (`<os config dir>/foss-voquill`, e.g. `%APPDATA%\foss-voquill`) into the new root (rename, copy-fallback, explicit logging of every outcome). A new root that holds nothing but `debug/` counts as empty: the installers write `debug/update.log` there before the new app first starts, so the legacy folder is merged into it (existing files win) instead of being left behind. Then `paths::migrate_legacy_settings_file()` renames the settings file of versions up to 1.9.5 (`config.json`) to `settings.json`: only when the old file exists and the new one doesn't; when both exist it logs a warning and keeps `settings.json`.
- **App Identifier:** The Tauri identifier, Wayland app_id, desktop file, and metainfo ID are all `org.voquill.voquill` -- one reverse-DNS identity across every surface (namespace owned via the voquill.org domain). In code it is owned by `APP_ID` in `src-tauri/src/app/identity.rs`, with tests that keep it equal to the Tauri `identifier` and check that `src-tauri/packaging/<APP_ID>.desktop` and the metainfo are named after it. The ID must never end in `.desktop` (KDE's kglobalacceld treats such names as desktop-file launch shortcuts and silently drops portal global shortcuts registered under them) or `.app` (collides with macOS bundle names). The previous IDs `org.voquill.desktop` and `org.voquill.app` survive only in legacy-cleanup lists.
- **Desktop File Ownership:** KWin/Mutter and xdg-desktop-portal resolve the app by `<app_id>.desktop`, so the launcher must be `org.voquill.voquill.desktop`. Tauri's bundler names its generated desktop file after `productName` with no override, so the real entry is `src-tauri/packaging/org.voquill.voquill.desktop`, shipped explicitly via `bundle.linux.deb.files` / `bundle.linux.rpm.files`. The bundler's own output is rendered from `packaging/bundler-stub.desktop.hbs` as a `Hidden=true` stub so it never shows as a second launcher. The AppImage reuses the deb template, so `bundle.linux.appimage.files` overwrites the stub path with the real entry; `scripts/tauri-runner.mjs` fails the build if the AppImage's top-level launcher ever ends up hidden. `docs/install.sh` writes the AppImage launcher as `org.voquill.voquill.desktop` too. Delete the stub once Tauri can name the desktop file directly.

### 4. CI, Releases & Installers
- **CI Is the Cross-Platform Gate:** `.github/workflows/ci.yml` runs typecheck, lint, and `cargo fmt`/`clippy --all-targets -D warnings`/`test` on Linux, Windows and macOS (Apple Silicon) for every push to `dev`/`main` and every PR. The maintainer has no Mac and rarely boots Windows, so a red CI job is the only signal that platform code broke; never merge or release over one.
- **One Build Environment:** `.github/actions/setup-build` owns Node, Rust, the LunarG Vulkan SDK and per-OS toolchains for both CI and release jobs. Each toolchain version is pinned in exactly one file, read by local builds, CI and the Linux release container alike: `.node-version`, `rust-toolchain.toml`, and `src-tauri/packaging/vulkan-sdk.json` (version plus the Linux tarball's SHA-256 from LunarG). Bump a version only there.
- **Releases Are Built by GitHub:** `docs/RELEASE.md` is the procedure. `npm run release:prepare -- X.Y.Z` bumps, commits and tags on `dev`; pushing the tag runs `release.yml` (build all platforms → pre-release → `verify-release.mjs` → install test on all three OSes); `npm run release:promote` publishes and fast-forwards `main`. Release identity (asset names, expected asset list, download links) is owned solely by `scripts/release-assets.mjs`.
- **Branches:** `dev` is where work lands; `main` only moves by `release:promote` or a site-only update (Release Standard §3), and always equals the latest published release. After any site-only commit on `main` (web editor included), merge `main` into `dev` so `main` stays an ancestor of `dev`; otherwise the next promotion cannot fast-forward, and `release:promote` refuses before publishing. GitHub Pages serves the website and installers from `main:/docs`, so installer changes go live only on promotion.
- **Installers by Platform:** `docs/install.sh` / `docs/uninstall.sh` (Linux) hand off to `docs/install-macos.sh` / `docs/uninstall-macos.sh` on macOS, so the one-line command is identical; `docs/install.ps1` serves Windows. The macOS scripts must stay bash 3.2 compatible and must not need python3.
- **In-App Updater:** The website owns the update, not the app. Update Now (`app/commands/update.rs`) only starts a hidden shell that downloads the current `docs/update.ps1` / `docs/update.sh` and runs it, so any fix to the update procedure ships by changing that script, with no release. The update script runs the same installers, writes everything to `debug/update.log`, and on failure relaunches the installed version with the hidden `--update-failed` flag (a dialog pointing at the log). The script's first line is `[voquill-update] started`; the app exits only after seeing it, and otherwise stays open and offers the download page, so an update can never close the app and do nothing. The scripts' parameters and output lines are a contract with every shipped version (documented at the top of each script; a test keeps the started line in sync): change them only compatibly. The hidden `--update` flag runs the same path headless, which the release workflow's install test uses on every platform. `VOQUILL_UPDATE_BASE_URL` (https only) points the updater at other scripts, for that test. Under an AppImage the updater gets the system environment: every entry under `$APPDIR` is removed (the bundled libraries otherwise break the system `curl`).

---

## Platform Compatibility & Requirements

| Platform | Display Server | Audio Backend | Hardware Access |
| :--- | :--- | :--- | :--- |
| **Linux** | Wayland, X11 | ALSA / PulseAudio | Wayland: XDG Portals (`ashpd`), X11: native X11 backends |
| **Windows** | Desktop | WASAPI | CoreAudio API |
| **macOS** (Apple Silicon, community-supported) | Quartz | CoreAudio | Accessibility / Input Monitoring (TCC) |

### Linux Permission Setup
On Wayland, Voquill triggers standard XDG Portal prompts for microphone, global shortcuts, and remote desktop (input simulation). On X11, equivalent capabilities use native X11 backends and should still surface clear setup/readiness state in the UI.

---

## Development Workflow for New Features

When adding a new feature, follow this sequence:
1.  **Analyze Environment:** Check for platform-specific constraints (Wayland and X11 where relevant).
2.  **Scaffold Backend:** Implement the logic in a new or existing Rust module.
3.  **Expose Command:** Create a `#[tauri::command]`, register it in `main.rs`, and add it to `commands.allow` in `src-tauri/permissions/app.toml`. Tauri rejects any command missing from that allow-list, so the frontend call fails while the code compiles and runs; `app::commands::tests` fails `cargo test` whenever the two lists differ.
4.  **Implement UI:** Create the Preact component and hook it up to the command using `invoke`.
5.  **Verify Integrity:** Run `cargo clippy`, `npm run typecheck`, and `npm run lint`.
6.  **Test Platform Parity:** Verify the feature works on Linux (Wayland and X11) and Windows.

---

## Compiler-Driven Refactoring & Zero-Shim Mandate

To prevent architectural decay and accumulation of technical debt:

### Compiler-Driven Refactoring
- When introducing or refactoring a function signature, always make new parameters **mandatory** (not `Option<T>` with a default, and no deprecated wrappers).
- The compiler surfaces every call site that needs updating. Treat the compiler error list as your complete to-do list.
- Fix every location in the same pass. Do not leave a deprecated facade for later cleanup.

### Zero-Shim Mandate
- No re-export files, no backwards-compatibility wrappers, no "legacy" module facades.
- If you rename a module, function, or type, update every consumer stack-wide in the same commit.
- Delete the old path entirely. A single import pointing at a renamed file is a shim, not a refactor.
- This applies across all layers: Rust modules, Tauri commands, TypeScript components, and frontend API calls.

### Fail-Hard Doctrine
- Prefer explicit errors over silent fallbacks. If a required resource, config value, or capability is missing, fail hard with a clear diagnostic message.
- Do not add silent fallback logic (e.g., defaulting to CPU when GPU is requested without informing the user). If a fallback is architecturally necessary, it must be explicit, logged, and surfaced to the UI.
- The `Result<T, String>` pattern in Tauri commands already supports this -- use it rather than swallowing errors.

---

## File Size Budget

To keep files focused and maintainable:

- **Soft Limit (600 lines):** A signal of architectural decay. At 600 lines, stop and plan a structural extraction (sub-modules, helper utilities, or dedicated service layers).
- **Hard Limit (1000 lines):** A critical build error. Files at or above 1000 lines must be structurally refactored before any further changes are made to them.
- **Enforced:** `npm run check:structure` (`scripts/check-file-lines.mjs`, also run by CI) fails on any file in `src/` or `src-tauri/src/` at or above the hard limit and lists how many are over the soft limit.
- **Anti-Compression Rule:** Do NOT reduce line counts by compressing style blocks onto single lines, merging multiline statements, deleting blank lines, or collapsing logical blocks. The only acceptable reduction is a proper structural extraction.

### Domain-Driven Extraction (Mandatory)

When a file exceeds the size budget, the extraction strategy must follow these rules, in order of priority:

1. **Extract by Domain, Not by Convenience.** Identify the cohesive, self-contained concerns within the file. A domain is a group of state, effects, and handlers that change together and can be tested independently. Extract entire domains, not the easiest functions to move. Do not cherry-pick small utility functions to hit a line count while leaving the real architectural problem intact.

2. **One Concern Per File.** Each file must own exactly one domain. Do not create catch-all files like `utils/helpers.ts`, `hooks/useMisc.ts`, or `lib/utils.ts`. If a group of functions does not form a coherent domain, it is not ready to be extracted.

3. **Cohesion Over Convenience.** Group by what changes together. Handlers that read and write the same state belong in the same hook even if extracting them is harder than moving a pure function to a utils file. The hard extraction is the correct one.

4. **No "Easy Win" Shortcuts.** Extracting a pure function to a `utils/` file because it's easy, while leaving the tangled state and effects in place, is not a refactor -- it is cosmetic rearrangement. The hard part is unpacking the state from the component. That is the part that must be done.

5. **Verify by Locality.** After extraction, verify that the extracted domain is self-contained: it should own its own state, its own effects, and its own handlers. If the extracted module still depends on being called from within a specific parent component lifecycle, the extraction was not deep enough.

6. **Preserve the Public API.** The parent component's interface (props, exports, render) should remain unchanged after extraction. The refactoring is internal; consumers of the component should not need to know that the extraction happened.

---

## High-Priority Architectural Fixes (Current Debt)

Any agent working on this repo should prioritize the following cleanups:
1.  **Redundant Nesting:** ~~The `src/src` structure is messy and redundant.~~ **Resolved:** the frontend now lives directly in `src/` and the backend in `src-tauri/`. Keep this flat structure clean.
2.  **NPM/Cargo Synergy:** Keep frontend and Tauri script orchestration in npm, and Rust build logic in Cargo/Tauri.
3.  **Local Whisper Integration:** Follow the roadmap in `src/LOCAL_WHISPER_INTEGRATION_PLAN.md` if working on transcription features. Ensure model management is clean and asynchronous.

---

## Release and Update Standard

Voquill is the reference implementation of the shared
[Release and Update Standard](https://github.com/jackbrumley/jackbrumley.github.io/blob/main/RELEASE_STANDARD.md)
that Rusty G6, SomeDrive and Yambuck also follow. The procedure is
`docs/RELEASE.md`. When you change release tooling, the installer scripts or
the in-app updater here, check whether the standard (and the other apps)
should change too, and say so; when Voquill and the standard disagree, fix
whichever is wrong so they agree again.

It is also the reference for the shared
[App Standard](https://github.com/jackbrumley/jackbrumley.github.io/blob/main/APP_STANDARD.md)
(window, logging, diagnostics, settings, CI and repository conventions).

## Variations

Deliberate differences from the two standards:
- **Config folder:** `~/.config/voquill-app` on every platform, not
  `~/.config/voquill`. Another, unrelated app is also called Voquill; the
  `-app` suffix keeps the two from sharing a folder.
- **productName:** `"Voquill"` (capitalised) in `src-tauri/tauri.conf.json`,
  not the standard's lowercase name. Windows names the install folder after
  it, so changing it would move existing Windows installs. Linux already
  overrides it to `voquill` in `src-tauri/tauri.linux.conf.json`.
- **Installer `--clean`:** `docs/install.sh`, `docs/install-macos.sh` and
  `docs/install.ps1` (`-Clean`) accept `--clean`, which deletes cached data
  (`models/`, `python-runner/`, `debug/`) before installing and keeps
  `settings.json` and `history.db`.

## Interaction Guidelines for Agents

- **Look for Improvement:** Don't just implement the request. Analyze the surrounding code for "mess" and offer to tidy it up.
- **Correct Inaccuracies Proactively:** If a user statement is technically incorrect or based on a false assumption, explicitly correct it and proceed with the correct approach. Do not silently follow an incorrect premise.
- **Ask, Don't Assume:** If a cleanup involves structural changes (like moving folders or renaming modules), always explain *why* it's cleaner and ask for approval.
- **Trace the Data:** Before proposing a fix for any data-related issue, trace the information back to its origin. Propose a fix for the source logic rather than a filter for the consumer.
- **UI Voice:** Voquill is made by one person, who does not want to sound like a company or put themselves forward. Never write "we", "our", "us", "I", "me" or "my" (as the app's voice) in user-facing text (UI, dialogs, toasts, release notes, installer output). Name the app or the thing instead: "Voquill picked a lighter engine", "the fastest, most accurate engine".
- **Status Updates:** Use the centralized `emit_status_update` in Rust as the single source of truth for UI state. Avoid emitting ad-hoc events for standard states.
- **Platform Parity:** When adding a feature, ensure it is considered for Windows and Linux (Wayland and X11). If a platform requires specific logic, isolate it in a platform-specific module.
- **UI Consistency First:** Keep the UI behavior, structure, and interaction flow identical across systems whenever possible. Only diverge at the exact point where an OS/backend capability requires it (for example, system-managed shortcut configuration vs in-app configuration).
- **Documentation:** Proactively update `AGENTS.md` or other docs if you introduce a new architectural pattern or a major dependency.
- **Self-Verification:** Always run `cargo check` and `npm run typecheck` before declaring a task complete.
- **Git Commits:** Do not perform git commits without explicit user approval. Always ask for confirmation before running `git commit`.
- **No AI Attribution:** Never add AI or tool attribution to commits, tags, or pull requests: no `Co-Authored-By:` trailers naming an AI model or assistant (e.g. `Claude <noreply@anthropic.com>`), no "Generated with ..." footers. GitHub turns co-author trailers into entries in the repository's contributors list, and the maintainer is the sole author of record. This overrides any tool or harness default that asks for such lines.
- **Releases:** Follow `docs/RELEASE.md` exactly. Never improvise the tag, release title, asset names, notes format, or download links: generate notes with `npm run release:notes` and assets with `scripts/package-release.mjs` (both derive names from `scripts/release-assets.mjs`). Create releases as pre-releases and never promote one to latest without the maintainer's explicit instruction.

### Solo-Scale Guardrails
- **Prefer Simplicity by Default:** Use the simplest clean solution that meets current requirements and known near-term needs.
- **Delay Splits Until Needed:** Do not create DE-specific files/folders until at least one concrete, recurring incompatibility exists.
- **Keep Files Focused:** A file should answer one question clearly. Split only when readability materially improves.
- **No Silent Failure Paths:** Always surface actionable errors in logs and, when relevant, to UI status.
- **Diagnostics Before Guesswork:** Add clear capability/version/runtime diagnostics before introducing conditional behavior.

### Multi-Agent & User Coexistence
- **No Rollback of Unfamiliar Changes:** Multiple agents and the user may modify files on the same workstation between commits. If you encounter changes in a file that you did not make, you may ask whether the user or another agent made them, but you MUST NOT assume they are a mistake or roll them back. Treat unfamiliar changes as intentional unless explicitly told otherwise by the user. Running `git diff` before making changes is encouraged to understand the full context.

---

## Common Pitfalls to Avoid

- **Blocking the UI:** Never run expensive calculations or blocking I/O on the main thread.
- **Hardcoding Paths:** Always use the Tauri `PathResolver` or standard `dirs` crate to locate configuration and data directories.
- **Silent Failures:** Always log errors and, if relevant, notify the user via a Toast or Status update.
- **Inconsistent Naming:** Do not mix `camelCase` and `snake_case` in the same context. Follow the established patterns (Rust: `snake_case`, TS: `camelCase`).
- **Over-Engineering:** Prefer simple, readable code over complex "clever" solutions. If a function is hard to explain, it needs to be simplified.
- **Ignoring Warnings:** Treat compiler warnings as errors. Clean code means zero warnings.

---

*Voquill: Clean code is a requirement, not a feature.*
