import type { DictationStatus, StatusUpdatePayload } from './types.ts';

export const DICTATION_STATUSES: readonly DictationStatus[] = [
  'Ready',
  'Recording',
  'WakingUp',
  'Transcribing',
  'Processing',
  'Typing',
  'Error',
];

export interface NormalizedStatusUpdate {
  seq: number | null;
  status: string;
}

export function isDictationStatus(status: string): status is DictationStatus {
  return (DICTATION_STATUSES as readonly string[]).includes(status);
}

export function normalizeStatusUpdate(payload: string | StatusUpdatePayload): NormalizedStatusUpdate {
  return typeof payload === 'string' ? { seq: null, status: payload } : payload;
}

export function statusLabel(status: DictationStatus): string {
  switch (status) {
    case 'Error':
      return 'Mic not found';
    case 'WakingUp':
      return 'Waking up…';
    default:
      return status;
  }
}

/** Statuses where Voquill is working on a finished recording. */
export function isBusyStatus(status: DictationStatus): boolean {
  return status === 'WakingUp' || status === 'Transcribing' || status === 'Processing' || status === 'Typing';
}
