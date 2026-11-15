import { useSignal } from '@preact/signals';
import type { ReadinessStatus } from '../readiness.ts';
import type { SetupStepState } from '../pages/setup/SetupStep.tsx';
import type { SetupStepId } from '../pages/setup/setupTypes.ts';

const STEP_ORDER: readonly SetupStepId[] = ['permissions', 'microphone', 'speech', 'try'];

interface UseSetupStepsReturn {
  stateOf: (step: SetupStepId) => SetupStepState;
  isOpen: (step: SetupStepId) => boolean;
  toggle: (step: SetupStepId) => void;
}

/// Turns the readiness gate's verdict into setup's step sequence. A step is
/// done when its readiness checks pass; the first unfinished step is open.
/// The user may reopen a finished step to change it; `initiallyOpen` opens a
/// finished step from the start (the speech engine, when reviewing new
/// options). Readiness itself is decided by the backend (`app::readiness`); this hook
/// only presents it.
export function useSetupSteps(readiness: ReadinessStatus, initiallyOpen: SetupStepId | null): UseSetupStepsReturn {
  const reopenedStep = useSignal<SetupStepId | null>(initiallyOpen);

  const isDone = (step: SetupStepId): boolean => {
    switch (step) {
      case 'permissions':
        return readiness.isPermissionsReady && readiness.isHotkeyReady;
      case 'microphone':
        return readiness.isAudioDeviceReady;
      case 'speech':
        return readiness.isTranscriptionReady;
      case 'try':
        return false;
    }
  };

  const currentStep = STEP_ORDER.find((step) => !isDone(step)) ?? 'try';

  const stateOf = (step: SetupStepId): SetupStepState => {
    if (step === currentStep) {
      return 'current';
    }
    return isDone(step) ? 'done' : 'upcoming';
  };

  const isOpen = (step: SetupStepId): boolean => {
    if (reopenedStep.value !== null && isDone(reopenedStep.value)) {
      return step === reopenedStep.value;
    }
    return step === currentStep;
  };

  const toggle = (step: SetupStepId) => {
    reopenedStep.value = reopenedStep.value === step ? null : step;
  };

  return { stateOf, isOpen, toggle };
}
