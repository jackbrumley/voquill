
import { ComponentChildren } from 'preact';
import { IconChevronDown } from '@tabler/icons-preact';
import { tokens } from '../design-tokens.ts';

interface CollapsibleSectionProps {
  title: string;
  children: ComponentChildren;
  isOpen: boolean;
  onToggle: () => void;
}

export const CollapsibleSection = ({ title, children, isOpen, onToggle }: CollapsibleSectionProps) => {
  return (
    <div
      style={{
        display: 'flex',
        flexDirection: 'column',
        overflow: 'visible',
        flex: isOpen ? 1 : '0 0 auto',
        minHeight: 0,
      }}
    >
      <div
        onClick={onToggle}
        style={{
          display: 'flex',
          justifyContent: 'space-between',
          alignItems: 'center',
          padding: `${tokens.spacing.sm} ${tokens.spacing.md}`,
          cursor: 'pointer',
          userSelect: 'none',
          background: 'transparent',
          position: 'sticky',
          top: 0,
          zIndex: 1,
          borderBottom: isOpen ? `1px solid rgba(255, 255, 255, 0.2)` : '1px solid transparent',
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
            transform: isOpen ? 'rotate(0deg)' : 'rotate(-90deg)',
            transition: tokens.transitions.normal,
            color: isOpen ? tokens.colors.textPrimary : tokens.colors.textMuted,
          }}
        >
          <IconChevronDown size={20} />
        </div>
      </div>
      {isOpen && (
        <div style={{
          padding: tokens.spacing.md,
          display: 'flex',
          flexDirection: 'column',
          gap: tokens.spacing.md,
          overflowY: 'auto',
          minHeight: 0,
        }}>
          {children}
        </div>
      )}
    </div>
  );
};
