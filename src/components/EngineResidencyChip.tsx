import { invoke } from '@tauri-apps/api/core';
import { useSignal } from '@preact/signals';
import { tokens } from '../design-tokens.ts';
import { describeIdleUnload, type UseEngineResidencyReturn } from '../hooks/useEngineResidency.ts';
import type { Config, EngineResidency } from '../types.ts';
import { Button } from './Button.tsx';
import { Modal } from './Modal.tsx';

interface EngineResidencyChipProps {
  config: Config;
  engine: UseEngineResidencyReturn;
}

interface ResidencyPresentation {
  title: string;
  note: string | null;
  dotColor: string;
  tooltip: string;
}

/// The home screen is sized to fit the default window without scrolling, so
/// this stays compact: a 22px status row, plus a 14px note line only when
/// voice macros are on (the only time a note can appear). The space is
/// reserved even while the button or note is absent, so the content below
/// never moves when the state changes.
const ROW_HEIGHT_PX = 22;
const NOTE_HEIGHT_PX = 14;

function reservedHeight(config: Config): string {
  return `${ROW_HEIGHT_PX + (config.voice_macros_enabled ? NOTE_HEIGHT_PX : 0)}px`;
}

const compactButtonStyle = { padding: '2px 10px', lineHeight: '16px' };

function present(residency: EngineResidency, config: Config): ResidencyPresentation {
  switch (residency) {
    case 'Loaded':
      return {
        title: 'Speech engine ready',
        note: null,
        dotColor: tokens.colors.success,
        tooltip: `Unloading frees up memory for other apps. ${describeIdleUnload(config.engine_idle_unload_minutes)}`,
      };
    case 'Loading':
      return {
        title: 'Speech engine waking up…',
        note: null,
        dotColor: '#f59e0b',
        tooltip: 'This takes a few seconds.',
      };
    case 'Unloaded':
      return {
        title: 'Speech engine asleep',
        note: config.voice_macros_enabled ? 'Voice macros paused' : null,
        dotColor: tokens.colors.textMuted,
        tooltip: `Unloaded to save memory. It wakes up when you press ${config.hotkey}.`,
      };
  }
}

/// A quiet status line showing whether the speech engine is in memory, with a
/// button to unload or wake it. Unloading asks first when voice macros are on,
/// because they pause until the engine wakes up again.
export function EngineResidencyChip({ config, engine }: EngineResidencyChipProps) {
  const { residency, loadEngine, unloadEngine } = engine;
  const confirmingUnload = useSignal(false);
  if (residency === null) {
    return <div style={{ minHeight: reservedHeight(config) }} />;
  }

  const presentation = present(residency, config);

  const requestUnload = () => {
    if (config.voice_macros_enabled) {
      confirmingUnload.value = true;
    } else {
      void unloadEngine();
    }
  };

  const confirmUnload = () => {
    confirmingUnload.value = false;
    void unloadEngine();
  };

  const action = residency === 'Loaded'
    ? { label: 'Unload', run: requestUnload }
    : residency === 'Unloaded'
      ? { label: 'Wake up', run: () => void loadEngine() }
      : null;

  return (
    <>
      <div
        title={presentation.tooltip}
        style={{
          display: 'flex',
          justifyContent: 'center',
          alignItems: 'flex-start',
          gap: '8px',
          minHeight: reservedHeight(config),
          fontSize: tokens.typography.sizeXs,
          color: tokens.colors.textMuted,
        }}
      >
        <span style={{
          width: '6px',
          height: '6px',
          marginTop: `${(ROW_HEIGHT_PX - 6) / 2}px`,
          borderRadius: '50%',
          background: presentation.dotColor,
          flexShrink: 0,
        }} />
        <div style={{ display: 'flex', flexDirection: 'column', alignItems: 'flex-start' }}>
          <span style={{ lineHeight: `${ROW_HEIGHT_PX}px` }}>{presentation.title}</span>
          {presentation.note && (
            <span style={{ lineHeight: `${NOTE_HEIGHT_PX}px`, opacity: 0.8 }}>{presentation.note}</span>
          )}
        </div>
        {action && (
          <Button
            variant="secondary"
            size="sm"
            pill
            style={compactButtonStyle}
            onClick={() => {
              invoke('log_ui_event', { message: `[Button clicked] Speech engine ${action.label}` }).catch(() => {});
              action.run();
            }}
          >
            {action.label}
          </Button>
        )}
      </div>

      {confirmingUnload.value && (
        <Modal
          title="Unload speech engine?"
          onClose={() => { confirmingUnload.value = false; }}
          maxWidth="420px"
          centerContent
          footerAlign="center"
          footer={
            <>
              <Button variant="ghost" pill onClick={() => { confirmingUnload.value = false; }}>
                Cancel
              </Button>
              <Button variant="primary" pill onClick={confirmUnload}>
                Unload
              </Button>
            </>
          }
        >
          <p style={{ margin: 0, fontSize: tokens.typography.sizeSm, color: tokens.colors.textSecondary, lineHeight: 1.5, textAlign: 'center' }}>
            Voice macros will pause until the speech engine wakes up again. It wakes up when you press <strong>{config.hotkey}</strong>.
          </p>
        </Modal>
      )}
    </>
  );
}
