import { useSignal } from '@preact/signals';
import { invoke } from '@tauri-apps/api/core';
import type { AudioDevice, LinuxPermissions, HotkeyBindingState, MicVolumePayload } from '../types.ts';

interface UseAudioSetupReturn {
  permissions: LinuxPermissions | null;
  availableMics: AudioDevice[];
  availableSpeakers: AudioDevice[];
  hotkeyError: string | null;
  micTestStatus: 'idle' | 'recording' | 'playing' | 'processing';
  micVolume: number;
  isMicTriggered: boolean;
  micTestPassed: boolean;
  hasLoadedSetupStatus: boolean;
  hasLoadedMics: boolean;
  hasLoadedSpeakers: boolean;
  loadMics: () => Promise<void>;
  loadSpeakers: () => Promise<void>;
  startMicTest: () => Promise<void>;
  stopMicTest: () => Promise<void>;
  stopMicPlayback: () => Promise<void>;
  handleAudioSetup: () => Promise<void>;
  handleInputSetup: () => Promise<void>;
  checkSetupStatus: () => Promise<{ perms: LinuxPermissions; bindingState: HotkeyBindingState } | undefined>;
  setMicTestStatus: (status: 'idle' | 'recording' | 'playing' | 'processing') => void;
  setMicVolume: (payload: MicVolumePayload | number) => void;
  setMicTestPassed: (passed: boolean) => void;
}

export function useAudioSetup(showToast: (message: string, type: 'success' | 'error' | 'info' | 'saved') => void): UseAudioSetupReturn {
  const permissions = useSignal<LinuxPermissions | null>(null);
  const availableMics = useSignal<AudioDevice[]>([]);
  const availableSpeakers = useSignal<AudioDevice[]>([]);
  const hotkeyError = useSignal<string | null>(null);
  const micTestStatus = useSignal<'idle' | 'recording' | 'playing' | 'processing'>('idle');
  const micVolume = useSignal<number>(0);
  const isMicTriggered = useSignal<boolean>(false);
  const micTestPassed = useSignal(false);
  const hasLoadedSetupStatus = useSignal(false);
  const hasLoadedMics = useSignal(false);
  const hasLoadedSpeakers = useSignal(false);

  const checkSetupStatus = async () => {
    try {
      const perms = await invoke<LinuxPermissions>('get_linux_setup_status');
      permissions.value = perms;
      const bindingState = await invoke<HotkeyBindingState>('get_hotkey_binding_state');
      hotkeyError.value = await invoke<string | null>('check_hotkey_status');
      return { perms, bindingState };
    } catch (error) {
      console.error('Failed to check setup status:', error);
    } finally {
      hasLoadedSetupStatus.value = true;
    }
  };

  const loadMics = async () => {
    try {
      const devices = await invoke<AudioDevice[]>('get_audio_devices');
      availableMics.value = devices;
    } catch (error) {
      showToast(`Failed to load microphones: ${error}`, 'error');
    } finally {
      hasLoadedMics.value = true;
    }
  };

  const loadSpeakers = async () => {
    try {
      const devices = await invoke<AudioDevice[]>('get_output_devices');
      availableSpeakers.value = devices;
    } catch (error) {
      showToast(`Failed to load playback devices: ${error}`, 'error');
    } finally {
      hasLoadedSpeakers.value = true;
    }
  };

  const handleAudioSetup = async () => {
    try {
      await invoke('request_audio_permission');
      showToast('Audio permission granted!', 'success');
      await checkSetupStatus();
    } catch (error) {
      showToast(`Failed to get audio permission: ${error}`, 'error');
    }
  };

  const handleInputSetup = async () => {
    try {
      await invoke('request_input_permission');
      showToast('Input permission granted!', 'success');
      await checkSetupStatus();
    } catch (error) {
      showToast(`Failed to get input permission: ${error}`, 'error');
    }
  };

  const startMicTest = async () => {
    try {
      micTestStatus.value = 'recording';
      await invoke('start_mic_test');
    } catch (error) {
      micTestStatus.value = 'idle';
      showToast(`Failed to start mic test: ${error}`, 'error');
    }
  };

  const stopMicTest = async () => {
    micTestStatus.value = 'processing';
    isMicTriggered.value = false;
    micVolume.value = 0;
    try {
      await invoke('stop_mic_test');
    } catch (error) {
      micTestStatus.value = 'idle';
      showToast(`Failed to stop mic test: ${error}`, 'error');
    }
  };

  const stopMicPlayback = async () => {
    try {
      await invoke('stop_mic_playback');
      micTestStatus.value = 'idle';
      isMicTriggered.value = false;
      micVolume.value = 0;
    } catch (error) {
      showToast(`Failed to stop playback: ${error}`, 'error');
    }
  };

  return {
    permissions: permissions.value,
    availableMics: availableMics.value,
    availableSpeakers: availableSpeakers.value,
    hotkeyError: hotkeyError.value,
    micTestStatus: micTestStatus.value,
    micVolume: micVolume.value,
    isMicTriggered: isMicTriggered.value,
    micTestPassed: micTestPassed.value,
    hasLoadedSetupStatus: hasLoadedSetupStatus.value,
    hasLoadedMics: hasLoadedMics.value,
    hasLoadedSpeakers: hasLoadedSpeakers.value,
    loadMics,
    loadSpeakers,
    startMicTest,
    stopMicTest,
    stopMicPlayback,
    handleAudioSetup,
    handleInputSetup,
    checkSetupStatus,
    setMicTestStatus: (status) => {
      micTestStatus.value = status;
      if (status !== 'recording') {
        isMicTriggered.value = false;
        micVolume.value = 0;
      }
    },
    setMicVolume: (payload) => {
      if (typeof payload === 'number') {
        micVolume.value = payload;
        isMicTriggered.value = false;
      } else if (payload && typeof payload === 'object') {
        micVolume.value = payload.volume;
        isMicTriggered.value = payload.is_triggered;
      }
    },
    setMicTestPassed: (passed) => { micTestPassed.value = passed; },
  };
}
