
import { invoke } from '@tauri-apps/api/core';
import { tokens } from '../design-tokens.ts';

interface ModeOption<T> {
  value: T;
  label: string;
  title: string;
}

interface ModeSwitcherProps<T> {
  value: T;
  options: [ModeOption<T>, ModeOption<T>];
  onToggle: (value: T) => void;
  className?: string;
}

export function ModeSwitcher<T extends string>({ value, options, onToggle, className = "" }: ModeSwitcherProps<T>) {
  const activeIndex = options.findIndex(opt => opt.value === value);
  const sliderTransform = activeIndex === 0 ? 'translateX(0)' : 'translateX(100%)';

  return (
    <div className={className} style={{ marginTop: tokens.spacing.md, display: 'flex', justifyContent: 'center', width: '100%' }}>
      <div
        style={{
          position: 'relative',
          display: 'flex',
          background: tokens.colors.bgSecondary,
          borderRadius: '30px',
          padding: '4px',
          boxShadow: tokens.shadows.sm,
          width: 'auto',
        }}
      >
        <div
          style={{
            position: 'absolute',
            top: '4px',
            left: '4px',
            width: 'calc(50% - 4px)',
            height: 'calc(100% - 8px)',
            background: tokens.colors.accentPrimary,
            borderRadius: '26px',
            transition: 'transform 0.3s cubic-bezier(0.4, 0, 0.2, 1)',
            boxShadow: tokens.shadows.md,
            zIndex: 1,
            transform: sliderTransform,
          }}
        ></div>
        {options.map((option) => (
          <button
            key={option.value}
            style={{
              position: 'relative',
              zIndex: 1,
              flex: 1,
              border: 'none',
              background: 'transparent',
              color: value === option.value ? tokens.colors.textPrimary : tokens.colors.textSecondary,
              fontSize: tokens.typography.sizeXs,
              fontWeight: 700,
              padding: '6px 16px',
              cursor: 'pointer',
              borderRadius: '26px',
              textTransform: 'uppercase',
              letterSpacing: '0.5px',
              minWidth: '140px',
              justifyContent: 'center',
              display: 'flex',
              alignItems: 'center',
              gap: tokens.spacing.sm,
              transition: tokens.transitions.normal,
            }}
            onClick={() => {
              invoke('log_ui_event', { message: `🖱️ Button clicked: ${option.label}` }).catch(() => {});
              onToggle(option.value);
            }}
            title={option.title}
          >
            <span>{option.label}</span>
          </button>
        ))}
      </div>
    </div>
  );
}
