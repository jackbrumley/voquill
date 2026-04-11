
import { ComponentChildren } from 'preact';
import { IconChevronDown } from '@tabler/icons-preact';
import { SurfaceCard } from './SurfaceCard.tsx';

interface CollapsibleSectionProps {
  title: string;
  children: ComponentChildren;
  isOpen: boolean;
  onToggle: () => void;
}

export const CollapsibleSection = ({ title, children, isOpen, onToggle }: CollapsibleSectionProps) => {
  return (
    <SurfaceCard className={`collapsible-section ${isOpen ? 'is-open' : ''}`}>
      <div className="collapsible-header" onClick={onToggle}>
        <div className="collapsible-title">
          <span>{title}</span>
        </div>
        <div className="collapsible-chevron">
          <IconChevronDown size={16} />
        </div>
      </div>
      {isOpen && (
        <div className="collapsible-content">
          {children}
        </div>
      )}
    </SurfaceCard>
  );
};
