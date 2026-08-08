
import { ComponentChildren } from 'preact';
import { useSignal } from '@preact/signals';
import { IconChevronLeft } from '@tabler/icons-preact';
import { tokens } from '../design-tokens.ts';

interface CollapsibleSectionProps {
  title: string;
  children: ComponentChildren;
  isOpen: boolean;
  onToggle: () => void;
}

export const CollapsibleSection = ({ title, children, isOpen, onToggle }: CollapsibleSectionProps) => {
  const isHovered = useSignal(false);

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
        onMouseEnter={() => { isHovered.value = true; }}
        onMouseLeave={() => { isHovered.value = false; }}
        style={{
          display: 'flex',
          alignItems: 'center',
          gap: tokens.spacing.sm,
          padding: `${tokens.spacing.sm} ${tokens.spacing.md}`,
          cursor: 'pointer',
          userSelect: 'none',
          WebkitUserSelect: 'none',
          MozUserSelect: 'none',
          background: isOpen
            ? tokens.colors.bgPrimary
            : isHovered.value
              ? 'rgba(255, 255, 255, 0.08)'
              : 'transparent',
          position: 'sticky',
          top: 0,
          zIndex: 2,
          borderBottom: isOpen ? `1px solid rgba(255, 255, 255, 0.2)` : '1px solid transparent',
          transition: 'background 0.15s ease',
        }}
      >
        {isOpen ? (
          <IconChevronLeft size={20} style={{ flexShrink: 0, color: tokens.colors.textMuted }} />
        ) : (
          <span style={{ color: isHovered.value ? tokens.colors.textPrimary : tokens.colors.textMuted, fontSize: '18px', lineHeight: 1, flexShrink: 0, transition: tokens.transitions.fast }}>•</span>
        )}
        <span style={{
          fontWeight: 700,
          fontSize: tokens.typography.sizeSm,
          color: isOpen ? tokens.colors.textPrimary : isHovered.value ? tokens.colors.textPrimary : tokens.colors.textSecondary,
          textTransform: 'uppercase',
          letterSpacing: '0.5px',
          transition: tokens.transitions.fast,
        }}>
          {title}
        </span>
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
