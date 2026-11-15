import { useSignal } from '@preact/signals';
import { IconAlertTriangle, IconCheck } from '@tabler/icons-preact';
import { Modal } from '../../../components/Modal.tsx';
import { Button } from '../../../components/Button.tsx';
import type { VoicePreset } from '../../../types.ts';
import { tokens } from '../../../design-tokens.ts';
import { useVoiceStudio } from './useVoiceStudio.ts';
import { VoiceStudioTab } from './VoiceStudioTab.tsx';
import { VoicePresetsTab } from './VoicePresetsTab.tsx';

interface VoiceLabModalProps {
  onClose: () => void;
  onPresetSaved?: (preset: VoicePreset) => void;
}

type TabType = 'studio' | 'presets';

/// Voice Studio: tune a spoken-feedback voice and save it as a preset.
export function VoiceLabModal({ onClose, onPresetSaved }: VoiceLabModalProps) {
  const activeTab = useSignal<TabType>('studio');
  const studio = useVoiceStudio(onPresetSaved);

  const tabs: { id: TabType; label: string }[] = [
    { id: 'studio', label: 'Studio' },
    { id: 'presets', label: studio.savedPresets.length > 0 ? `Presets (${studio.savedPresets.length})` : 'Presets' },
  ];

  return (
    <Modal
      onClose={onClose}
      showCloseButton={false}
      maxWidth="540px"
      topBar={
        <div
          style={{
            display: 'grid',
            gridTemplateColumns: 'repeat(2, 1fr)',
            gap: '4px',
            background: 'rgba(0, 0, 0, 0.25)',
            padding: '3px',
            borderRadius: '999px',
            border: '1px solid rgba(255, 255, 255, 0.08)',
          }}
        >
          {tabs.map((tab) => {
            const isActive = activeTab.value === tab.id;
            return (
              <button
                key={tab.id}
                type="button"
                onClick={() => { activeTab.value = tab.id; }}
                style={{
                  display: 'flex',
                  alignItems: 'center',
                  justifyContent: 'center',
                  padding: '6px 12px',
                  borderRadius: '999px',
                  background: isActive
                    ? 'linear-gradient(135deg, rgba(88, 101, 242, 0.35) 0%, rgba(129, 140, 248, 0.2) 100%)'
                    : 'transparent',
                  border: isActive ? '1px solid rgba(99, 102, 241, 0.55)' : '1px solid transparent',
                  color: isActive ? '#ffffff' : tokens.colors.textSecondary,
                  fontSize: '12px',
                  fontWeight: isActive ? 600 : 500,
                  cursor: 'pointer',
                  transition: tokens.transitions.fast,
                  whiteSpace: 'nowrap',
                }}
              >
                <span>{tab.label}</span>
              </button>
            );
          })}
        </div>
      }
      footer={
        <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', width: '100%', gap: '8px' }}>
          <div>
            {studio.saveMessage && (
              <span style={{ fontSize: '11px', color: tokens.colors.success, display: 'flex', alignItems: 'center', gap: '4px' }}>
                <IconCheck size={13} /> {studio.saveMessage}
              </span>
            )}
          </div>

          <div style={{ display: 'flex', alignItems: 'center', gap: '6px' }}>
            <Button variant="ghost" onClick={onClose} style={{ padding: '6px 14px', fontSize: '12px' }}>
              Close
            </Button>
            {activeTab.value === 'studio' && (
              <Button
                variant="configAction"
                onClick={() => void studio.savePreset()}
                disabled={!studio.presetName.trim()}
                style={{ padding: '6px 16px', fontSize: '12px', display: 'flex', alignItems: 'center', gap: '5px' }}
              >
                <IconCheck size={14} />
                <span>{studio.currentPresetId ? 'Update Preset' : 'Save Preset'}</span>
              </Button>
            )}
          </div>
        </div>
      }
    >
      <div style={{ display: 'flex', flexDirection: 'column', gap: '14px', width: '100%' }}>
        {activeTab.value === 'studio' && <VoiceStudioTab studio={studio} />}

        {activeTab.value === 'presets' && (
          <VoicePresetsTab
            presets={studio.savedPresets}
            onLoad={(preset) => {
              studio.loadPreset(preset);
              activeTab.value = 'studio';
            }}
            onDelete={(presetId) => void studio.deletePreset(presetId)}
          />
        )}

        {studio.error && (
          <div style={{ display: 'flex', alignItems: 'center', gap: '4px', fontSize: '11px', color: '#f87171', flexShrink: 0 }}>
            <IconAlertTriangle size={13} />
            <span>{studio.error}</span>
          </div>
        )}
      </div>
    </Modal>
  );
}
