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

### 6. Platform Adaptation Pattern
When implementing platform-sensitive features, follow this structure:
1. **Platform Boundary First:** Keep OS/display boundaries (`linux/wayland`, `linux/x11`, `windows`) as top-level separations.
2. **Provider Layer Second:** Within a platform, isolate backend/provider behavior (e.g., portal capabilities and session handling).
3. **Quirks Last:** Only add DE/provider-specific quirk modules when a real incompatibility is confirmed and cannot be solved generically.

This pattern keeps the codebase clean as new distros, compositor versions, or portal changes appear.

### 7. Architecture-First, No Band-Aids
If implementing a feature or fix requires an architectural change, the architectural change must be made first. Do not implement features in a way that circumvents the current architecture because it is easier or faster. A correct feature on a correct architecture is the only acceptable outcome. If a full-stack refactor is required, do the full-stack refactor. This is non-negotiable. A feature jammed in with a shim or workaround is not a feature -- it is technical debt that will need to be undone later at greater cost.

---

## Pre-Submit Verification (Mandatory)

Before marking any task as complete, run each check as a separate command (never chained with `&&` -- if one silently fails, the chain hides it):

1. `cargo fmt` (Formatting)
2. `cargo check` (Compilation check -- zero warnings required)
3. `cargo clippy` (Static analysis -- zero warnings required)
4. `npm run lint` (Frontend lint -- pre-existing warnings are acceptable, new code must not introduce additional warnings)
5. `npm run typecheck` (TypeScript integrity)

All five must pass without warnings. Treat compiler warnings as errors.

---

## Architecture & Patterns

### 1. Backend (Rust)
- **Async Flow:** Use `tokio` or `tauri::async_runtime` for all I/O, network, and audio operations. Never block the main thread.
- **Error Handling:** Use `anyhow` for internal propagation to maintain context.
- **Command Safety:** Return `Result<T, String>` for all `#[tauri::command]` functions. The error string is what the frontend `Promise.reject` receives.
- **State Management:** Use `AppState` (managed by Tauri) to hold shared resources like `Config`, `AudioStream`, or `RecordingState`.
- **Dictation Session Lifecycle:** `SessionState` (`Idle` / `Recording` / `Transcribing` / `Typing`) in `AppState` is the authoritative session guard; a new session may only start from `Idle`. Each session carries a cancel token in `AppState.active_session` so a cancelled pipeline can never clobber a newer session's state or status.
- **Hotkey Gestures:** All press/release semantics (hold-to-talk, toggle mode, press-to-cancel) live in `app/hotkey_handler.rs`. Platform backends (the Wayland portal loop, the X11/Windows plugin handler in `main.rs`) only feed it events; never implement gesture logic inside a backend.
- **Modularity:** Keep hardware-specific logic isolated in modules (e.g., `audio.rs`, `typing.rs`, `hotkey.rs`).
- **Audio File Decoding:** Imported audio files (m4a/mp3/flac/ogg/wav) are decoded with `symphonia` (pure Rust, no external binaries) in `audio/decode.rs`; `audio/conversion.rs` owns normalization to 16kHz mono WAV for whisper. WAV inputs keep the original hound fast path.

### 2. Frontend (Preact, lives in `src/`)
- **Strict TypeScript:** No `any`. Explicit interfaces for all data structures (API responses, State slices).
- **Domain Hooks (Required):** State, effects, and handlers for a single domain must be co-located in a dedicated hook file (e.g., `useConfig.ts`, `useAudioSetup.ts`). Do not scatter related state across multiple hooks or leave it in the parent component. A hook should own its state, its effects, and its Tauri `invoke` calls. The parent component only wires hooks together and renders.
- **Signals-Only State Management (Strict Prohibition on useState):** `useState` is FORBIDDEN. All reactive state MUST use `useSignal` or `signal` from `@preact/signals`. `useCallback` is FORBIDDEN -- signals make it unnecessary. `useMemo` MUST be replaced with `useComputed`. `useEffect` MUST use `useSignalEffect` when reacting to state changes. Mount-only effects (event listeners, one-time probes) may use `useEffect` with an empty dependency array. `useRef` is FORBIDDEN for mutable state values -- use `useSignal` instead. `useRef` is ONLY permitted for DOM element references. Any new code introducing `useState`, `useCallback`, `useMemo`, or `useRef` for state will be rejected.
- **Styles (Current Convention):** Prefer component-local inline style objects with design tokens for layout, spacing, and color. Use global CSS (`index.css`) for resets, root-level variables, and truly global concerns only.
- **Style Consistency:** When touching existing UI, follow the style approach already used in that component/file. Do not introduce a separate styling pattern unless there is a clear architectural reason.
- **Tauri Core:** Use `@tauri-apps/api` for communication with the backend.

---

## Platform Compatibility & Requirements

| Platform | Display Server | Audio Backend | Hardware Access |
| :--- | :--- | :--- | :--- |
| **Linux** | Wayland, X11 | ALSA / PulseAudio | Wayland: XDG Portals (`ashpd`), X11: native X11 backends |
| **Windows** | Desktop | WASAPI | CoreAudio API |

### Linux Permission Setup
On Wayland, Voquill triggers standard XDG Portal prompts for microphone, global shortcuts, and remote desktop (input simulation). On X11, equivalent capabilities use native X11 backends and should still surface clear setup/readiness state in the UI.

---

## Development Workflow for New Features

When adding a new feature, follow this sequence:
1.  **Analyze Environment:** Check for platform-specific constraints (Wayland and X11 where relevant).
2.  **Scaffold Backend:** Implement the logic in a new or existing Rust module.
3.  **Expose Command:** Create a `#[tauri::command]` and register it in `main.rs`.
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

## Interaction Guidelines for Agents

- **Look for Improvement:** Don't just implement the request. Analyze the surrounding code for "mess" and offer to tidy it up.
- **Correct Inaccuracies Proactively:** If a user statement is technically incorrect or based on a false assumption, explicitly correct it and proceed with the correct approach. Do not silently follow an incorrect premise.
- **Ask, Don't Assume:** If a cleanup involves structural changes (like moving folders or renaming modules), always explain *why* it's cleaner and ask for approval.
- **Trace the Data:** Before proposing a fix for any data-related issue, trace the information back to its origin. Propose a fix for the source logic rather than a filter for the consumer.
- **Status Updates:** Use the centralized `emit_status_update` in Rust as the single source of truth for UI state. Avoid emitting ad-hoc events for standard states.
- **Platform Parity:** When adding a feature, ensure it is considered for Windows and Linux (Wayland and X11). If a platform requires specific logic, isolate it in a platform-specific module.
- **UI Consistency First:** Keep the UI behavior, structure, and interaction flow identical across systems whenever possible. Only diverge at the exact point where an OS/backend capability requires it (for example, system-managed shortcut configuration vs in-app configuration).
- **Documentation:** Proactively update `AGENTS.md` or other docs if you introduce a new architectural pattern or a major dependency.
- **Self-Verification:** Always run `cargo check` and `npm run typecheck` before declaring a task complete.
- **Git Commits:** Do not perform git commits without explicit user approval. Always ask for confirmation before running `git commit`.

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