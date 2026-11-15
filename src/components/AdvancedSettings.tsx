import { ComponentChildren } from 'preact';
import { useSignal } from '@preact/signals';
import { IconChevronRight } from '@tabler/icons-preact';
import { tokens } from '../design-tokens.ts';

interface AdvancedSettingsProps {
  children: ComponentChildren;
}

/// Collapsed group at the end of a settings section for settings most people
/// never need. Everyday settings stay visible above it; these stay one click
/// away.
export function AdvancedSettings({ children }: AdvancedSettingsProps) {
  const isOpen = useSignal(false);

  return (
    <div style={{ display: 'flex', flexDirection: 'column' }}>
      <button
        type="button"
        aria-expanded={isOpen.value}
        onClick={() => { isOpen.value = !isOpen.value; }}
        style={{
          display: 'flex',
          alignItems: 'center',
          gap: '6px',
          alignSelf: 'flex-start',
          border: 'none',
          background: 'transparent',
          padding: `${tokens.spacing.sm} 0`,
          cursor: 'pointer',
          color: tokens.colors.textSecondary,
          fontSize: tokens.typography.sizeSm,
          fontWeight: 600,
        }}
      >
        <IconChevronRight
          size={16}
          style={{ transform: isOpen.value ? 'rotate(90deg)' : 'none', transition: tokens.transitions.fast }}
        />
        Advanced settings
      </button>
      {isOpen.value && (
        <div style={{ display: 'flex', flexDirection: 'column', gap: tokens.spacing.md }}>
          {children}
        </div>
      )}
    </div>
  );
}
