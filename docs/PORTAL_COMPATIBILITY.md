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

## Troubleshooting Signals

- `status=unavailable`: Portal service/backend missing or inaccessible.
- `status=unsupported`: Wayland not available or portal version not usable.
- `status=error`: Portal call failed (inspect `detail` for root cause).
- `shortcuts=false` with `status=ready`: Portal available but no `record` binding yet.
