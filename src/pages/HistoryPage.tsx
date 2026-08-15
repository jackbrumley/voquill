import { IconCopy, IconSearch, IconX } from '@tabler/icons-preact';
import { tokens } from '../design-tokens.ts';
import type { HistoryItem } from '../types.ts';
import { getSpeakerColor } from '../speakerColors.ts';

interface HistoryPageProps {
  history: HistoryItem[];
  searchQuery: string;
  searchResults: HistoryItem[];
  onCopyToClipboard: (text: string) => void;
  onSearch: (query: string) => void;
}

interface HighlightSegment {
  text: string;
  highlighted: boolean;
}

function highlightText(text: string, query: string): HighlightSegment[] {
  if (!query.trim()) return [{ text, highlighted: false }];

  const words = query.trim().split(/\s+/).filter(Boolean);
  if (words.length === 0) return [{ text, highlighted: false }];

  const pattern = words.map((w) =>
    w.replace(/[.*+?^${}()|[\]\\]/g, '\\$&'),
  ).join('|');
  const regex = new RegExp(`(${pattern})`, 'gi');

  const segments: HighlightSegment[] = [];
  let lastIndex = 0;
  let match: RegExpExecArray | null;

  while ((match = regex.exec(text)) !== null) {
    if (match.index > lastIndex) {
      segments.push({ text: text.slice(lastIndex, match.index), highlighted: false });
    }
    segments.push({ text: match[0], highlighted: true });
    lastIndex = regex.lastIndex;
  }

  if (lastIndex < text.length) {
    segments.push({ text: text.slice(lastIndex), highlighted: false });
  }

  return segments.length > 0 ? segments : [{ text, highlighted: false }];
}



function renderItemText(item: HistoryItem, searchQuery: string) {
  const segments = item.segments;

  if (segments && segments.length > 0) {
    // Render with speaker labels
    return segments.map((seg, i) => (
      <div key={i} style={{ marginBottom: '2px' }}>
        {seg.speaker && (
          <span style={{ color: getSpeakerColor(seg.speaker), fontWeight: 800, fontSize: tokens.typography.sizeXs }}>
            [{seg.speaker}]
          </span>
        )}
        {' '}
        {searchQuery
          ? highlightText(seg.text, searchQuery).map((hs, j) =>
              hs.highlighted
                ? <span key={j} style={{ background: 'rgba(255, 213, 0, 0.2)', color: '#ffd700', borderRadius: '3px', padding: '0 2px' }}>{hs.text}</span>
                : <span key={j}>{hs.text}</span>
            )
          : seg.text
        }
      </div>
    ));
  }

  // Fall back to plain text
  if (searchQuery) {
    return highlightText(item.text, searchQuery).map((seg, i) =>
      seg.highlighted
        ? <span key={i} style={{ background: 'rgba(255, 213, 0, 0.2)', color: '#ffd700', borderRadius: '3px', padding: '0 2px' }}>{seg.text}</span>
        : <span key={i}>{seg.text}</span>
    );
  }
  return item.text;
}

export function HistoryPage({ history, searchQuery, searchResults, onCopyToClipboard, onSearch }: HistoryPageProps) {
  const displayItems = searchQuery.trim() ? searchResults : history;

  const inputBaseStyle: Record<string, string | number> = {
    width: '100%',
    padding: '8px 12px',
    borderRadius: '8px',
    border: '1px solid rgba(255, 255, 255, 0.1)',
    background: 'rgba(255, 255, 255, 0.04)',
    color: tokens.colors.textPrimary,
    fontSize: tokens.typography.sizeSm,
    fontFamily: tokens.typography.fontMain,
    outline: 'none',
    boxSizing: 'border-box',
  };

  return (
    <div style={{ height: '100%', display: 'flex', flexDirection: 'column', overflow: 'hidden', minHeight: 0 }} key="history">
      <div style={{ padding: '12px 12px 8px' }}>
        <div style={{ maxWidth: '900px', margin: '0 auto' }}>
          <div style={{ position: 'relative' }}>
            <IconSearch
              size={16}
              style={{
                position: 'absolute',
                left: '10px',
                top: '50%',
                transform: 'translateY(-50%)',
                color: tokens.colors.textMuted,
                pointerEvents: 'none',
              }}
            />
            <input
              type="text"
              placeholder="Search transcriptions..."
              value={searchQuery}
              onInput={(e) => onSearch((e.target as HTMLInputElement).value)}
              style={{ ...inputBaseStyle, paddingLeft: '32px', paddingRight: searchQuery ? '32px' : '12px' }}
            />
            {searchQuery && (
              <button
                onClick={() => onSearch('')}
                style={{
                  position: 'absolute',
                  right: '6px',
                  top: '50%',
                  transform: 'translateY(-50%)',
                  background: 'transparent',
                  border: 'none',
                  color: tokens.colors.textMuted,
                  cursor: 'pointer',
                  padding: '4px',
                  display: 'flex',
                  alignItems: 'center',
                  justifyContent: 'center',
                  borderRadius: '4px',
                }}
                title="Clear search"
              >
                <IconX size={16} />
              </button>
            )}
          </div>
        </div>
      </div>

      <div style={{ flex: 1, minHeight: 0, overflowY: 'auto' }}>
        <div style={{ maxWidth: '900px', margin: '0 auto', display: 'flex', flexDirection: 'column', gap: tokens.spacing.sm, padding: '0 12px 48px' }}>
          {displayItems.length === 0 ? (
            <p style={{ color: tokens.colors.textSecondary, fontSize: tokens.typography.sizeSm, marginTop: tokens.spacing.sm }}>
              {searchQuery.trim() ? 'No transcriptions match your search.' : 'No transcriptions yet.'}
            </p>
          ) : (
            displayItems.map((item) => (
              <div key={item.id} style={{ borderBottom: '1px solid rgba(255,255,255,0.06)', paddingBottom: '8px' }}>
                <div style={{ color: '#f1f4f8', fontSize: tokens.typography.sizeSm, lineHeight: 1.45 }}>
                  {renderItemText(item, searchQuery.trim())}
                </div>
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