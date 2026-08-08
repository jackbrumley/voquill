
import { ComponentChildren } from 'preact';
import { useSignal } from '@preact/signals';
import { IconChevronDown } from '@tabler/icons-preact';
import { tokens } from '../design-tokens.ts';

interface CollapsibleSectionProps {
  title: string;
  children: ComponentChildren;
  isOpen: boolean;
  onToggle: () => void;
}

export const CollapsibleSection = ({ title, children, isOpen, onToggle }: CollapsibleSectionProps) => {
  const hovered = useSignal(false);

  const getHeaderBackground = () => {
    if (isOpen) return tokens.colors.bgTertiary;
    if (hovered.value) return 'rgba(32, 34, 37, 0.3)';
    return 'transparent';
  };

  return (
    <div
      style={{
        display: 'flex',
        flexDirection: 'column',
        overflow: 'visible',
      }}
    >
      <div
        onClick={onToggle}
        onMouseEnter={() => { hovered.value = true; }}
        onMouseLeave={() => { hovered.value = false; }}
        style={{
          display: 'flex',
          justifyContent: 'space-between',
          alignItems: 'center',
          padding: `${tokens.spacing.sm} ${tokens.spacing.md}`,
          cursor: 'pointer',
          userSelect: 'none',
          background: getHeaderBackground(),
          transition: tokens.transitions.fast,
          position: 'sticky',
          top: 0,
          zIndex: 1,
        }}
      >
        <div
          style={{
            display: 'flex',
            alignItems: 'center',
            gap: tokens.spacing.sm,
            fontWeight: 700,
            fontSize: tokens.typography.sizeSm,
            color: isOpen ? tokens.colors.textPrimary : tokens.colors.textSecondary,
            textTransform: 'uppercase',
            letterSpacing: '0.5px',
            transition: tokens.transitions.fast,
          }}
        >
          <span>{title}</span>
        </div>
        <div
          style={{
            transform: isOpen ? 'rotate(180deg)' : 'rotate(0deg)',
            transition: tokens.transitions.normal,
            color: isOpen ? tokens.colors.textPrimary : tokens.colors.textMuted,
          }}
        >
          <IconChevronDown size={16} />
        </div>
      </div>
      {isOpen && (
        <div style={{
          padding: tokens.spacing.md,
          display: 'flex',
          flexDirection: 'column',
          gap: tokens.spacing.md,
          background: 'rgba(35, 37, 42, 0.5)',
          backdropFilter: 'blur(12px)',
          WebkitBackdropFilter: 'blur(12px)',
          border: '1px solid rgba(255, 255, 255, 0.06)',
          borderTop: 'none',
        }}>
          {children}
        </div>
      )}
    </div>
  );
};
