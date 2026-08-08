import { IconCopy } from '@tabler/icons-preact';
import { tabPanelPaddedStyle, tabPanelStyle } from '../theme/ui-primitives.ts';
import { tokens } from '../design-tokens.ts';

interface HistoryItem {
  id: number;
  text: string;
  timestamp: string;
}

interface HistoryPageProps {
  history: HistoryItem[];
  onCopyToClipboard: (text: string) => void;
}

export function HistoryPage({ history, onCopyToClipboard }: HistoryPageProps) {
  return (
    <div style={{ ...tabPanelStyle, overflow: 'auto' }} key="history">
      <div style={tabPanelPaddedStyle}>
        <div style={{ display: 'flex', flexDirection: 'column', gap: tokens.spacing.sm }}>
          {history.length === 0 ? (
            <p style={{ color: tokens.colors.textSecondary }}>No transcriptions yet.</p>
          ) : (
            history.map((item) => (
              <div key={item.id} style={{ borderBottom: '1px solid rgba(255,255,255,0.06)', paddingBottom: '8px' }}>
                <div style={{ color: '#f1f4f8', fontSize: tokens.typography.sizeSm, lineHeight: 1.45 }}>{item.text}</div>
                <div style={{ display: 'flex', alignItems: 'flex-end', marginTop: '4px', gap: tokens.spacing.sm }}>
                  <div style={{ fontSize: tokens.typography.sizeXs, color: tokens.colors.textMuted }}>{new Date(item.timestamp).toLocaleString()}</div>
                  <button
                    onClick={() => onCopyToClipboard(item.text)}
                    title="Copy to clipboard"
                    style={{
                      marginLeft: 'auto',
                      width: '32px',
                      height: '32px',
                      padding: '0',
                      background: 'transparent',
                      border: 'none',
                      borderRadius: '6px',
                      color: '#a9acb5',
                      cursor: 'pointer',
                      display: 'flex',
                      alignItems: 'center',
                      justifyContent: 'center',
                      transition: 'all 0.15s ease',
                    }}
                    onMouseEnter={(e) => {
                      (e.currentTarget as HTMLElement).style.background = 'rgba(255,255,255,0.1)';
                      (e.currentTarget as HTMLElement).style.color = '#ffffff';
                    }}
                    onMouseLeave={(e) => {
                      (e.currentTarget as HTMLElement).style.background = 'transparent';
                      (e.currentTarget as HTMLElement).style.color = '#a9acb5';
                    }}
                  >
                    <IconCopy size={16} />
                  </button>
                </div>
              </div>
            ))
          )}
        </div>
      </div>
    </div>
  );
}