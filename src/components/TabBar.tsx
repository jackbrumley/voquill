import type { AppRoute } from '../types.ts';

const TEXT_INACTIVE = '#c5cbd3';

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
  const activeIndex = tabs.findIndex(t => t.value === active);

  return (
    <div style={{ display: 'flex', justifyContent: 'center', width: '100%' }}>
      <div
        style={{
          display: 'flex',
          width: '100%',
          position: 'relative',
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
                padding: '10px 16px 8px',
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
                color: isActive ? '#ffffff' : TEXT_INACTIVE,
                fontSize: '16px',
                fontWeight: 400,
                lineHeight: 1,
                transition: 'color 0.15s ease',
              }}>
                {tab.label}
              </span>
            </button>
          );
        })}
        <div style={{
          position: 'absolute',
          bottom: 0,
          left: `${((activeIndex + 0.5) / tabs.length) * 100}%`,
          width: '72px',
          height: '3px',
          borderRadius: '2px',
          background: 'linear-gradient(90deg, #5865f2 0%, #c43991 100%)',
          boxShadow: '0 0 10px rgba(196, 57, 145, 0.35)',
          transform: 'translateX(-50%)',
          transition: 'left 0.25s cubic-bezier(0.4, 0, 0.2, 1)',
        }} />
      </div>
    </div>
  );
}