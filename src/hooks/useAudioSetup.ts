import { useSignal } from '@preact/signals';
import { invoke } from '@tauri-apps/api/core';
import type { AudioDevice, LinuxPermissions, HotkeyBindingState } from '../types.ts';

interface UseAudioSetupReturn {
  permissions: LinuxPermissions | null;
  availableMics: AudioDevice[];
  hotkeyError: string | null;
  micTestStatus: 'idle' | 'recording' | 'playing' | 'processing';
  micVolume: number;
  micTestPassed: boolean;
  hasLoadedSetupStatus: boolean;
  hasLoadedMics: boolean;
  loadMics: () => Promise<void>;
  startMicTest: () => Promise<void>;
  stopMicTest: () => Promise<void>;
  stopMicPlayback: () => Promise<void>;
  handleAudioSetup: () => Promise<void>;
  handleInputSetup: () => Promise<void>;
  checkSetupStatus: () => Promise<{ perms: LinuxPermissions; bindingState: HotkeyBindingState } | undefined>;
  setMicTestStatus: (status: 'idle' | 'recording' | 'playing' | 'processing') => void;
  setMicVolume: (volume: number) => void;
  setMicTestPassed: (passed: boolean) => void;
}

export function useAudioSetup(showToast: (message: string, type: 'success' | 'error' | 'info' | 'saved') => void): UseAudioSetupReturn {
  const permissions = useSignal<LinuxPermissions | null>(null);
  const availableMics = useSignal<AudioDevice[]>([]);
  const hotkeyError = useSignal<string | null>(null);
  const micTestStatus = useSignal<'idle' | 'recording' | 'playing' | 'processing'>('idle');
  const micVolume = useSignal<number>(0);
  const micTestPassed = useSignal(false);
  const hasLoadedSetupStatus = useSignal(false);
  const hasLoadedMics = useSignal(false);

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
    } catch (error) {
      showToast(`Failed to stop playback: ${error}`, 'error');
    }
  };

  return {
    permissions: permissions.value,
    availableMics: availableMics.value,
    hotkeyError: hotkeyError.value,
    micTestStatus: micTestStatus.value,
    micVolume: micVolume.value,
    micTestPassed: micTestPassed.value,
    hasLoadedSetupStatus: hasLoadedSetupStatus.value,
    hasLoadedMics: hasLoadedMics.value,
    loadMics,
    startMicTest,
    stopMicTest,
    stopMicPlayback,
    handleAudioSetup,
    handleInputSetup,
    checkSetupStatus,
    setMicTestStatus: (status) => { micTestStatus.value = status; },
    setMicVolume: (volume) => { micVolume.value = volume; },
    setMicTestPassed: (passed) => { micTestPassed.value = passed; },
  };
}