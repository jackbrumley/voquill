# Voquill Voice Macros & Natural Language Automation Engine Specification

## Status: Draft / Proposal for Discussion
**Date:** August 2026  
**Authors:** Voquill Core Team  
**Target Version:** Post-v1.5 Feature Proposal  

---

## 1. Executive Summary

Voice macro automation tools (most notably **VoiceAttack**) have long been essential for flight simulators, space sims (*Star Citizen*, *Elite Dangerous*), tactical shooters (*Hell Let Loose*, *Squad*, *Arma*), accessibility users, and desktop power users. However, existing solutions remain hindered by decades-old speech recognition engines, rigid grammar trees, high error rates, and platform lock-in (Windows only).

Voquill is uniquely positioned to redefine this category by combining:
1. **Modern Local Speech-to-Text (Whisper / Sherpa-ONNX):** Near-human transcription accuracy even with heavy game background noise, varied accents, and fast natural speech.
2. **Local LLM & Semantic Intent Matching:** Moving beyond rigid keyword trees. Users can speak naturally (*"Let's drop a guided bomb on that position"*) and have the local system resolve the semantic intent to the configured macro action (*"Call Guided Airstrike"*).
3. **Cross-Platform Parity:** Native support across Linux (Wayland via XDG Portals & X11 via XTest) and Windows.
4. **Zero Cloud Latency & Total Privacy:** Fully offline on-device processing.

---

## 2. Comparison: VoiceAttack vs. Voquill Voice Macros

| Capability | VoiceAttack | Voquill Voice Macros |
| :--- | :--- | :--- |
| **Speech Recognition Engine** | Windows Speech API (SAPI / legacy acoustic models). High misrecognition rate. | Modern Whisper (`whisper.cpp`) / Sherpa-ONNX (`SenseVoice` / `FastConformer`). State-of-the-art accuracy. |
| **Command Flexibility** | Rigid grammar syntax trees (`[command] [arg1] [arg2]`). Single-word mismatch causes failure. | **Natural conversational speech** matched via 3-tier semantic cascade (exact alias → embedding vector similarity → local LLM). |
| **Platform Support** | Windows only (requires proprietary runtime). | **Linux (Wayland & X11) and Windows** native. |
| **Gaming Resource Management** | High background polling; unoptimized speech model. | **CPU-isolated low-latency STT** ensuring zero GPU contention and 0 FPS drop in heavy games. |
| **UI & UX** | Complex, multi-nested 2010-era dialogs. | Modern Preact + Signals interface, visual drag-and-drop sequence builder. |
| **Licensing & Extensibility** | Proprietary closed source. | Open, self-contained architecture. |

---

## 3. Architecture & Data Flow

```
                      ┌────────────────────────────────────────┐
                      │             Audio Capture              │
                      │  (PTT Hotkey / Wake Word / Stream VAD) │
                      └───────────────────┬────────────────────┘
                                          │ Audio Buffer (16kHz Mono)
                                          ▼
                      ┌────────────────────────────────────────┐
                      │       Fast Local STT Engine            │
                      │   (Whisper Tiny/Base or Sherpa-ONNX)   │
                      │   *Dedicated CPU routing for games*    │
                      └───────────────────┬────────────────────┘
                                          │ Raw Transcript
                                          ▼
                      ┌────────────────────────────────────────┐
                      │    Semantic Intent Matcher (3 Tiers)   │
                      │ 1. Exact / Regex Alias Match  (<1ms)   │
                      │ 2. Vector Embedding Distance  (2-5ms)  │
                      │ 3. Local LLM Intent & Params  (~150ms) │
                      └───────────────────┬────────────────────┘
                                          │ Resolved Action + Params
                                          ▼
                      ┌────────────────────────────────────────┐
                      │         Macro Execution Engine         │
                      │  - Key Combos (Ctrl+Alt+1)             │
                      │  - Key Press / Hold / Release Delays   │
                      │  - Text Typing / Sound Feedback        │
                      └───────────────────┬────────────────────┘
                                          │
                  ┌───────────────────────┴───────────────────────┐
                  ▼                                               ▼
     ┌────────────────────────┐                     ┌────────────────────────┐
     │  OS Input Simulation   │                     │  Feedback / HUD Mode   │
     │ - Windows (SendInput)  │                     │ - Audio Cues (Chimes)  │
     │ - Linux Wayland (ashpd)│                     │ - Minimal HUD Badge    │
     │ - Linux X11 (xtest)    │                     │ - Silent Mode          │
     └────────────────────────┘                     └────────────────────────┘
```

