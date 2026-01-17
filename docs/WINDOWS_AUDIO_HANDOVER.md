# Handover: Windows Audio Device Naming & Platform Parity

## 🎯 Objective
Achieve full feature parity between Windows and Linux while maintaining a clean, DRY, and high-integrity codebase. The primary focus is currently on Windows microphone enumeration to match the detail seen in Windows System Sound settings.

## 📜 Global Rules & Philosophy
- **Integrity Over Expediency**: Do not use "quick hacks." If a solution isn't architecturally sound, do not implement it.
- **Root Cause First**: Fix issues at the data source (e.g., the backend enumeration logic), never at the consumer level (e.g., UI filtering).
- **OCD-Standard Code**: Code must be semantic, descriptive, and idiomatic. No abbreviations (e.g., use `index` instead of `idx`).
- **Linux Integrity (CRITICAL)**: The application is "perfect" on Linux. **NEVER** make changes that break or degrade Linux functionality.
- **Platform Parity**: Features must operate identically across platforms. If Linux has "Hold-to-Talk" and "Typewriter Mode," Windows must have them too.
- **Wayland-Only Mandate**: Linux support is strictly for Wayland using XDG Portals (via `ashpd`).
- **Zero-Warning Build**: The project must compile with zero errors and zero warnings on all supported platforms.

## ✅ Functional Requirements (Windows)
- **Audio Device Labels**: Must follow the format `[Friendly Name] - [Device Description]` (e.g., "Microphone - 2- Realtek(R) Audio").
- **Hold-to-Talk**: Must detect both "Pressed" and "Released" events for the global hotkey to start/stop recording.
- **Hardware Typing**: Must simulate keystrokes into focused applications using the native Windows `SendInput` API.
- **Environment Verification**: The Deno check scripts (`dev.ts`, `build.ts`) must verify that Windows build tools (LLVM/Clang for `bindgen`, CMake for `-sys` crates) are installed and in the PATH.

## 🏗️ Current State
- **Compiles**: The app compiles and runs on Windows after several refactoring rounds.
- **Parity**: "Hold-to-Talk" and "Typewriter Mode" are implemented for Windows and integrated into the shared `AppState` and `typing.rs`.
- **The Failure**: The audio device list currently only shows generic labels (e.g., "Microphone"). Multiple attempts to query the Windows MMDevice Property Store have failed due to `windows` crate API mismatches.

## 🧪 Attempted Approaches & Blockers (Audio Naming)
1. **Basic CPAL Enumeration**: Only provides the generic endpoint name (e.g., "Microphone").
2. **MMDevice API (Direct)**: Attempted to query `PKEY_Device_FriendlyName` and `PKEY_Device_DeviceDesc`.
   - **Blocker 1**: `PROPVARIANT` is a complex union. Manual access is fragile and differs between crate versions.
   - **Blocker 2**: `PropVariantToStringAlloc` signature mismatches in `windows` v0.58.
   - **Blocker 3**: The current fallback in `audio.rs` reverts to generic CPAL names because the advanced extraction was causing build failures.

## 💡 Guidance for Next Session
- **Right Way Only**: Do not retreat to generic CPAL names. Find the correct `windows` crate v0.58 invocation for `PropVariantToStringAlloc`.
- **Gating Strategy**: Use type aliasing (e.g., `VirtualKeyboardHandle`) and abstraction modules to keep the code DRY while isolating platform-specific logic.
- **Deno Deps**: See `scripts/deps.ts` for the centralized "Smart Dependency" check logic that helps users fix their PATH/installation issues.
- **Final Target**: The UI dropdown should look like the Windows Sound control panel, showing both the device name and the card it belongs to.
