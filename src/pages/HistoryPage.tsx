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
        <div style={{ display: 'flex', flexDirection: 'column', gap: '10px', paddingBottom: '120px' }}>
          {history.length === 0 ? (
            <Card><p style={{ color: tokens.colors.textSecondary }}>No transcriptions yet.</p></Card>
          ) : (
            history.map((item) => (
              <Card key={item.id}>
                <div style={{ color: '#f1f4f8', lineHeight: 1.5 }}>{item.text}</div>
                <Button variant="ghost" size="sm" onClick={() => onCopyToClipboard(item.text)} title="Copy to clipboard" style={{ alignSelf: 'flex-end', marginTop: '8px' }}>
                  <IconCopy size={14} />
                </Button>
                <div style={{ fontSize: tokens.typography.sizeXs, color: tokens.colors.textMuted, marginTop: '6px' }}>{new Date(item.timestamp).toLocaleString()}</div>
              </Card>
            ))
          )}
        </div>
      </div>
    </div>
  );
}
