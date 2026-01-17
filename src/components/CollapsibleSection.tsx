
import { ComponentChildren } from 'preact';

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
          <svg 
            xmlns="http://www.w3.org/2000/svg" 
            width="16" height="16" 
            viewBox="0 0 24 24" fill="none" 
            stroke="currentColor" strokeWidth="2" 
            strokeLinecap="round" strokeLinejoin="round"
          >
            <path d="m6 9 6 6 6-6"/>
          </svg>
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
