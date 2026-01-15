
import './StatusIcon.css';

interface StatusIconProps {
  status: string;
  className?: string;
}

function StatusIcon({ status, className = '' }: StatusIconProps) {
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
    <div className={`status-icon-container ${className}`}>
      <div className={`icon-circle ${getStatusClass(status)}`}>
        <span className="status-icon">{getStatusIcon(status)}</span>
      </div>
    </div>
  );
}

export default StatusIcon;
