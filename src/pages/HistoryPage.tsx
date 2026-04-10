import { IconCopy } from '@tabler/icons-preact';
import { Card } from '../components/Card.tsx';
import { Button } from '../components/Button.tsx';

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
    <div className="tab-panel page-scroll" key="history">
      <div className="tab-panel-padded">
        <div className="history-list">
          {history.length === 0 ? (
            <Card className="empty-history"><p>No transcriptions yet.</p></Card>
          ) : (
            history.map((item) => (
              <Card key={item.id} className="history-item">
                <div className="history-text">{item.text}</div>
                <Button variant="ghost" size="sm" className="copy-button" onClick={() => onCopyToClipboard(item.text)} title="Copy to clipboard">
                  <IconCopy size={14} />
                </Button>
                <div className="history-timestamp">{new Date(item.timestamp).toLocaleString()}</div>
              </Card>
            ))
          )}
        </div>
      </div>
    </div>
  );
}
