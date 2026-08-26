# Voquill Voice Macro Audio Feedback & Local TTS Engine Specification

## Status: Approved Architecture / Implementation Plan
**Date:** August 2026  
**Authors:** Voquill Core Team  
**Scope:** Voice Macro Audio Feedback, Offline Neural Text-to-Speech (TTS), Custom Audio File Import, and In-App Microphone Recording  

---

## 1. Executive Summary

Auditory confirmation is a cornerstone of immersive voice automation—especially in flight simulators, space sims (*Star Citizen*, *Elite Dangerous*), tactical shooters (*Hell Let Loose*, *Arma*), and hands-free desktop workflows. When a user speaks a command (e.g. *"Call airstrike"* or *"Deploy landing gear"*), receiving an instant, context-appropriate spoken confirmation (e.g. *"Airstrike inbound on target coordinates"* in a futuristic cockpit AI or deep mech computer voice) provides immediate assurance without requiring the user to look away from their screen.

This specification details the end-to-end architecture for Voquill's multi-modal audio feedback system, combining **pre-generated offline neural Text-to-Speech (TTS)**, **custom audio file import**, **in-modal microphone clip recording**, and **instant zero-latency runtime playback**.

---

## 2. Core Architectural Pillars

### 2.1 Pre-Generation at Save Time (Zero Runtime Latency)
- **Problem:** Running neural TTS inference on CPU/GPU at the exact moment a macro triggers during a 144 FPS game causes micro-stutters and adds 100–300ms audio delay.
- **Solution:** When a user configures or edits a macro and clicks **Save Macro**, the audio clip is synthesized or normalized once and cached on disk as `~/.config/voquill-app/sounds/macros/<macro_id>.wav`.
- **Runtime Execution:** When the voice macro triggers during gameplay or dictation, playback latency is **<2 milliseconds** via Voquill's existing `cpal` low-latency audio engine. Zero CPU/GPU spikes mid-game.

### 2.2 The 5-Way Audio Feedback Selector
Each individual macro can be assigned its own feedback personality:
1. **Default Chirp:** The classic short confirmation tone.
2. **Local AI Voice (TTS):** Type any confirmation phrase, select a voice persona (*Cyberpunk EVA*, *Titan Mech*, *Tactical Radio*, *Nova Studio*), test preview, and save.
3. **Custom Audio File:** Select any `.mp3`, `.wav`, `.ogg`, `.flac`, or `.m4a` file. Voquill decodes via `symphonia`, normalizes to 44.1kHz WAV, and caches it.
4. **Microphone Recording:** Record a quick custom voice line directly inside the macro editor.
5. **Mute / Silent:** Macro executes completely silently.

---

## 3. Storage Hierarchy & Data Model

### 3.1 Directory Structure
All macro audio artifacts reside inside Voquill's unified storage root (`paths.rs`):
```
~/.config/voquill-app/
├── config.json
├── history.db
├── models/
│   ├── transcription/
│   ├── post-process/
│   └── tts/
│       ├── vits-piper-en_US-amy-low/
│       ├── vits-piper-en_US-ryan-low/
│       └── kokoro-en-v0_19/
└── sounds/
    └── macros/
        ├── <macro_id_1>.wav
        ├── <macro_id_2>.wav
        └── <macro_id_3>.wav
```

### 3.2 Rust Configuration Schema (`src-tauri/src/config.rs`)
```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "snake_case")]
pub enum MacroSoundMode {
    #[default]
    Default,
    None,
    Tts,
    CustomFile,
    MicRecording,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VoiceMacroCommand {
    pub id: String,
    pub phrase: String,
    #[serde(default)]
    pub phrases: Vec<String>,
    #[serde(default)]
    pub steps: Vec<MacroStep>,
    #[serde(default)]
    pub key_combination: Option<String>,
    #[serde(default)]
    pub hold_ms: Option<u64>,
    #[serde(default)]
    pub delay_after_ms: Option<u64>,

    // Audio Feedback Configuration
    #[serde(default)]
    pub sound_mode: MacroSoundMode,
    #[serde(default)]
    pub sound_tts_text: Option<String>,
    #[serde(default)]
    pub sound_tts_voice: Option<String>,
    #[serde(default)]
    pub sound_tts_speed: Option<f32>,
}
```

---

## 4. Local Neural TTS Engine (`python-runner/`)

### 4.1 Sherpa-ONNX & Piper Offline Synthesis
Voquill's `python-runner` already integrates `sherpa-onnx`. We leverage `sherpa_onnx.OfflineTts` for fast, lightweight, CPU-efficient offline voice synthesis.

