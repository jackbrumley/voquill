
import { tokens } from '../design-tokens.ts';

interface ModeSwitcherProps {
  method: 'Typewriter' | 'Clipboard';
  onToggle: (method: 'Typewriter' | 'Clipboard') => void;
}

export const ModeSwitcher = ({ method, onToggle }: ModeSwitcherProps) => {
  return (
    <div className="mode-switcher-container">
      <div className={`mode-switcher mode-${method.toLowerCase()}`}>
        <div className="mode-switcher-slider"></div>
        <button 
          className={method === 'Typewriter' ? 'active' : ''} 
          onClick={() => onToggle('Typewriter')}
          title="Typewriter Mode: Simulates key presses"
        >
          <span className="mode-icon">⌨️</span>
          <span>Typewriter</span>
        </button>
        <button 
          className={method === 'Clipboard' ? 'active' : ''} 
          onClick={() => onToggle('Clipboard')}
          title="Clipboard Mode: Fast copy-paste"
        >
          <span className="mode-icon">📋</span>
          <span>Clipboard</span>
        </button>
      </div>
    </div>
  );
};
