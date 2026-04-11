
interface StatusIconProps {
  status: string;
  className?: string;
  large?: boolean;
}

function StatusIcon({ status, className = '', large = false }: StatusIconProps) {
  const getStatusIcon = (status: string) => {
    switch (status) {
      case 'Ready':
        return '';
      case 'Recording':
        return '🎤';
      case 'Converting audio':
        return '🔄';
      case 'Transcribing':
        return '🧠';
      case 'Typing':
        return '⌨️';
      default:
        return '📊';
    }
  };

  const getStatusClass = (status: string) => {
    switch (status) {
      case 'Ready':
        return 'status-ready';
      case 'Recording':
        return 'status-recording';
      case 'Converting audio':
      case 'Transcribing':
        return 'status-transcribing';
      case 'Typing':
        return 'status-typing';
      default:
        return '';
    }
  };

  const animationName = status === 'Recording'
    ? 'voquill-status-bounce'
    : status === 'Converting audio' || status === 'Transcribing'
      ? 'voquill-status-spin'
      : status === 'Typing'
        ? 'voquill-status-bounce-fast'
        : 'voquill-status-enter';

  const jumpHeight = large ? '-35px' : '-18px';

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
          width: large ? '110px' : '80px',
          height: large ? '110px' : '80px',
          borderRadius: '50%',
          background: 'radial-gradient(circle, #5ab7d6 0%, rgba(90, 183, 214, 1) 20%, rgba(123, 120, 163, 1) 35%, rgba(156, 90, 136, 1) 50%, rgba(196, 57, 145, 1) 65%, #c43991 100%)',
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          position: 'relative',
          boxShadow: '0 8px 24px rgba(0, 0, 0, 0.3), 0 4px 12px rgba(0, 0, 0, 0.2)',
          backdropFilter: 'blur(8px)',
          border: '1px solid rgba(255, 255, 255, 0.2)',
        }}
      >
        <div
          style={{
            position: 'absolute',
            top: '-4px',
            left: '-4px',
            right: '-4px',
            bottom: '-4px',
            borderRadius: '50%',
            background: 'conic-gradient(from 0deg, #5ab7d6, #c43991, #5ab7d6)',
            animation: 'voquill-ring-rotate 3s linear infinite',
            opacity: 0.6,
            zIndex: 0,
            filter: 'blur(2px)',
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
                width: large ? '20px' : '12px',
                height: large ? '20px' : '12px',
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
              fontSize: large ? '60px' : '48px',
              display: 'inline-block',
              position: 'relative',
              zIndex: 2,
              filter: 'drop-shadow(0 2px 4px rgba(0, 0, 0, 0.3)) drop-shadow(0 1px 2px rgba(0, 0, 0, 0.5))',
              textShadow: '0 1px 3px rgba(0, 0, 0, 0.4)',
              animation: status === 'Typing' ? `${animationName} 1s ease-in-out infinite` : status === 'Recording' ? `${animationName} 1.5s ease-in-out infinite` : status === 'Transcribing' || status === 'Converting audio' ? `${animationName} 2s linear infinite` : `${animationName} 0.15s cubic-bezier(0.4, 0, 0.2, 1) forwards`,
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
