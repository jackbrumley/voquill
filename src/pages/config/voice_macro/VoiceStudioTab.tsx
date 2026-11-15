import { IconLoader2, IconPlayerPlay, IconPlayerStop, IconPlus, IconRefresh } from '@tabler/icons-preact';
import { Button } from '../../../components/Button.tsx';
import { DownloadProgressBar } from '../../../components/DownloadProgressBar.tsx';
import { SelectField } from '../../../components/SelectField.tsx';
import { SliderField } from '../../../components/SliderField.tsx';
import { tokens } from '../../../design-tokens.ts';
import { inputBaseStyle } from '../../../theme/ui-primitives.ts';
import { CHIME_OPTIONS } from './voiceStudioCatalog.ts';
import type { VoiceStudio } from './useVoiceStudio.ts';

interface VoiceStudioTabProps {
  studio: VoiceStudio;
}

const fieldLabelStyle = {
  fontSize: '11px',
  fontWeight: 600,
  color: tokens.colors.textMuted,
  textTransform: 'uppercase' as const,
};

interface TuningSliderProps {
  label: string;
  display: string;
  min: number;
  max: number;
  step: number;
  value: number;
  onChange: (value: number) => void;
}

function TuningSlider({ label, display, min, max, step, value, onChange }: TuningSliderProps) {
  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: '4px' }}>
      <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: '11px', color: tokens.colors.textMuted }}>
        <span>{label}</span>
        <span style={{ fontFamily: 'monospace', color: '#c7d2fe' }}>{display}</span>
      </div>
      <SliderField min={min} max={max} step={step} value={value} onChange={onChange} hideEndLabels ariaLabel={label} />
    </div>
  );
}

function ChimeSelect({ label, value, onChange }: { label: string; value: string; onChange: (value: string) => void }) {
  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: '4px' }}>
      <label style={{ fontSize: '11px', fontWeight: 600, color: tokens.colors.textMuted }}>{label}</label>
      <SelectField value={value} options={CHIME_OPTIONS} onChange={onChange} ariaLabel={label} style={{ width: '100%', fontSize: '11.5px' }} />
    </div>
  );
}