---

## 4. Detailed Component Design

### 4.1 Listening Modes & Resource Optimization

Running intensive 3D games (e.g., *Hell Let Loose*, *Cyberpunk*, *MSFS*) consumes maximum GPU and VRAM. A continuous heavy Whisper or LLM model running on the GPU would cause frame drops. We address this through **three distinct listening modes** and **dedicated CPU execution**:

#### Listening Modes:
1. **Push-to-Command (PTT) [Default for Competitive Gaming]:**
   - User holds a dedicated macro key (e.g. Mouse Button 4, Foot Pedal, Caps Lock), speaks a command, and releases.
   - **0% CPU/GPU idle overhead.**
   - Audio is buffered only during the keypress and transcribed upon release (sub-100ms turnaround).
2. **Wake-Word Mode [For Flight/Space Simulators]:**
   - A lightweight micro-wake-word engine (e.g. `openWakeWord` or ONNX micro-model running on 1 CPU thread) listens continuously for a callsign (e.g., *"Computer"*, *"Voquill"*, or *"Overlord"*).
   - When triggered, it captures the following speech phrase and routes it to the STT matcher.
   - **< 0.5% CPU idle, 0 MB GPU VRAM.**
3. **Continuous Streaming VAD [For Desktop / Hands-Free]:**
   - Silero VAD monitors microphone energy; active speech chunks are sliced and evaluated.
   - Best suited for hands-free desktop tasks or casual navigation.

#### CPU-Isolated Fast Transcription:
- For voice macros, transcription is routed to **CPU-bound compact models** (e.g., `whisper.cpp tiny.en` / `base.en` or `sherpa-onnx` `SenseVoice-Small`).
- On modern multi-core CPUs, a 1.5-second command phrase processes in **~50–90ms on 2 CPU threads**.
- **Result:** Complete isolation from the game's GPU rendering pipeline.

---

### 4.2 Three-Tier Semantic Intent Matcher

Instead of requiring users to remember exact syntax (e.g. `Menu -> Fire Support -> Artillery`), the matcher uses a cascading strategy:

```
User says: "Let's call in an artillery strike on that squad"
  │
  ├─► Tier 1: Exact / Alias Regex (< 1ms, CPU)
  │    Checks direct phrases: ["artillery", "call artillery", "barrage", "bring the rain"]
  │    → Exact match found? Execute immediately.
  │
  ├─► Tier 2: Fast Vector Embeddings / Cosine Similarity (2–5ms, CPU)
  │    Compares transcript vector against pre-indexed Action Description vectors:
  │    • Action "Call Artillery": "request heavy artillery bombardment on target" (Similarity: 0.89)
  │    • Action "Deploy Smoke": "deploy defensive smoke screen" (Similarity: 0.22)
  │    → Similarity >= threshold (e.g. 0.78)? Execute immediately.
  │
  └─► Tier 3: Local LLM Intent & Parameter Extractor (Optional, ~150ms)
       Used for complex multi-variable actions or conditional logic:
       User: "Set engine throttle to eighty percent and gear down"
       LLM Output:
       [
         { "action_id": "set_throttle", "param": 80 },
         { "action_id": "landing_gear_down" }
       ]
```

#### Confidence Thresholds & Failure Safety:
- Every action has a configurable `confidence_threshold` (default `0.75`).
- If no action reaches the confidence threshold:
  - An optional low-pitch subtle audio chime indicates no match.
  - No accidental key presses are emitted into the game.

---

### 4.3 Macro Action & Step Model

A macro is a declarative sequence of atomic steps executed sequentially:

