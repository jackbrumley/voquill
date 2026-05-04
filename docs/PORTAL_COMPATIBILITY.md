# Wayland Portal Compatibility Matrix

This document tracks expected behavior for Wayland portal integrations, with emphasis on `org.freedesktop.portal.GlobalShortcuts`.

## Scope

- Platform: Linux Wayland sessions (portal path). X11 uses native backends and is tracked outside this document.
- Features: Global shortcuts, input emulation, microphone access
- Backends: GNOME, KDE (others may work if they implement the required portal interfaces)

## Current Baseline

| Environment | Portal Backend | GlobalShortcuts Version | Expected Flow | Notes |
| --- | --- | --- | --- | --- |
| Fedora GNOME 49 | `xdg-desktop-portal-gnome` | 1 | Bind/List | No `ConfigureShortcuts`; use explicit bind flow |
| KDE Plasma 6 (Wayland) | `xdg-desktop-portal-kde` | 1+ (varies) | Bind/List or Configure when available | Version-dependent behavior |

## Runtime Rules

1. Detect portal capabilities at runtime.
2. Choose flow by capability, not distro name.
3. Keep one active shortcut session owner and close old sessions on replacement.
4. Surface actionable errors to logs and UI.

## Verification Checklist

- `get_portal_diagnostics` returns `available=true` on supported Wayland systems.
- `get_linux_setup_status` reports `shortcuts=true` only when `record` is actually bound.
- Initial startup reuses existing shortcut binding when present.
- Manual bind path triggers portal request and persists trigger description.
- Press/release emits recording start/stop transitions.

## Fedora GNOME release-order edge case (GlobalShortcuts)

### Observed behavior

- Environment: Fedora GNOME Wayland with `xdg-desktop-portal-gnome`
- Shortcut: `Ctrl+Shift+Space` push-to-talk hold flow
- Failure pattern: portal emits repeated `Activated` events (roughly every 30ms) and does not emit a matching `Deactivated` for that shortcut cycle when keys are released in a problematic order
- Recovery pattern: a later press/release often emits `Activated` then `Deactivated`, which finally unlatches recording

### Rationale for Voquill fallback

- Push-to-talk must never remain latched after keys are physically released
- We keep the native portal as the primary source of truth (`Activated` starts, `Deactivated` stops)
- We add a defensive repeat-heartbeat fallback only for this edge case class:
  - detect rapid repeated `Activated` cadence while already recording
  - treat the cadence as a temporary heartbeat mode
  - if heartbeat stops for a short silence window and recording is still active, force `stop_recording`

### Current thresholds

- `REPEAT_ACTIVATION_WINDOW_MS = 120`
- `REPEAT_SILENCE_TIMEOUT_MS = 220`
- `REPEAT_WATCHDOG_TICK_MS = 50`

These values are intentionally conservative and should be tuned only with portal event logs from affected systems.

## Troubleshooting Signals

- `status=unavailable`: Portal service/backend missing or inaccessible.
- `status=unsupported`: Wayland not available or portal version not usable.
- `status=error`: Portal call failed (inspect `detail` for root cause).
- `shortcuts=false` with `status=ready`: Portal available but no `record` binding yet.
