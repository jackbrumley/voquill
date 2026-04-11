import { IconCopy } from '@tabler/icons-preact';
import { Card } from '../components/Card.tsx';
import { Button } from '../components/Button.tsx';
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
        <div style={{ display: 'flex', flexDirection: 'column', gap: tokens.spacing.sm, paddingBottom: '120px' }}>
          {history.length === 0 ? (
            <Card><p style={{ color: tokens.colors.textSecondary }}>No transcriptions yet.</p></Card>
          ) : (
            history.map((item) => (
              <Card
                key={item.id}
                style={{
                  padding: '12px 14px',
                  borderRadius: '10px',
                  border: '1px solid rgba(255, 255, 255, 0.08)',
                  background: 'rgba(255, 255, 255, 0.02)',
                  boxShadow: 'none',
                }}
              >
                <div style={{ color: '#f1f4f8', fontSize: tokens.typography.sizeSm, lineHeight: 1.45 }}>{item.text}</div>
                <div style={{ display: 'flex', alignItems: 'flex-end', marginTop: '8px', gap: tokens.spacing.sm }}>
                  <div style={{ fontSize: tokens.typography.sizeXs, color: tokens.colors.textMuted }}>{new Date(item.timestamp).toLocaleString()}</div>
                  <Button
                    variant="ghost"
                    size="sm"
                    onClick={() => onCopyToClipboard(item.text)}
                    title="Copy to clipboard"
                    style={{ marginLeft: 'auto', width: '32px', height: '32px', padding: '6px' }}
                  >
                    <IconCopy size={14} />
                  </Button>
                </div>
              </Card>
            ))
          )}
        </div>
      </div>
    </div>
  );
}