```json
{
  "id": "hll_artillery_smoke",
  "name": "Call Smoke Barrage",
  "description": "Request artillery smoke round on target location",
  "aliases": [
    "smoke barrage",
    "artillery smoke",
    "drop smoke on mark",
    "smoke screen"
  ],
  "confidence_threshold": 0.75,
  "steps": [
    { "type": "KeyDown", "key": "F2" },
    { "type": "Delay", "duration_ms": 60 },
    { "type": "KeyUp", "key": "F2" },
    { "type": "Delay", "duration_ms": 120 },
    { "type": "KeyCombo", "modifiers": ["Control"], "key": "Key3" },
    { "type": "PlayAudio", "file": "sounds/radio_click.wav" }
  ]
}
```

#### Supported Step Primitives:
1. `KeyCombo { modifiers: Vec<Modifier>, key: Key }` — e.g., `Ctrl + Shift + F5`
2. `KeyPress { key: Key, hold_ms: u64 }` — Press, hold for specified milliseconds, release.
3. `KeyDown { key: Key }` / `KeyUp { key: Key }` — Stateful holding (e.g., hold accelerator / thruster for 3 seconds).
4. `Delay { duration_ms: u64 }` — Inter-step pause (critical for game UI animation timings).
5. `TypeText { text: String }` — Hardware typing of full strings (e.g. game chat commands `/all gg`).
6. `PlayAudio { path: String }` — Local confirmation sounds (radio clicks, sci-fi chimes).
7. `RunCommand { command: String, args: Vec<String> }` — External hooks (OBS scene switch, Discord mute, launch app).
8. `SpeakTTS { text: String }` — Synthesized audio feedback (e.g., *"Landing gear down"*).

---

### 4.4 OS Input Emulation (Platform Parity)

In accordance with Voquill's architectural guidelines, macro execution extends the existing `InputSimulation` trait in `src-tauri/src/platform/traits.rs`:

```rust
#[async_trait]
pub trait InputSimulation: Send + Sync {
    async fn execute_macro_sequence(
        &self,
        app_handle: &AppHandle,
        steps: &[MacroStep],
    ) -> Result<(), String>;
    
    // Existing methods:
    async fn type_text_hardware(...);
    async fn send_paste_shortcut(...);
}
```

- **Windows:** Win32 `SendInput` using direct scancodes (`KEYEVENTF_SCANCODE`) to support DirectX / raw-input game engines.
- **Linux Wayland:** `ashpd` Remote Desktop portal keysym and keycode event emission.
- **Linux X11:** `x11rb` / `XTest` keycode press and release sequences.

---

### 4.5 Overlay & Gaming Ergonomics

Standard desktop overlays can disrupt exclusive full-screen gaming, cause FPS stutters, or obstruct critical HUD elements (minimap, crosshair, ammo counter).

Voquill Voice Macros introduces **Game-Ready Overlay Modes**:

1. **Audio-Only / Headless (Recommended for Gaming):**
   - Visual overlay window is completely hidden.
   - Subtle acoustic cues provide feedback:
     - *Start Chime:* Mic listening active.
     - *Confirmation Beep / Radio Chirp:* Command matched and macro executed.
     - *Soft Error Thud:* Unrecognized command / below confidence threshold.
2. **Minimalist Gaming HUD:**
   - Ultra-compact, borderless translucent badge (e.g., `120x24px`) docked to a user-selected corner (Top-Right, Bottom-Left, etc.).
   - Displays recognized action name and confidence score (e.g., `⚡ Smoke Barrage [92%]`), fading smoothly after 1.5 seconds.
3. **Standard Overlay:**
   - Existing Voquill bottom-center dictation pill.
4. **Disabled:**
   - Pure silence and no visual indicators.

---

### 4.6 Profiles & Auto-Switching

Users switch between games and productivity apps frequently.

- **Profile Structure:** Each profile encapsulates its own listening mode, PTT bindings, audio feedback preferences, and action list.
  - Examples: `Hell Let Loose.json`, `Star Citizen.json`, `Microsoft Flight Simulator.json`, `Blender 3D.json`.
- **Automatic Process Detection:**
  - An optional background focus watcher monitors the active window process name (e.g. `HLL-Win64-Shipping.exe`, `StarCitizen.exe`, `blender.exe`).
  - Automatically swaps active macro profiles when focus changes.
- **Export & Community Sharing:**
  - Profiles are serialized as standard JSON files, enabling effortless import, export, and community sharing.

---

## 5. Proposed Data Models & Schemas

