
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
    <div className={className} style={{ display: 'flex' }}>
      <div
        style={{
          position: 'relative',
          display: 'flex',
          background: tokens.colors.bgSecondary,
          border: '1px solid rgba(255, 255, 255, 0.08)',
          borderRadius: '24px',
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
            background: 'linear-gradient(135deg, #5865f2 0%, #4338ca 100%)',
            borderRadius: '22px',
            transition: 'transform 0.3s cubic-bezier(0.4, 0, 0.2, 1)',
            boxShadow: '0 2px 8px rgba(88, 101, 242, 0.22)',
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
              fontSize: tokens.typography.sizeSm,
              fontWeight: 600,
              padding: '5px 12px',
              cursor: 'pointer',
              borderRadius: '22px',
              letterSpacing: '0.3px',
              minWidth: '120px',
              whiteSpace: 'nowrap',
              justifyContent: 'center',
              display: 'flex',
              alignItems: 'center',
              gap: '6px',
              transition: tokens.transitions.normal,
            }}
            onClick={() => {
              invoke('log_ui_event', { message: `[Button clicked] ${option.label}` }).catch(() => {});
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
