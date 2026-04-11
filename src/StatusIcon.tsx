
interface StatusIconProps {
  status: string;
  className?: string;
  large?: boolean;
  size?: number;
}

function StatusIcon({ status, className = '', large = false, size }: StatusIconProps) {
  const getStatusIcon = (status: string) => {
    switch (status) {
      case 'Ready':
        return '';
      case 'Recording':
        return '🎤';
      case 'Transcribing':
        return '🧠';
      default:
        throw new Error(`Unknown status icon state: ${status}`);
    }
  };

  const getStatusClass = (status: string) => {
    switch (status) {
      case 'Ready':
        return 'status-ready';
      case 'Recording':
        return 'status-recording';
      case 'Transcribing':
        return 'status-transcribing';
      default:
        throw new Error(`Unknown status class state: ${status}`);
    }
  };

  const animationName = status === 'Recording'
    ? 'voquill-status-bounce'
    : status === 'Transcribing'
      ? 'voquill-status-spin'
      : 'voquill-status-enter';

  const orbSize = size ?? (large ? 108 : 80);
  const emojiSize = Math.max(22, Math.round(orbSize * 0.6));
  const readyDotSize = Math.max(10, Math.round(orbSize * 0.18));
  const jumpHeight = `-${Math.max(16, Math.round(orbSize * 0.32))}px`;

  return (
    <div className={className} style={{ display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
      <style>{`
        @keyframes voquill-status-spin { from { transform: rotate(0deg); } to { transform: rotate(360deg); } }
        @keyframes voquill-status-bounce { 0%,20%,53%,80%,100% { transform: translateY(0); } 40%,43% { transform: translateY(-4px); } 70% { transform: translateY(-2px); } 90% { transform: translateY(-1px); } }
        @keyframes voquill-status-bounce-fast { 0%,20%,53%,80%,100% { transform: translateY(0); } 40%,43% { transform: translateY(-4px); } 70% { transform: translateY(-2px); } 90% { transform: translateY(-1px); } }
        @keyframes voquill-status-enter { from { opacity: 0.6; transform: scale(0.92); } to { opacity: 1; transform: scale(1); } }
        @keyframes voquill-ring-rotate { from { transform: rotate(0deg); } to { transform: rotate(360deg); } }
        @keyframes voquill-gradient-pulse { 0%, 100% { opacity: 0; } 50% { opacity: 1; } }
        @keyframes voquill-ready-dot-jump {
          0%, 40%, 100% { transform: translateY(0) scale(1, 1); }
          43% { transform: translateY(0) scale(1.25, 0.75); }
          53% { transform: translateY(var(--jump-height)) scale(0.9, 1.1); }
          63% { transform: translateY(0) scale(1.2, 0.8); }
          68% { transform: translateY(calc(var(--jump-height) / 4)) scale(0.95, 1.05); }
          73% { transform: translateY(0) scale(1.05, 0.95); }
        }
      `}</style>
      <div
        className={getStatusClass(status)}
        style={{
          width: `${orbSize}px`,
          height: `${orbSize}px`,
          borderRadius: '50%',
          background: 'radial-gradient(circle, #5ab7d6 0%, rgba(90, 183, 214, 1) 20%, rgba(123, 120, 163, 1) 35%, rgba(156, 90, 136, 1) 50%, rgba(196, 57, 145, 1) 65%, #c43991 100%)',
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          position: 'relative',
          boxShadow: 'none',
          overflow: 'hidden',
          isolation: 'isolate',
        }}
      >
        <div
          style={{
            position: 'absolute',
            inset: 0,
            borderRadius: '50%',
            background: 'radial-gradient(circle, rgba(0,0,0,0) 72%, rgba(47,49,54,0.16) 92%, rgba(47,49,54,0.26) 100%)',
            pointerEvents: 'none',
            zIndex: 1,
          }}
        ></div>
        <div
          style={{
            position: 'absolute',
            inset: 0,
            borderRadius: '50%',
            background: 'radial-gradient(circle, #c43991 0%, rgba(196, 57, 145, 1) 20%, rgba(156, 90, 136, 1) 35%, rgba(123, 120, 163, 1) 50%, rgba(90, 183, 214, 1) 65%, #5ab7d6 100%)',
            animation: 'voquill-gradient-pulse 4s ease-in-out infinite',
            pointerEvents: 'none',
            zIndex: 1,
          }}
        ></div>
        {status === 'Ready' ? (
          <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'center', transform: 'translateY(calc(var(--jump-height) / -2))', ['--jump-height' as any]: jumpHeight, zIndex: 2 }}>
            <span
              style={{
                width: `${readyDotSize}px`,
                height: `${readyDotSize}px`,
                backgroundColor: '#fff',
                borderRadius: '50%',
                animation: 'voquill-ready-dot-jump 3s infinite ease-in-out',
                ['--jump-height' as any]: jumpHeight,
              }}
            ></span>
          </div>
        ) : (
          <span
            key={status}
            style={{
              fontSize: `${emojiSize}px`,
              display: 'inline-block',
              lineHeight: 1,
              background: 'transparent',
              fontFamily: "'Apple Color Emoji', 'Segoe UI Emoji', 'Noto Color Emoji', 'EmojiOne Color', sans-serif",
              position: 'relative',
              zIndex: 2,
              filter: 'none',
              textShadow: 'none',
              backfaceVisibility: 'hidden',
              WebkitFontSmoothing: 'antialiased',
              animation: status === 'Recording'
                ? `${animationName} 1.5s ease-in-out infinite`
                : status === 'Transcribing'
                  ? `${animationName} 2s linear infinite`
                  : `${animationName} 0.15s cubic-bezier(0.4, 0, 0.2, 1) forwards`,
            }}
          >
            {getStatusIcon(status)}
          </span>
        )}
      </div>
    </div>
  );
}

export default StatusIcon;
