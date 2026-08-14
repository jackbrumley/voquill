import type { ComponentChildren } from 'preact';

interface GlassOrbProps {
  size?: number;
  children?: ComponentChildren;
}

export function GlassOrb({ size = 120, children }: GlassOrbProps) {
  return (
    <div style={{
      width: `${size}px`,
      height: `${size}px`,
      borderRadius: '50%',
      background: `linear-gradient(135deg, #3a2c3d 0%, #2b2e34 50%, #243942 100%)`,
      boxShadow: '0 6px 24px rgba(0, 0, 0, 0.35), 0 2px 6px rgba(0, 0, 0, 0.25)',
      display: 'flex',
      alignItems: 'center',
      justifyContent: 'center',
      position: 'relative',
      overflow: 'hidden',
      isolation: 'isolate',
    }}>
      <style>{`
        @keyframes glass-orb-pulse {
          0%, 100% { opacity: 0; }
          50% { opacity: 0.35; }
        }
      `}</style>
      <div style={{
        position: 'absolute',
        inset: 0,
        borderRadius: '50%',
        background: `linear-gradient(135deg, #243942 0%, #2b2e34 50%, #3a2c3d 100%)`,
        animation: 'glass-orb-pulse 4s ease-in-out infinite',
        pointerEvents: 'none',
        zIndex: 1,
      }}></div>
      <div style={{
        position: 'absolute',
        inset: 0,
        borderRadius: '50%',
        background: 'radial-gradient(circle, rgba(0,0,0,0) 72%, rgba(47,49,54,0.16) 92%, rgba(47,49,54,0.26) 100%)',
        pointerEvents: 'none',
        zIndex: 1,
      }}></div>
      <div style={{ position: 'relative', zIndex: 2, width: '100%', height: '100%', display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
        {children}
      </div>
    </div>
  );
}