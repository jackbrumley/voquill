import { useEffect } from 'preact/hooks';
import { useSignal } from '@preact/signals';
import { invoke } from '@tauri-apps/api/core';
import type { SetupRecommendation } from '../pages/setup/setupTypes.ts';

interface UseSetupRecommendationReturn {
  recommendation: SetupRecommendation | null;
  error: string | null;
}

/// Fetches the speech engine choices the backend recommends for this
/// computer's hardware. The backend owns the rule; this only loads it.
export function useSetupRecommendation(): UseSetupRecommendationReturn {
  const recommendation = useSignal<SetupRecommendation | null>(null);
  const error = useSignal<string | null>(null);

  useEffect(() => {
    invoke<SetupRecommendation>('get_setup_recommendation')
      .then((result) => {
        recommendation.value = result;
      })
      .catch((failure: unknown) => {
        error.value = String(failure);
        console.error('Failed to load setup recommendation:', failure);
      });
  }, []);

  return { recommendation: recommendation.value, error: error.value };
}
