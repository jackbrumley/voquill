/// Readiness is decided by the backend (`app::readiness`); these types
/// mirror its `ReadinessStatus`, and `useAppReadiness` loads and follows it.

/// What an empty API key field is saved as (matches `API_KEY_PLACEHOLDER`
/// in `app/readiness.rs`).
export const API_KEY_PLACEHOLDER = 'your_api_key_here';

export type AudioDeviceIssue = 'none' | 'no-devices' | 'device-missing';

export interface ReadinessStatus {
  isPermissionsReady: boolean;
  audioDeviceIssue: AudioDeviceIssue;
  isAudioDeviceReady: boolean;
  isTranscriptionReady: boolean;
  isHotkeyReady: boolean;
  isAllReady: boolean;
  postProcessModelMissing: boolean;
  /** Why the app is not ready, in plain words; empty when it is. */
  reasons: string[];
}

/// Shown until the backend's first verdict arrives. Routing waits for that
/// verdict, so this placeholder never decides anything.
export const READINESS_NOT_YET_CHECKED: ReadinessStatus = {
  isPermissionsReady: false,
  audioDeviceIssue: 'none',
  isAudioDeviceReady: false,
  isTranscriptionReady: false,
  isHotkeyReady: false,
  isAllReady: false,
  postProcessModelMissing: false,
  reasons: ['readiness has not been checked yet'],
};

/// One line for logs explaining the verdict.
export function describeReadiness(status: ReadinessStatus): string {
  return status.isAllReady ? 'Ready' : `Not ready: [${status.reasons.join('; ')}]`;
}
