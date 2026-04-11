
import { ComponentChildren } from 'preact';
import { IconChevronDown } from '@tabler/icons-preact';
import { SurfaceCard } from './SurfaceCard.tsx';
import { tokens } from '../design-tokens.ts';

interface CollapsibleSectionProps {
  title: string;
  children: ComponentChildren;
  isOpen: boolean;
  onToggle: () => void;
}

export const CollapsibleSection = ({ title, children, isOpen, onToggle }: CollapsibleSectionProps) => {
  return (
    <SurfaceCard
      style={{
        display: 'flex',
        flexDirection: 'column',
        background: 'transparent',
        backdropFilter: 'none',
        WebkitBackdropFilter: 'none',
        borderRadius: 0,
        border: 'none',
        boxShadow: 'none',
        overflow: 'visible',
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
          background: 'rgba(32, 34, 37, 0.3)',
          transition: tokens.transitions.fast,
          borderRadius: 0,
          position: 'relative',
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
        <div style={{ padding: tokens.spacing.md, display: 'flex', flexDirection: 'column', gap: tokens.spacing.sm, background: 'transparent' }}>
          {children}
        </div>
      )}
    </SurfaceCard>
  );
};
