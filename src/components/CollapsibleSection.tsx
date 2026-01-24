
import { ComponentChildren } from 'preact';
import { IconChevronDown } from '@tabler/icons-react';

interface CollapsibleSectionProps {
  title: string;
  children: ComponentChildren;
  isOpen: boolean;
  onToggle: () => void;
}

export const CollapsibleSection = ({ title, children, isOpen, onToggle }: CollapsibleSectionProps) => {
  return (
    <div className={`collapsible-section ${isOpen ? 'is-open' : ''}`}>
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
    </div>
  );
};