#### Supported Voice Persona Catalog:
| Voice ID | Engine | Name / Persona | Style / Tone | Model Size |
| :--- | :--- | :--- | :--- | :--- |
| `piper-en_US-amy-low` | Piper/VITS | **Cyberpunk EVA** | Crisp, futuristic female ship computer | ~18MB |
| `piper-en_US-ryan-low` | Piper/VITS | **Titan Mech** | Deep, authoritative male cockpit computer | ~18MB |
| `piper-en_GB-southern_english_female-low` | Piper/VITS | **Aero Cockpit** | British ATC / flight deck AI | ~18MB |
| `piper-en_US-arctic-low` | Piper/VITS | **Tactical Radio** | Robotic, radio-filtered military comms | ~16MB |
| `kokoro-en-v0_19` | Kokoro | **Nova Studio** | Human-grade expressive neural voice | ~82MB |

### 4.2 Python Runner Endpoints
1. `GET /tts/voices` -> Returns catalog of available voice presets and their download/readiness status.
2. `POST /tts/synthesize` ->
   ```json
   {
     "text": "Airstrike inbound on target coordinates.",
     "voice_id": "piper-en_US-amy-low",
     "speed": 1.0,
     "output_path": "/path/to/sounds/macros/temp_preview.wav"
   }
   ```
   Returns:
   ```json
   {
     "output_path": "/path/to/sounds/macros/temp_preview.wav",
     "duration_secs": 1.85,
     "sample_rate": 22050,
     "provider": "sherpa-onnx"
   }
   ```

---

## 5. Rust Backend API & Tauri Commands

| Tauri Command | Purpose | Input | Output |
| :--- | :--- | :--- | :--- |
| `get_available_tts_voices` | Query voice persona catalog | None | `Vec<TtsVoiceInfo>` |
| `preview_tts_voice` | Synthesize and instantly play preview audio | `text: String, voice_id: String, speed: f32` | `Result<TtsPreviewResult, String>` |
| `save_macro_tts_audio` | Synthesize and commit `.wav` to macro storage | `macro_id: String, text: String, voice_id: String, speed: f32` | `Result<String, String>` |
| `import_macro_audio_file` | Decode external audio file, normalize to WAV, save | `macro_id: String, source_path: String` | `Result<String, String>` |
| `save_macro_mic_recording` | Save in-modal recorded microphone buffer | `macro_id: String, samples: Vec<f32>, sample_rate: u32` | `Result<String, String>` |
| `play_macro_sound_preview` | Play the currently configured sound for a macro | `macro_id: String` | `Result<(), String>` |
| `delete_macro_sound` | Remove macro audio file when reset to default/none | `macro_id: String` | `Result<(), String>` |

---

## 6. Frontend UI / UX Architecture

### 6.1 Macro Editor Modal Integration (`MacroSoundStep.tsx`)
In `MacroEditorModal.tsx`, as Step 3 of the editor stepper tab flow, a dedicated **Audio Feedback** step provides seamless switching:

- **Mode Tabs:** `[ 🔔 Default ] [ 🗣️ AI Voice ] [ 📁 File ] [ 🎙️ Mic ] [ 🔇 Mute ]`
- **AI Voice Mode View:**
  - Voice Persona dropdown with persona badges and categories.
  - Spoken text input field with placeholder examples (*"Deploying countermeasures"*, *"Landing gear down"*).
  - Playback speed slider (0.75x – 1.25x).
  - `[ ▶️ Test Voice ]` button with live waveform / loading state.
- **Audio File Mode View:**
  - `[ 📁 Browse File... ]` button triggering native file picker.
  - File details chip (duration, sample rate, filename).
  - `[ ▶️ Play Clip ]` button.
- **Mic Recording Mode View:**
  - `[ 🎙️ Record Clip ]` / `[ ⏹️ Stop Recording ]` toggle with live RMS level meter.
  - `[ ▶️ Play Recording ]` button.

### 6.2 Macro Card Badge in Settings
Each macro in `VoiceMacrosSection.tsx` displays an audio badge:
- `🗣️ "Airstrike inbound"`
- `📁 custom_sfx.wav`
- `🎙️ voice_note.wav`
- `🔔 chirp`
- `🔇 silent`

---

## 7. Execution Runtime Data Flow

```
[User speaks: "Call airstrike"]
               │
               ▼
   STT Engine (Whisper/Sherpa)
               │ (matches macro: id="abc123")
               ▼
      execute_macro_steps()
        ├── 1. Spawn macro audio playback in background:
        │      • Check ~/.config/voquill-app/sounds/macros/abc123.wav
        │      • If found -> play_audio(samples, sample_rate, playback_device) (<2ms)
        │      • Else if sound_mode == Default -> play_macro_trigger_sound()
        └── 2. Execute keypresses / system commands concurrently
```

---

## 8. Safety, Performance & Fallback Guarantees

1. **Non-Blocking Playback:** Sound playback always spawns in a dedicated thread; audio playback never delays keypress execution.
2. **Missing File Fallback:** If a configured audio file is deleted or corrupted, execution safely falls back to the default chirp and logs a clean diagnostic warning.
3. **Format Normalization:** All imported files are converted through `symphonia` + `hound` into standard 16-bit 44.1kHz PCM WAV files, eliminating runtime format incompatibilities.
4. **Offline Isolation:** All TTS models operate 100% offline with zero outbound network calls during synthesis.
