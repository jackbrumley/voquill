import type { Config } from './types.ts';

declare global {
  interface Window {
    /** Set before any script runs by the backend (`app::default_config_script`). */
    __VOQUILL_DEFAULT_CONFIG__?: Config;
  }
}

/// The backend's default settings (`Config::default()` in Rust), the only
/// source of default values. Used as the placeholder until the saved
/// settings load; it is never persisted.
export function backendDefaultConfig(): Config {
  const defaults = window.__VOQUILL_DEFAULT_CONFIG__;
  if (!defaults) {
    throw new Error('Default settings were not provided by the backend (window.__VOQUILL_DEFAULT_CONFIG__ is missing).');
  }
  return { ...defaults };
}
