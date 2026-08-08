import type { AppRoute } from '../types.ts';

const ACCENT = '#5865f2';
const TEXT_INACTIVE = '#A9ACB5';

interface Tab {
  value: AppRoute;
  label: string;
}

interface TabBarProps {
  active: AppRoute;
  tabs: Tab[];
  onNavigate: (route: AppRoute) => void;
  onLogUI: (msg: string) => void;
}

export function TabBar({ active, tabs, onNavigate, onLogUI }: TabBarProps) {
  const activeIndex = tabs.findIndex((t) => t.value === active);
  const n = tabs.length;
  const sliderWidth = `calc(${100 / n}% - 4px)`;
  const sliderTransform = `translateX(${activeIndex * 100}%)`;

  return (
    <div style={{ display: 'flex', justifyContent: 'center', padding: '8px 8px 4px 8px', width: '100%' }}>
      <div
        style={{
          position: 'relative',
          display: 'flex',
          background: 'transparent',
          borderRadius: '30px',
          padding: '4px',
          width: '100%',
        }}
      >
        <div
          style={{
            position: 'absolute',
            top: '4px',
            left: '4px',
            width: sliderWidth,
            height: 'calc(100% - 8px)',
            background: 'transparent',
            borderRadius: '26px',
            transition: 'transform 0.3s cubic-bezier(0.4, 0, 0.2, 1)',
            border: `1px solid ${ACCENT}`,
            zIndex: 1,
            transform: sliderTransform,
          }}
        ></div>
        {tabs.map((tab) => {
          const isActive = tab.value === active;
          return (
            <button
              key={tab.value}
              type="button"
              style={{
                position: 'relative',
                zIndex: 2,
                flex: 1,
                border: 'none',
                background: 'transparent',
                color: isActive ? '#ffffff' : TEXT_INACTIVE,
                fontSize: '13px',
                fontWeight: isActive ? 600 : 400,
                padding: '7px 16px',
                cursor: 'pointer',
                borderRadius: '26px',
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'center',
                transition: 'color 0.15s ease',
              }}
              onClick={() => {
                onLogUI(`Tab: ${tab.label}`);
                onNavigate(tab.value);
              }}
              aria-current={isActive ? 'page' : undefined}
            >
              {tab.label}
            </button>
          );
        })}
      </div>
    </div>
  );
}