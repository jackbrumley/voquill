import { IconChevronLeft } from '@tabler/icons-preact';
import { useSignal } from '@preact/signals';
import type { DownloadPhase, GpuStatus, EngineCapabilities, Config, AudioDevice, ModelInfo } from '../types.ts';
import { tokens } from '../design-tokens.ts';
import { GeneralSection } from './config/GeneralSection.tsx';
import { AudioSection } from './config/AudioSection.tsx';
import { DictionarySection } from './config/DictionarySection.tsx';
import { FillerWordsSection } from './config/FillerWordsSection.tsx';
import { TranscriptionSection } from './config/TranscriptionSection.tsx';
import { PostProcessSection } from './config/PostProcessSection.tsx';
import { TypingSection } from './config/TypingSection.tsx';
import { DebugSection } from './config/DebugSection.tsx';

interface ConfigPageProps {
  config: Config;
  activeConfigSection: string | null;
  setActiveConfigSection: (value: string | null) => void;
  availableEngines: string[];
  availableModels: ModelInfo[];
  modelStatus: Record<string, boolean>;
  downloadProgress: number;
  downloadPhase: DownloadPhase;
  isDownloading: boolean;
  isTestingApi: boolean;
  portalVersion: number;
  isSystemManagedShortcut: boolean;
  hotkeyBindingState: { bound: boolean; active_trigger?: string } | null;
  isApplyingHotkey: boolean;
  availableMics: AudioDevice[];
  micTestStatus: 'idle' | 'recording' | 'playing' | 'processing';
  micVolume: number;
  overlayPositioningCapabilities: { manual_offset_supported: boolean; detail?: string };
  updateConfig: (key: string, value: string | number | boolean | null | string[] | Record<string, unknown> | unknown[]) => void;
  testApiKey: () => void;
  downloadModel: (size: string, engine?: string) => void;
  loadModels: () => void;
  loadMics: () => void;
  handleConfigureHotkey: () => void;
  setShowModelGuide: (show: boolean) => void;
  setShowPostProcessGuide: (show: boolean) => void;
  startMicTest: () => void;
  stopMicTest: () => void;
  stopMicPlayback: () => void;
  openDebugFolder: () => void;
  onReopenInitialSetup: () => void;
  onFactoryReset: () => void;
  checkingUpdates: boolean;
  onCheckForUpdates: () => void;
  onOpenUiLab: () => void;
  autostartEnabled: boolean;
  onToggleAutostart: (enabled: boolean) => void;
  testCleanupApi: () => void;
  gpuStatus: GpuStatus | null;
  postProcessGpuStatus: GpuStatus | null;
  engineCapabilities: EngineCapabilities | null;
}

const languageOptions = [
  { value: 'auto', label: 'Automatic Detection' },
  { value: 'en-AU', label: 'English (Australia)' },
  { value: 'en-GB', label: 'English (United Kingdom)' },
  { value: 'en-US', label: 'English (United States)' },
  { value: 'fr', label: 'French' },
  { value: 'es', label: 'Spanish' },
  { value: 'de', label: 'German' },
  { value: 'it', label: 'Italian' },
  { value: 'pt', label: 'Portuguese' },
  { value: 'nl', label: 'Dutch' },
  { value: 'ja', label: 'Japanese' },
  { value: 'zh', label: 'Chinese' },
];

const sectionTitleMap: Record<string, string> = {
  'general': 'General',
  'audio': 'Audio',
  'dictionary': 'Dictionary',
  'filler-words': 'Filler Words',
  'transcription': 'Transcription',
  'post-process': 'Post-Processing',
  'typing': 'Typing',
  'debug': 'Debug',
};

