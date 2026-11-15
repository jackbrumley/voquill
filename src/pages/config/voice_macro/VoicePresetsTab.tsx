import { IconTrash } from '@tabler/icons-preact';
import { Button } from '../../../components/Button.tsx';
import { tokens } from '../../../design-tokens.ts';
import type { VoicePreset } from '../../../types.ts';

interface VoicePresetsTabProps {
  presets: VoicePreset[];
  onLoad: (preset: VoicePreset) => void;
  onDelete: (presetId: string) => void;
}

/// Voice Studio's saved presets, each loadable back into the studio.
export function VoicePresetsTab({ presets, onLoad, onDelete }: VoicePresetsTabProps) {
  if (presets.length === 0) {
    return (
      <p style={{ color: tokens.colors.textMuted, fontSize: '12px', padding: '24px', textAlign: 'center' }}>
        No custom presets saved yet. Tune a voice in the Studio and click Save!
      </p>
    );
  }

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: '8px', width: '100%' }}>
      {presets.map((preset) => (
        <div
          key={preset.id}
          style={{
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'space-between',
            padding: '8px 12px',
            borderRadius: '6px',
            background: 'rgba(255, 255, 255, 0.03)',
            border: '1px solid rgba(255, 255, 255, 0.06)',
            gap: '8px',
          }}
        >
          <div style={{ minWidth: 0, flex: 1 }}>
            <div style={{ fontSize: '12px', fontWeight: 600, color: '#c7d2fe', whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'ellipsis' }}>
              {preset.name}
            </div>
            <div style={{ fontSize: '10.5px', color: tokens.colors.textMuted, whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'ellipsis' }}>
              {preset.model_key} • Pitch: {preset.pitch}st • Speed: {preset.speed}x • Radio: {preset.radio_bandpass ? 'Yes' : 'No'}
            </div>
          </div>
          <div style={{ display: 'flex', gap: '6px', flexShrink: 0 }}>
            <Button variant="primary" onClick={() => onLoad(preset)} style={{ fontSize: '10.5px', padding: '4px 8px' }}>
              Load in Studio
            </Button>
            <Button variant="danger" onClick={() => onDelete(preset.id)} style={{ fontSize: '10.5px', padding: '4px 8px' }}>
              <IconTrash size={12} />
            </Button>
          </div>
        </div>
      ))}
    </div>
  );
}
