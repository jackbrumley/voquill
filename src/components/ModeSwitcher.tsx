
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
  const modeClass = `mode-${activeIndex === 0 ? 'first' : 'second'}`;

  return (
    <div className={`mode-switcher-container ${className}`}>
      <div className={`mode-switcher ${modeClass}`}>
        <div className="mode-switcher-slider"></div>
        {options.map((option, index) => (
          <button 
            key={option.value}
            className={value === option.value ? 'active' : ''} 
            onClick={() => onToggle(option.value)}
            title={option.title}
          >
            <span>{option.label}</span>
          </button>
        ))}
      </div>
    </div>
  );
}