export function ConfigPage(props: ConfigPageProps) {
  const {
    config,
    activeConfigSection,
    setActiveConfigSection,
    availableEngines,
    availableModels,
    modelStatus,
    downloadProgress,
    downloadPhase,
    isDownloading,
    isTestingApi,
    portalVersion,
    isSystemManagedShortcut,
    hotkeyBindingState,
    isApplyingHotkey,
    availableMics,
    micTestStatus,
    micVolume,
    overlayPositioningCapabilities,
    updateConfig,
    testApiKey,
    downloadModel,
    loadModels,
    loadMics,
    handleConfigureHotkey,
    setShowModelGuide,
    setShowPostProcessGuide,
    startMicTest,
    stopMicTest,
    stopMicPlayback,
    openDebugFolder,
    onReopenInitialSetup,
    onFactoryReset,
    checkingUpdates,
    onCheckForUpdates,
    onOpenUiLab,
    autostartEnabled,
    onToggleAutostart,
    testCleanupApi,
    gpuStatus,
    postProcessGpuStatus,
    engineCapabilities,
  } = props;

  function SectionNavItem({ title, section }: { title: string; section: string }) {
    const isHovered = useSignal(false);

    return (
      <div
        onClick={() => setActiveConfigSection(section)}
        onMouseEnter={() => { isHovered.value = true; }}
        onMouseLeave={() => { isHovered.value = false; }}
        style={{
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          textAlign: 'center',
          gap: tokens.spacing.sm,
          padding: '14px 0',
          cursor: 'pointer',
          userSelect: 'none',
          WebkitUserSelect: 'none',
          MozUserSelect: 'none',
          background: isHovered.value ? 'rgba(255, 255, 255, 0.08)' : 'transparent',
          transition: 'background 0.15s ease',
        }}
      >
        <span style={{
          fontWeight: 700,
          fontSize: '15px',
          color: isHovered.value ? tokens.colors.textPrimary : tokens.colors.textSecondary,
          textTransform: 'uppercase',
          letterSpacing: '0.5px',
          transition: tokens.transitions.fast,
        }}>
          {title}
        </span>
      </div>
    );
  }

  return (
    <div style={{ height: '100%', display: 'flex', flexDirection: 'column', overflow: 'hidden' }} key="settings">
      {activeConfigSection === null ? (
        <div style={{ flex: 1, minHeight: 0, overflowY: 'auto', paddingTop: '12px' }}>
          <SectionNavItem title="General" section="general" />
          <SectionNavItem title="Audio" section="audio" />
          <SectionNavItem title="Dictionary" section="dictionary" />
          <SectionNavItem title="Filler Words" section="filler-words" />
          <SectionNavItem title="Transcription" section="transcription" />
          <SectionNavItem title="Post-Processing" section="post-process" />
          <SectionNavItem title="Typing" section="typing" />
          <SectionNavItem title="Debug" section="debug" />
        </div>
      ) : (
        <div style={{ display: 'flex', flexDirection: 'column', flex: 1, minHeight: 0, overflow: 'hidden' }}>
          <div
            onClick={() => setActiveConfigSection(null)}
            style={{
              display: 'flex',
              alignItems: 'center',
              gap: tokens.spacing.sm,
              padding: '12px 16px',
              cursor: 'pointer',
              userSelect: 'none',
              WebkitUserSelect: 'none',
              MozUserSelect: 'none',
              borderBottom: '1px solid rgba(255, 255, 255, 0.1)',
              flex: '0 0 auto',
              transition: 'background 0.15s ease',
            }}
            onMouseEnter={(e) => { (e.currentTarget as HTMLElement).style.background = 'rgba(255, 255, 255, 0.08)'; }}
            onMouseLeave={(e) => { (e.currentTarget as HTMLElement).style.background = 'transparent'; }}
          >
            <IconChevronLeft size={20} style={{ flexShrink: 0, color: tokens.colors.textMuted }} />
            <span style={{ fontWeight: 700, fontSize: '15px', color: tokens.colors.textPrimary, textTransform: 'uppercase', letterSpacing: '0.5px' }}>
              {sectionTitleMap[activeConfigSection]}
            </span>
          </div>
          <div style={{ flex: 1, minHeight: 0, overflowY: 'auto', padding: tokens.spacing.md, display: 'flex', flexDirection: 'column', gap: tokens.spacing.md }}>
            {activeConfigSection === 'general' && (
              <GeneralSection
                config={config}
                updateConfig={updateConfig}
                isSystemManagedShortcut={isSystemManagedShortcut}
                portalVersion={portalVersion}
                hotkeyBindingState={hotkeyBindingState}
                isApplyingHotkey={isApplyingHotkey}
                handleConfigureHotkey={handleConfigureHotkey}
                checkingUpdates={checkingUpdates}
                onCheckForUpdates={onCheckForUpdates}
                autostartEnabled={autostartEnabled}
                onToggleAutostart={onToggleAutostart}
                overlayPositioningCapabilities={overlayPositioningCapabilities}
                languageOptions={languageOptions}
              />
            )}

            {activeConfigSection === 'audio' && (
              <AudioSection
                config={config}
                updateConfig={updateConfig}
                availableMics={availableMics}
                loadMics={loadMics}
                micTestStatus={micTestStatus}
                micVolume={micVolume}
                startMicTest={startMicTest}
                stopMicTest={stopMicTest}
                stopMicPlayback={stopMicPlayback}
              />
            )}

            {activeConfigSection === 'dictionary' && (
              <DictionarySection
                config={config}
                updateConfig={updateConfig}
              />
            )}

            {activeConfigSection === 'filler-words' && (
              <FillerWordsSection
                config={config}
                updateConfig={updateConfig}
              />
            )}

            {activeConfigSection === 'transcription' && (
              <TranscriptionSection
                config={config}
                updateConfig={updateConfig}
                availableEngines={availableEngines}
                availableModels={availableModels}
                modelStatus={modelStatus}
                isDownloading={isDownloading}
                downloadProgress={downloadProgress}
                downloadPhase={downloadPhase}
                gpuStatus={gpuStatus}
                engineCapabilities={engineCapabilities}
                testApiKey={testApiKey}
                isTestingApi={isTestingApi}
                downloadModel={downloadModel}
                loadModels={loadModels}
                setShowModelGuide={setShowModelGuide}
              />
            )}

            {activeConfigSection === 'post-process' && (
              <PostProcessSection
                config={config}
                updateConfig={updateConfig}
                availableModels={availableModels}
                modelStatus={modelStatus}
                isDownloading={isDownloading}
                downloadProgress={downloadProgress}
                downloadPhase={downloadPhase}
                postProcessGpuStatus={postProcessGpuStatus}
                testCleanupApi={testCleanupApi}
                downloadModel={downloadModel}
                loadModels={loadModels}
                setShowPostProcessGuide={setShowPostProcessGuide}
              />
            )}

            {activeConfigSection === 'typing' && (
              <TypingSection
                config={config}
                updateConfig={updateConfig}
              />
            )}

            {activeConfigSection === 'debug' && (
              <DebugSection
                config={config}
                updateConfig={updateConfig}
                openDebugFolder={openDebugFolder}
                onReopenInitialSetup={onReopenInitialSetup}
                onFactoryReset={onFactoryReset}
                onOpenUiLab={onOpenUiLab}
              />
            )}
          </div>
        </div>
      )}
    </div>
  );
}
