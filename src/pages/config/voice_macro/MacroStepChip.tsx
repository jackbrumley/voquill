import { IconClock } from '@tabler/icons-preact';
import type { MacroStep } from '../../../types.ts';

interface MacroStepChipProps {
  step: MacroStep;
}

export function MacroStepChip({ step }: MacroStepChipProps) {
  if (step.type === 'KeyPress') {
    return (
      <span
        style={{
          padding: '2px 6px',
          borderRadius: '4px',
          background: 'rgba(88, 101, 242, 0.18)',
          border: '1px solid rgba(88, 101, 242, 0.35)',
          fontSize: '11px',
          fontWeight: 600,
          color: '#9ba5ff',
          fontFamily: 'monospace',
          display: 'inline-flex',
          alignItems: 'center',
          gap: '3px',
        }}
      >
        <span>⌨️ {step.key}</span>
        {step.hold_ms && step.hold_ms !== 50 && (
          <span style={{ fontSize: '10px', opacity: 0.7 }}>({step.hold_ms}ms)</span>
        )}
      </span>
    );
  }

  if (step.type === 'KeyDown') {
    return (
      <span
        style={{
          padding: '2px 6px',
          borderRadius: '4px',
          background: 'rgba(245, 158, 11, 0.18)',
          border: '1px solid rgba(245, 158, 11, 0.35)',
          fontSize: '11px',
          fontWeight: 600,
          color: '#fbbf24',
          fontFamily: 'monospace',
          display: 'inline-flex',
          alignItems: 'center',
          gap: '3px',
        }}
      >
        <span>⬇️ {step.key}</span>
      </span>
    );
  }

  if (step.type === 'KeyUp') {
    return (
      <span
        style={{
          padding: '2px 6px',
          borderRadius: '4px',
          background: 'rgba(168, 85, 247, 0.18)',
          border: '1px solid rgba(168, 85, 247, 0.35)',
          fontSize: '11px',
          fontWeight: 600,
          color: '#c084fc',
          fontFamily: 'monospace',
          display: 'inline-flex',
          alignItems: 'center',
          gap: '3px',
        }}
      >
        <span>⬆️ {step.key}</span>
      </span>
    );
  }

  if (step.type === 'Delay') {
    return (
      <span
        style={{
          padding: '2px 5px',
          borderRadius: '4px',
          background: 'rgba(14, 165, 233, 0.14)',
          border: '1px solid rgba(14, 165, 233, 0.28)',
          fontSize: '10px',
          fontWeight: 500,
          color: '#38bdf8',
          display: 'inline-flex',
          alignItems: 'center',
          gap: '2px',
        }}
      >
        <IconClock size={10} />
        <span>{step.duration_ms}ms</span>
      </span>
    );
  }

  if (step.type === 'TypeText') {
    return (
      <span
        style={{
          padding: '2px 6px',
          borderRadius: '4px',
          background: 'rgba(16, 185, 129, 0.18)',
          border: '1px solid rgba(16, 185, 129, 0.35)',
          fontSize: '11px',
          fontWeight: 600,
          color: '#34d399',
          fontFamily: 'monospace',
          display: 'inline-flex',
          alignItems: 'center',
          gap: '3px',
        }}
      >
        <span>📝 "{step.text}"</span>
      </span>
    );
  }

  return null;
}