/// Voice Studio's tuning form: voice, test phrase, tuning knobs and preview.
export function VoiceStudioTab({ studio }: VoiceStudioTabProps) {
  const { settings } = studio;
  const previewLabel = studio.isDownloading
    ? 'Downloading Voice Model...'
    : studio.isSynthesizing
      ? 'Rendering Audio...'
      : studio.isPlaying
        ? 'Stop Audio Preview'
        : 'Preview Audio Feedback';
  const PreviewIcon = studio.isDownloading || studio.isSynthesizing
    ? IconLoader2
    : studio.isPlaying
      ? IconPlayerStop
      : IconPlayerPlay;

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: '12px', width: '100%' }}>
      <div style={{ display: 'flex', flexDirection: 'column', gap: '4px' }}>
        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
          <label style={{ ...fieldLabelStyle, letterSpacing: '0.4px' }}>Preset Name</label>
          {studio.currentPresetId && (
            <button
              type="button"
              onClick={studio.startNewPreset}
              style={{
                background: 'none',
                border: 'none',
                color: tokens.colors.accentPrimary,
                fontSize: '10.5px',
                fontWeight: 600,
                cursor: 'pointer',
                display: 'flex',
                alignItems: 'center',
                gap: '3px',
                padding: 0,
              }}
              title="Clear current preset and start a new unlinked voice"
            >
              <IconPlus size={11} />
              <span>New Preset</span>
            </button>
          )}
        </div>
        <input
          type="text"
          value={studio.presetName}
          onInput={(event) => studio.setPresetName((event.target as HTMLInputElement).value)}
          placeholder="e.g. British SAS Overlord, Cockpit AI"
          style={{ ...inputBaseStyle, padding: '8px 10px', fontSize: '12px' }}
        />
      </div>

      <div style={{ display: 'flex', flexDirection: 'column', gap: '4px' }}>
        <label style={fieldLabelStyle}>Base Neural Voice Model</label>
        <SelectField
          value={settings.model_key}
          options={studio.models.map((model) => ({ value: model.id, label: model.label }))}
          onChange={(modelKey) => studio.update({ model_key: modelKey })}
          ariaLabel="Base Voice Model"
          style={{ width: '100%', fontSize: '11.5px' }}
        />
      </div>

      {studio.isMultiSpeaker && (
        <div style={{ display: 'flex', flexDirection: 'column', gap: '6px', background: 'rgba(99, 102, 241, 0.08)', padding: '10px 12px', borderRadius: '8px', border: '1px solid rgba(99, 102, 241, 0.2)' }}>
          <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: '11px', fontWeight: 600, color: '#c7d2fe' }}>
            <span>Speaker ID (0 to 903)</span>
            <span style={{ fontFamily: 'monospace' }}>{settings.speaker_id}</span>
          </div>
          <SliderField
            min={0}
            max={903}
            step={1}
            value={settings.speaker_id}
            onChange={(value) => studio.update({ speaker_id: Math.round(value) })}
            hideEndLabels
            ariaLabel="Speaker ID"
          />
          <span style={{ fontSize: '10px', color: tokens.colors.textMuted }}>
            Tip: Try ID 700 (Heavy Titan) or ID 200 (Operations Dispatcher).
          </span>
        </div>
      )}

      <div style={{ display: 'flex', flexDirection: 'column', gap: '4px' }}>
        <label style={fieldLabelStyle}>Spoken Test Phrase</label>
        <textarea
          value={studio.testPhrase}
          onInput={(event) => studio.setTestPhrase((event.target as HTMLTextAreaElement).value)}
          rows={2}
          style={{ ...inputBaseStyle, fontSize: '11.5px', padding: '7px 9px', resize: 'vertical' }}
        />
      </div>

      <div style={{ display: 'flex', flexDirection: 'column', gap: '12px', background: 'rgba(0, 0, 0, 0.25)', padding: '12px 14px', borderRadius: '8px', border: '1px solid rgba(255, 255, 255, 0.06)' }}>
        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', borderBottom: '1px solid rgba(255, 255, 255, 0.06)', paddingBottom: '6px' }}>
          <span style={{ fontSize: '11px', fontWeight: 700, color: tokens.colors.textSecondary, textTransform: 'uppercase', letterSpacing: '0.4px' }}>
            Acoustic Tuning Knobs
          </span>
          <button
            type="button"
            onClick={studio.resetTuningKnobs}
            style={{
              background: 'none',
              border: 'none',
              color: tokens.colors.textMuted,
              fontSize: '10.5px',
              fontWeight: 600,
              cursor: 'pointer',
              display: 'flex',
              alignItems: 'center',
              gap: '3px',
              transition: tokens.transitions.fast,
            }}
            onMouseEnter={(event) => { (event.currentTarget as HTMLButtonElement).style.color = '#ffffff'; }}
            onMouseLeave={(event) => { (event.currentTarget as HTMLButtonElement).style.color = tokens.colors.textMuted; }}
          >
            <IconRefresh size={11} />
            <span>Reset</span>
          </button>
        </div>

        <TuningSlider
          label="Pitch Shift"
          display={`${settings.pitch > 0 ? `+${settings.pitch}` : settings.pitch} st`}
          min={-12}
          max={12}
          step={0.5}
          value={settings.pitch}
          onChange={(pitch) => studio.update({ pitch })}
        />
        <TuningSlider
          label="Playback Speed"
          display={`${settings.speed.toFixed(2)}x`}
          min={0.6}
          max={1.5}
          step={0.05}
          value={settings.speed}
          onChange={(speed) => studio.update({ speed })}
        />
        <TuningSlider
          label="Sub-Bass Weight (<140Hz)"
          display={`${Math.round(settings.sub_bass * 100)}%`}
          min={0}
          max={1}
          step={0.05}
          value={settings.sub_bass}
          onChange={(subBass) => studio.update({ sub_bass: subBass })}
        />
        <TuningSlider
          label="Cockpit Comb Resonance"
          display={`${Math.round(settings.comb_mix * 100)}%`}
          min={0}
          max={1}
          step={0.05}
          value={settings.comb_mix}
          onChange={(combMix) => studio.update({ comb_mix: combMix })}
        />

        <ChimeSelect label="Opening Sound Effect" value={settings.opening_chime} onChange={(chime) => studio.update({ opening_chime: chime })} />
        <ChimeSelect label="Closing Sound Effect" value={settings.closing_chime} onChange={(chime) => studio.update({ closing_chime: chime })} />

        <div style={{ display: 'flex', alignItems: 'center', gap: '6px', marginTop: '4px' }}>
          <input
            type="checkbox"
            id="vl_radio_check"
            checked={settings.radio_bandpass}
            onChange={(event) => studio.setRadioBandpass((event.target as HTMLInputElement).checked)}
            style={{ accentColor: tokens.colors.accentPrimary, cursor: 'pointer' }}
          />
          <label htmlFor="vl_radio_check" style={{ fontSize: '10.5px', color: tokens.colors.textPrimary, cursor: 'pointer' }}>
            Military VHF Bandpass (420Hz – 3.4kHz)
          </label>
        </div>

        {settings.radio_bandpass && (
          <div style={{ display: 'flex', flexDirection: 'column', gap: '4px', paddingLeft: '14px' }}>
            <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: '10px', color: tokens.colors.textMuted }}>
              <span>Radio Drive: {settings.radio_drive.toFixed(1)}x</span>
            </div>
            <SliderField
              min={1.0}
              max={4.0}
              step={0.2}
              value={settings.radio_drive}
              onChange={(radioDrive) => studio.update({ radio_drive: radioDrive })}
              hideEndLabels
              ariaLabel="Radio Drive"
            />
          </div>
        )}
      </div>

      {studio.isDownloading && (
        <div style={{ width: '100%', marginTop: '4px' }}>
          <DownloadProgressBar
            isDownloading
            progress={studio.downloadProgress}
            phase={studio.downloadPhase}
            itemLabel="Voice Model"
          />
        </div>
      )}

      <div>
        <Button
          variant={studio.isPlaying ? 'danger' : 'primary'}
          onClick={() => void studio.togglePreview()}
          disabled={studio.isSynthesizing || studio.isDownloading}
          style={{
            width: '100%',
            padding: '8px 14px',
            fontSize: '12px',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            gap: '6px',
            ...(studio.isPlaying ? { background: 'rgba(239, 68, 68, 0.3)', borderColor: '#ef4444', color: '#fca5a5' } : {}),
          }}
        >
          <PreviewIcon size={14} className={studio.isDownloading || studio.isSynthesizing ? 'spin' : undefined} />
          <span>{previewLabel}</span>
        </Button>
      </div>
    </div>
  );
}