### Rust Configuration Types (`src-tauri/src/config.rs`)

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MacroListeningMode {
    PushToTalk,
    WakeWord,
    ContinuousVAD,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum OverlayDisplayMode {
    Standard,
    MinimalHud,
    AudioOnly,
    Disabled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceMacroProfile {
    pub id: String,
    pub name: String,
    pub process_match: Option<String>,
    pub listening_mode: MacroListeningMode,
    pub ptt_hotkey: Option<String>,
    pub wake_word: Option<String>,
    pub overlay_mode: OverlayDisplayMode,
    pub sound_feedback_enabled: bool,
    pub actions: Vec<VoiceMacroAction>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceMacroAction {
    pub id: String,
    pub name: String,
    pub description: String,
    pub aliases: Vec<String>,
    pub confidence_threshold: f32, // Default: 0.75
    pub steps: Vec<MacroStep>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum MacroStep {
    KeyCombo { modifiers: Vec<String>, key: String },
    KeyPress { key: String, hold_ms: u64 },
    KeyDown { key: String },
    KeyUp { key: String },
    Delay { duration_ms: u64 },
    TypeText { text: String },
    PlayAudio { path: String },
    RunCommand { command: String, args: Vec<String> },
    SpeakTTS { text: String },
}
```

---

## 6. Phased Implementation Roadmap

```
┌─────────────────────────────────────────────────────────────────────────┐
│ Phase 1: Macro Execution & Input Simulation Engine                     │
│ • Extend InputSimulation trait for KeyPress, KeyDown, KeyUp, Delay     │
│ • Implement Windows SendInput scancodes, Wayland ashpd, X11 xtest       │
│ • Build Macro Executor unit & integration test suite                    │
└────────────────────────────────────┬────────────────────────────────────┘
                                     │
┌────────────────────────────────────▼────────────────────────────────────┐
│ Phase 2: Intent Matching & Fast STT Routing                            │
│ • Build Tier-1 Exact & Alias Matcher                                    │
│ • Build Tier-2 Fast Vector Embedding / Cosine Similarity Matcher        │
│ • Add Audio Cues (chime/beep sound generator)                           │
└────────────────────────────────────┬────────────────────────────────────┘
                                     │
┌────────────────────────────────────▼────────────────────────────────────┐
│ Phase 3: Frontend Macro Studio & Profile Manager                       │
│ • Visual Step Builder (Add Combo -> Add Delay -> Play Sound)           │
│ • Profile Manager with JSON Import/Export                               │
│ • Overlay Mode Selector (Full / Minimal HUD / Audio-Only)              │
└────────────────────────────────────┬────────────────────────────────────┘
                                     │
┌────────────────────────────────────▼────────────────────────────────────┐
│ Phase 4: Advanced Features & Refinement                                │
│ • Push-to-Command secondary hotkey handling                            │
│ • Process auto-detection / profile switching                            │
│ • Optional openWakeWord / micro-wake-word integration                   │
└─────────────────────────────────────────────────────────────────────────┘
```

---

## 7. Open Questions & Points for Team Discussion

1. **Embedding Engine for Tier 2 Matching:**
   - *Option A:* Fast local ONNX sentence transformer (e.g., `all-MiniLM-L6-v2` via `sherpa-onnx` or `fastembed-rs` ~20MB, sub-3ms cosine similarity).
   - *Option B:* Direct LLM prompt classification via existing `llama-server`.
   - *Recommendation:* Option A for real-time <5ms game macros; Option B as fallback for parameter-heavy voice commands.
2. **Keybind Capture in UI:**
   - How should users record keystroke sequences in the UI? (e.g., a "Record Macro" button that records real-time key presses and inter-key timings).
3. **Sound Assets:**
   - Should Voquill bundle standard royalty-free radio/sci-fi chimes (e.g., *start-listen*, *ack*, *error*) as embedded Tauri assets?
4. **Overlay Positioning during Games:**
   - In full-screen exclusive mode on Windows/Linux, borderless windows can sometimes lose top-most status depending on the graphics API. Testing across DirectX 11/12, Vulkan, and OpenGL games will be part of Phase 1 verification.

---

*This document is ready for review, refinement, and team feedback.*
