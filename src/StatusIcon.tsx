
import './StatusIcon.css';

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

  return (
    <div className={`status-icon-container ${className} ${large ? 'large' : ''}`}>
      <div className={`icon-circle ${getStatusClass(status)}`}>
        {status === 'Ready' ? (
          <div className="ready-dot">
            <span></span>
          </div>
        ) : (
          <span className="status-icon" key={status}>{getStatusIcon(status)}</span>
        )}
      </div>
    </div>
  );
}

export default StatusIcon;
