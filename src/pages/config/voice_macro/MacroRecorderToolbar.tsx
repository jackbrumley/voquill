import { IconCircleFilled, IconCheck, IconTrash } from '@tabler/icons-preact';
import { Button } from '../../../components/Button.tsx';
import { tokens } from '../../../design-tokens.ts';

interface MacroRecorderToolbarProps {
  isRecording: boolean;
  stepCount: number;
  onToggleRecording: () => void;
  onClearSteps: () => void;
}

export function MacroRecorderToolbar({
  isRecording,
  stepCount,
  onToggleRecording,
  onClearSteps,
}: MacroRecorderToolbarProps) {
  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: '6px', flexShrink: 0 }}>
      {/* Live Sequence Recorder Controls Bar */}
      <div
        style={{
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'space-between',
          padding: '6px 10px',
          borderRadius: '6px',
          background: isRecording ? 'rgba(239, 68, 68, 0.12)' : 'rgba(255, 255, 255, 0.04)',
          border: isRecording
            ? '1px solid rgba(239, 68, 68, 0.45)'
            : '1px solid rgba(255, 255, 255, 0.08)',
        }}
      >
        <div style={{ display: 'flex', alignItems: 'center', gap: '6px' }}>
          <Button
            variant="configAction"
            onClick={onToggleRecording}
            style={
              isRecording
                ? {
                    display: 'flex',
                    alignItems: 'center',
                    gap: '4px',
                    padding: '4px 8px',
                    fontSize: '11px',
                    borderColor: '#ef4444',
                    color: '#f87171',
                    background: 'rgba(239, 68, 68, 0.2)',
                  }
                : {
                    display: 'flex',
                    alignItems: 'center',
                    gap: '4px',
                    padding: '4px 8px',
                    fontSize: '11px',
                  }
            }
          >
            {isRecording ? (
              <>
                <IconCheck size={13} />
                <span>Done Recording</span>
              </>
            ) : (
              <>
                <IconCircleFilled size={12} color="#ef4444" />
                <span>Record Sequence</span>
              </>
            )}
          </Button>

          {stepCount > 0 && !isRecording && (
            <Button
              variant="ghost"
              onClick={onClearSteps}
              style={{
                display: 'flex',
                alignItems: 'center',
                gap: '3px',
                padding: '4px 6px',
                fontSize: '11px',
                color: tokens.colors.textMuted,
              }}
            >
              <IconTrash size={12} />
              <span>Clear</span>
            </Button>
          )}
        </div>

        <span style={{ fontSize: '11px', color: tokens.colors.textMuted }}>
          {stepCount} {stepCount === 1 ? 'step' : 'steps'}
        </span>
      </div>

      {/* Live Recording Active Banner */}
      {isRecording && (
        <div
          style={{
            padding: '6px 10px',
            borderRadius: '5px',
            background: 'rgba(239, 68, 68, 0.15)',
            border: '1px solid rgba(239, 68, 68, 0.3)',
            fontSize: '11px',
            color: '#fca5a5',
            lineHeight: 1.3,
          }}
        >
          🔴 <strong>Recording...</strong> Press keys, modifiers, combos. Delays are captured. Press{' '}
          <strong>Esc</strong> to finish.
        </div>
      )}
    </div>
  );
}
