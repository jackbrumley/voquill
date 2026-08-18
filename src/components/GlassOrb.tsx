import type { ComponentChildren } from 'preact';

interface GlassOrbProps {
  size?: number;
  children?: ComponentChildren;
}

export function GlassOrb({ size = 120, children }: GlassOrbProps) {
  const auraSize = Math.round(size * 1.7);

  return (
    <div style={{
      position: 'relative',
      display: 'flex',
      alignItems: 'center',
      justifyContent: 'center',
      width: `${size}px`,
      height: `${size}px`,
    }}>
      <style>{`
        @keyframes glass-orb-pulse {
          0%, 100% { opacity: 0; }
          50% { opacity: 0.35; }
        }
        @keyframes voquill-aura-breathe {
          0%, 100% { transform: translate(-50%, -50%) scale(1); opacity: 0.75; }
          50% { transform: translate(-50%, -50%) scale(1.06); opacity: 1; }
        }
      `}</style>

      {/* Atmospheric gradient radiation centered perfectly behind the orb */}
      <div style={{
        position: 'absolute',
        top: '50%',
        left: '50%',
        transform: 'translate(-50%, -50%)',
        width: `${auraSize}px`,
        height: `${auraSize}px`,
        borderRadius: '50%',
        background: 'radial-gradient(circle, rgba(196, 57, 145, 0.24) 0%, rgba(90, 183, 214, 0.18) 38%, rgba(88, 101, 242, 0.08) 55%, transparent 72%)',
        animation: 'voquill-aura-breathe 6s ease-in-out infinite',
        pointerEvents: 'none',
        zIndex: 0,
      }} />

      {/* Main glass orb with concentric integer-aligned gradient border */}
      <div style={{
        width: `${size}px`,
        height: `${size}px`,
        borderRadius: '50%',
        padding: '2px',
        background: 'linear-gradient(135deg, #5ab7d6 0%, #5865f2 45%, #c43991 100%)',
        boxShadow: '0 8px 32px rgba(0, 0, 0, 0.5), 0 0 16px rgba(196, 57, 145, 0.28), 0 0 16px rgba(90, 183, 214, 0.2)',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        position: 'relative',
        zIndex: 1,
        boxSizing: 'border-box',
      }}>
        {/* Inner sphere */}
        <div style={{
          width: '100%',
          height: '100%',
          borderRadius: '50%',
          background: 'radial-gradient(circle at 35% 30%, #2e2050 0%, #1a1d36 50%, #0d1220 100%)',
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          position: 'relative',
          overflow: 'hidden',
          isolation: 'isolate',
        }}>
          <div style={{
            position: 'absolute',
            inset: 0,
            borderRadius: '50%',
            background: 'radial-gradient(circle at 65% 70%, rgba(90, 183, 214, 0.25) 0%, rgba(196, 57, 145, 0.22) 50%, transparent 80%)',
            animation: 'glass-orb-pulse 4s ease-in-out infinite',
            pointerEvents: 'none',
            zIndex: 1,
          }} />
          <div style={{
            position: 'absolute',
            inset: 0,
            borderRadius: '50%',
            background: 'radial-gradient(circle, rgba(0,0,0,0) 65%, rgba(10,12,20,0.3) 90%, rgba(10,12,20,0.55) 100%)',
            pointerEvents: 'none',
            zIndex: 1,
          }} />
          <div style={{ position: 'relative', zIndex: 2, width: '100%', height: '100%', display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
            {children}
          </div>
        </div>
      </div>
    </div>
  );
}