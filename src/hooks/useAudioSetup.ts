import { useState, useCallback } from 'preact/hooks';
import { invoke } from '@tauri-apps/api/core';
import type { AudioDevice, LinuxPermissions, HotkeyBindingState } from '../types.ts';

interface UseAudioSetupReturn {
  permissions: LinuxPermissions | null;
  setPermissions: (perms: LinuxPermissions | null) => void;
  availableMics: AudioDevice[];
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
  setHasLoadedSetupStatus: (loaded: boolean) => void;
}

export function useAudioSetup(showToast: (message: string, type: 'success' | 'error' | 'info' | 'saved') => void): UseAudioSetupReturn {
  const [permissions, setPermissions] = useState<LinuxPermissions | null>(null);
  const [availableMics, setAvailableMics] = useState<AudioDevice[]>([]);
  const [micTestStatus, setMicTestStatus] = useState<'idle' | 'recording' | 'playing' | 'processing'>('idle');
  const [micVolume, setMicVolume] = useState<number>(0);
  const [micTestPassed, setMicTestPassed] = useState(false);
  const [hasLoadedSetupStatus, setHasLoadedSetupStatus] = useState(false);
  const [hasLoadedMics, setHasLoadedMics] = useState(false);

  const checkSetupStatus = useCallback(async () => {
    try {
      const perms = await invoke<LinuxPermissions>('get_linux_setup_status');
      setPermissions(perms);
      const bindingState = await invoke<HotkeyBindingState>('get_hotkey_binding_state');
      return { perms, bindingState };
    } catch (error) {
      console.error('Failed to check setup status:', error);
    } finally {
      setHasLoadedSetupStatus(true);
    }
  }, []);

  const loadMics = useCallback(async () => {
    try {
      const devices = await invoke<AudioDevice[]>('get_audio_devices');
      setAvailableMics(devices);
    } catch (error) {
      showToast(`Failed to load microphones: ${error}`, 'error');
    } finally {
      setHasLoadedMics(true);
    }
  }, [showToast]);

  const handleAudioSetup = useCallback(async () => {
    try {
      await invoke('request_audio_permission');
      showToast('Audio permission granted!', 'success');
      await checkSetupStatus();
    } catch (error) {
      showToast(`Failed to get audio permission: ${error}`, 'error');
    }
  }, [showToast, checkSetupStatus]);

  const handleInputSetup = useCallback(async () => {
    try {
      await invoke('request_input_permission');
      showToast('Input permission granted!', 'success');
      await checkSetupStatus();
    } catch (error) {
      showToast(`Failed to get input permission: ${error}`, 'error');
    }
  }, [showToast, checkSetupStatus]);

  const startMicTest = useCallback(async () => {
    try {
      setMicTestStatus('recording');
      await invoke('start_mic_test');
    } catch (error) {
      setMicTestStatus('idle');
      showToast(`Failed to start mic test: ${error}`, 'error');
    }
  }, [showToast]);

  const stopMicTest = useCallback(async () => {
    setMicTestStatus('processing');
    try {
      await invoke('stop_mic_test');
    } catch (error) {
      setMicTestStatus('idle');
      showToast(`Failed to stop mic test: ${error}`, 'error');
    }
  }, [showToast]);

  const stopMicPlayback = useCallback(async () => {
    try {
      await invoke('stop_mic_playback');
      setMicTestStatus('idle');
    } catch (error) {
      showToast(`Failed to stop playback: ${error}`, 'error');
    }
  }, [showToast]);

  return {
    permissions,
    setPermissions,
    availableMics,
    micTestStatus,
    micVolume,
    micTestPassed,
    hasLoadedSetupStatus,
    hasLoadedMics,
    loadMics,
    startMicTest,
    stopMicTest,
    stopMicPlayback,
    handleAudioSetup,
    handleInputSetup,
    checkSetupStatus,
    setMicTestStatus,
    setMicVolume,
    setMicTestPassed,
    setHasLoadedSetupStatus,
  };
}