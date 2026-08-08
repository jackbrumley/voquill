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
  return (
    <div style={{ display: 'flex', justifyContent: 'center', width: '100%' }}>
      <div
        style={{
          display: 'flex',
          width: '100%',
        }}
      >
        {tabs.map((tab) => {
          const isActive = tab.value === active;
          return (
            <button
              key={tab.value}
              type="button"
              style={{
                flex: 1,
                border: 'none',
                background: 'transparent',
                color: isActive ? '#ffffff' : TEXT_INACTIVE,
                fontSize: '16px',
                fontWeight: 400,
                fontFamily: 'system-ui, -apple-system, sans-serif',
                padding: '10px 16px',
                cursor: 'pointer',
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'center',
              }}
              onClick={() => {
                onLogUI(`Tab: ${tab.label}`);
                onNavigate(tab.value);
              }}
              aria-current={isActive ? 'page' : undefined}
            >
              <span style={{
                display: 'inline-block',
                position: 'relative',
                paddingBottom: '8px',
                color: isActive ? '#ffffff' : TEXT_INACTIVE,
                fontSize: '16px',
                fontWeight: 400,
                lineHeight: 1,
                transition: 'color 0.15s ease',
              }}>
                {tab.label}
                <span style={{
                  position: 'absolute',
                  bottom: 0,
                  left: 0,
                  right: 0,
                  height: '2px',
                  background: isActive ? ACCENT : 'transparent',
                  transition: 'background 0.15s ease',
                }} />
              </span>
            </button>
          );
        })}
      </div>
    </div>
  );
}