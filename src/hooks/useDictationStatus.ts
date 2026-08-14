import { useSignal } from '@preact/signals';
import type { DictationStatus, StatusUpdatePayload } from '../types.ts';
import { isDictationStatus, normalizeStatusUpdate } from '../status.ts';

interface UseDictationStatusOptions {
  accept?: readonly DictationStatus[];
}

export function useDictationStatus(options?: UseDictationStatusOptions) {
  const status = useSignal<DictationStatus>('Ready');
  const lastSeq = useSignal(0);
  const accept = options?.accept;

  const handleStatusUpdate = (payload: string | StatusUpdatePayload): DictationStatus | null => {
    const normalized = normalizeStatusUpdate(payload);
    const nextSeq = normalized.seq ?? lastSeq.value + 1;

    if (nextSeq < lastSeq.value) {
      return null;
    }
    if (!isDictationStatus(normalized.status)) {
      console.warn(`[status] Ignoring unknown dictation status: ${normalized.status}`);
      return null;
    }
    lastSeq.value = nextSeq;
    if (accept && !accept.includes(normalized.status)) {
      return null;
    }
    status.value = normalized.status;
    return normalized.status;
  };

  return { status, handleStatusUpdate };
}
