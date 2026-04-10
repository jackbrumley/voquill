# Voquill Agent Manifesto & Guidelines

This document serves as a constitution for all agentic coding entities (and humans) operating within the Voquill repository. Integrity, cleanliness, and architectural soundness are our primary metrics of success.

---

## 🏛️ The Voquill Philosophy

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

### 3. The "Wayland-Only" Mandate
Linux support is strictly Wayland.
- **XDG Portals:** Hardware access (Microphone, Shortcuts, Input Emulation) must use **XDG Portals** (via `ashpd`).
- **Compositor Awareness:** Recognize that Wayland compositors (GNOME, KDE, Hyprland) have strict security models; handle window positioning and input simulation using proper, future-proof protocols.
- **Primary Delivery:** The Linux AppImage is our gold standard for delivery. Keep packaging logic simple, portable, and self-contained.

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

---

## 🛠️ Essential Commands

### Project-wide (Root)
Managed via **npm** scripts and the Tauri CLI.
- **Dependency Check:** `npm run deps:check`
  - Verifies required system dependencies and prints install commands when missing.
- **Dev:** `npm run tauri:dev`
  - Runs dependency checks and starts the Tauri development server.
- **Build:** `npm run tauri:build`
  - Runs dependency checks, builds the frontend, and packages the app.
- **Tauri CLI:** `npm run tauri -- <command>`
  - Use for tauri-specific tasks like `tauri icon` or `tauri info`.

### Backend (src/)
- **Lint:** `cargo clippy` (Static analysis) and `cargo fmt` (Formatting).
- **Check:** `cargo check` (Fast compilation check).
- **Test:** `cargo test` (Run all tests).
- **Single Test:** `cargo test -- <name>` (Execute a specific test function).
- **Doc:** `cargo doc --open` (Generate and view crate documentation).

### Frontend (src/ui/)
- **Type Check:** `npm run typecheck`
  - Essential for verifying TypeScript integrity.
- **Lint:** `npm run lint`
  - Uses ESLint to enforce project styling rules.
- **Dev Server:** `npm run dev`
  - Starts the Vite dev server for UI-only iteration.
- **Preview:** `npm run preview`
  - Previews the production build of the UI.

---

## 🏗️ Architecture & Patterns

### 1. Backend (Rust)
- **Async Flow:** Use `tokio` or `tauri::async_runtime` for all I/O, network, and audio operations. Never block the main thread.
- **Error Handling:** Use `anyhow` for internal propagation to maintain context.
- **Command Safety:** Return `Result<T, String>` for all `#[tauri::command]` functions. The error string is what the frontend `Promise.reject` receives.
- **State Management:** Use `AppState` (managed by Tauri) to hold shared resources like `Config`, `AudioStream`, or `RecordingState`.
- **Modularity:** Keep hardware-specific logic isolated in modules (e.g., `audio.rs`, `typing.rs`, `hotkey.rs`).

### 2. Frontend (Preact)
- **Strict TypeScript:** No `any`. Explicit interfaces for all data structures (API responses, State slices).
- **Hooks over Classes:** Use functional components and custom hooks (in `src/ui/src/hooks/`) for logic isolation.
- **Styles:** Use semantic CSS. Avoid inline styles or "magic number" positioning. Themes should be managed via global CSS variables in `index.css`.
- **Tauri Core:** Use `@tauri-apps/api` for communication with the backend.

---

## 📋 Platform Compatibility & Requirements

| Platform | Display Server | Audio Backend | Hardware Access |
| :--- | :--- | :--- | :--- |
| **Linux** | Wayland (Primary) | ALSA / PulseAudio | XDG Portals (ashpd) |
| **Windows** | Desktop | WASAPI | CoreAudio API |

### Linux Permission Setup
On first launch or if permissions are missing, Voquill triggers standard XDG Portal prompts to request access to the microphone, global shortcuts, and remote desktop (for input simulation). Agents must ensure any new hardware interaction respects this Portal-first flow.

---

## 🔄 Development Workflow for New Features

When adding a new feature, follow this sequence:
1.  **Analyze Environment:** Check for platform-specific constraints (especially Wayland).
2.  **Scaffold Backend:** Implement the logic in a new or existing Rust module.
3.  **Expose Command:** Create a `#[tauri::command]` and register it in `main.rs`.
4.  **Implement UI:** Create the Preact component and hook it up to the command using `invoke`.
5.  **Verify Integrity:** Run `cargo clippy`, `npm run typecheck`, and `npm run lint`.
6.  **Test Platform Parity:** Verify the feature works on Linux (Wayland) and Windows.

---

## 🚧 High-Priority Architectural Fixes (Current Debt)

Any agent working on this repo should prioritize the following cleanups:
1.  **Redundant Nesting:** The `src/src` structure is messy and redundant. We aim to flatten this into a logical `/backend` and `/frontend` structure while keeping the Tauri root clean.
2.  **NPM/Cargo Synergy:** Keep frontend and Tauri script orchestration in npm, and Rust build logic in Cargo/Tauri.
3.  **Local Whisper Integration:** Follow the roadmap in `src/LOCAL_WHISPER_INTEGRATION_PLAN.md` if working on transcription features. Ensure model management is clean and asynchronous.

---

## 🤖 Interaction Guidelines for Agents
- **Look for Improvement:** Don't just implement the request. Analyze the surrounding code for "mess" and offer to tidy it up.
- **Ask, Don't Assume:** If a cleanup involves structural changes (like moving folders or renaming modules), always explain *why* it's cleaner and ask for approval.
- **Trace the Data:** Before proposing a fix for any data-related issue, trace the information back to its origin. Propose a fix for the source logic rather than a filter for the consumer.
- **Status Updates:** Use the centralized `emit_status_update` in Rust as the single source of truth for UI state. Avoid emitting ad-hoc events for standard states.
- **Platform Parity:** When adding a feature, ensure it is considered for Windows and Linux (Wayland). If a platform requires specific logic, isolate it in a platform-specific module.
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

---

## ⚠️ Common Pitfalls to Avoid
- **Blocking the UI:** Never run expensive calculations or blocking I/O on the main thread.
- **Hardcoding Paths:** Always use the Tauri `PathResolver` or standard `dirs` crate to locate configuration and data directories.
- **Silent Failures:** Always log errors and, if relevant, notify the user via a Toast or Status update.
- **Inconsistent Naming:** Do not mix `camelCase` and `snake_case` in the same context. Follow the established patterns (Rust: `snake_case`, TS: `camelCase`).
- **Over-Engineering:** Prefer simple, readable code over complex "clever" solutions. If a function is hard to explain, it needs to be simplified.
- **Ignoring Warnings:** Treat compiler warnings as errors. Clean code means zero warnings.

---
*Voquill: Clean code is a requirement, not a feature.*
