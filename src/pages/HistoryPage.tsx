import { IconCopy, IconPlayerPause, IconPlayerPlay, IconSearch, IconTrash, IconX } from '@tabler/icons-preact';
import { useSignal } from '@preact/signals';
import { invoke } from '@tauri-apps/api/core';
import { tokens } from '../design-tokens.ts';
import type { HistoryItem } from '../types.ts';
import { getSpeakerColor } from '../speakerColors.ts';

interface HistoryPageProps {
  history: HistoryItem[];
  searchQuery: string;
  searchResults: HistoryItem[];
  onCopyToClipboard: (text: string) => void;
  onSearch: (query: string) => void;
  onDelete?: (id: number) => void;
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

function getStatusBadge(status?: string) {
  if (!status || status === 'success') return null;
  if (status === 'failed') {
    return (
      <span
        style={{
          display: 'inline-flex',
          alignItems: 'center',
          padding: '1px 6px',
          borderRadius: '4px',
          fontSize: tokens.typography.sizeXs,
          fontWeight: 700,
          background: 'rgba(255, 80, 80, 0.15)',
          border: '1px solid rgba(255, 80, 80, 0.3)',
          color: '#ff6b6b',
        }}
      >
        Failed
      </span>
    );
  }
  if (status === 'empty') {
    return (
      <span
        style={{
          display: 'inline-flex',
          alignItems: 'center',
          padding: '1px 6px',
          borderRadius: '4px',
          fontSize: tokens.typography.sizeXs,
          fontWeight: 700,
          background: 'rgba(255, 180, 50, 0.15)',
          border: '1px solid rgba(255, 180, 50, 0.3)',
          color: '#ffb432',
        }}
      >
        Empty
      </span>
    );
  }
  if (status === 'cancelled') {
    return (
      <span
        style={{
          display: 'inline-flex',
          alignItems: 'center',
          padding: '1px 6px',
          borderRadius: '4px',
          fontSize: tokens.typography.sizeXs,
          fontWeight: 700,
          background: 'rgba(255, 255, 255, 0.08)',
          border: '1px solid rgba(255, 255, 255, 0.15)',
          color: tokens.colors.textMuted,
        }}
      >
        Cancelled
      </span>
    );
  }
  return null;
}

function renderItemText(item: HistoryItem, searchQuery: string, isRaw: boolean) {
  const text = isRaw && item.raw_text ? item.raw_text : item.text;
  const segments = item.segments;

  if (segments && segments.length > 0 && !isRaw) {
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

  // If text is empty (e.g. failed or empty status)
  if (!text.trim()) {
    const placeholder = item.error_message || (item.status === 'empty' ? 'No speech detected' : 'Transcription failed');
    return (
      <span style={{ color: tokens.colors.textMuted, fontStyle: 'italic' }}>
        {placeholder}
      </span>
    );
  }

  // Fall back to plain text
  if (searchQuery) {
    return highlightText(text, searchQuery).map((seg, i) =>
      seg.highlighted
        ? <span key={i} style={{ background: 'rgba(255, 213, 0, 0.2)', color: '#ffd700', borderRadius: '3px', padding: '0 2px' }}>{seg.text}</span>
        : <span key={i}>{seg.text}</span>
    );
  }
  return text;
}

export function HistoryPage({ history, searchQuery, searchResults, onCopyToClipboard, onSearch, onDelete }: HistoryPageProps) {
  const showRaw = useSignal<Set<number>>(new Set());
  const playingAudioId = useSignal<number | null>(null);
  const audioInstance = useSignal<HTMLAudioElement | null>(null);

  const toggleRaw = (id: number) => {
    const next = new Set(showRaw.value);
    if (next.has(id)) {
      next.delete(id);
    } else {
      next.add(id);
    }
    showRaw.value = next;
  };

  const togglePlayAudio = async (item: HistoryItem) => {
    if (!item.audio_file) return;

    if (playingAudioId.value === item.id) {
      if (audioInstance.value) {
        audioInstance.value.pause();
      }
      playingAudioId.value = null;
      return;
    }

    try {
      if (audioInstance.value) {
        audioInstance.value.pause();
      }
      const rawBytes = await invoke<number[]>('get_history_audio', { fileName: item.audio_file });
      const u8 = new Uint8Array(rawBytes);
      const blob = new Blob([u8], { type: 'audio/wav' });
      const url = URL.createObjectURL(blob);
      const audio = new Audio(url);
      audio.onended = () => {
        playingAudioId.value = null;
      };
      audio.onerror = () => {
        playingAudioId.value = null;
      };
      await audio.play();
      playingAudioId.value = item.id;
      audioInstance.value = audio;
    } catch (err) {
      console.error('Failed to play history audio:', err);
      playingAudioId.value = null;
    }
  };

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
            displayItems.map((item) => {
              const isRaw = showRaw.value.has(item.id);
              const displayText = isRaw && item.raw_text ? item.raw_text : (item.text || item.error_message || '');
              const statusBadge = getStatusBadge(item.status);
              const isPlaying = playingAudioId.value === item.id;
              return (
                <div key={item.id} style={{ borderBottom: '1px solid rgba(255,255,255,0.06)', paddingBottom: '8px' }}>
                  <div style={{ color: '#f1f4f8', fontSize: tokens.typography.sizeSm, lineHeight: 1.45 }}>
                    {renderItemText(item, searchQuery.trim(), isRaw)}
                  </div>
                  <div style={{ display: 'flex', alignItems: 'center', flexWrap: 'wrap', marginTop: '6px', gap: tokens.spacing.xs }}>
                    {statusBadge}
                    {item.source === 'file' && (
                      <span
                        style={{
                          display: 'inline-flex',
                          alignItems: 'center',
                          padding: '1px 5px',
                          borderRadius: '4px',
                          fontSize: tokens.typography.sizeXs,
                          fontWeight: 600,
                          background: 'rgba(255, 255, 255, 0.08)',
                          border: '1px solid rgba(255, 255, 255, 0.15)',
                          color: tokens.colors.textSecondary,
                        }}
                      >
                        File
                      </span>
                    )}
                    <div style={{ fontSize: tokens.typography.sizeXs, color: tokens.colors.textMuted }}>
                      {new Date(item.timestamp).toLocaleString()}
                    </div>
                    {item.duration_secs !== null && item.duration_secs !== undefined && (
                      <div style={{ fontSize: tokens.typography.sizeXs, color: tokens.colors.textMuted }}>
                        • {item.duration_secs.toFixed(1)}s
                      </div>
                    )}
                    {item.language && (
                      <div style={{ fontSize: tokens.typography.sizeXs, color: tokens.colors.textMuted }}>
                        • {item.language}
                      </div>
                    )}
                    {item.engine && (
                      <div style={{ fontSize: tokens.typography.sizeXs, color: tokens.colors.textMuted }}>
                        • {item.engine}
                      </div>
                    )}
                    {item.prompt_name && (
                      <div style={{ fontSize: tokens.typography.sizeXs, color: tokens.colors.textMuted }}>
                        • {item.prompt_name}
                      </div>
                    )}
                    {item.audio_file && (
                      <button
                        onClick={() => void togglePlayAudio(item)}
                        title={isPlaying ? 'Pause audio' : 'Play recorded audio'}
                        style={{
                          padding: '2px 8px',
                          fontSize: tokens.typography.sizeXs,
                          background: isPlaying ? 'rgba(100, 255, 150, 0.2)' : 'rgba(255,255,255,0.06)',
                          border: `1px solid ${isPlaying ? 'rgba(100, 255, 150, 0.4)' : 'rgba(255,255,255,0.1)'}`,
                          borderRadius: '4px',
                          color: isPlaying ? '#64ff96' : tokens.colors.textMuted,
                          cursor: 'pointer',
                          display: 'inline-flex',
                          alignItems: 'center',
                          gap: '4px',
                        }}
                      >
                        {isPlaying ? <IconPlayerPause size={12} /> : <IconPlayerPlay size={12} />}
                        <span>Audio</span>
                      </button>
                    )}
                    {item.raw_text && (
                      <button
                        onClick={() => toggleRaw(item.id)}
                        title={isRaw ? 'Show cleaned' : 'Show original'}
                        style={{
                          padding: '2px 8px',
                          fontSize: tokens.typography.sizeXs,
                          background: isRaw ? 'rgba(100, 200, 255, 0.15)' : 'rgba(255,255,255,0.06)',
                          border: `1px solid ${isRaw ? 'rgba(100, 200, 255, 0.3)' : 'rgba(255,255,255,0.1)'}`,
                          borderRadius: '4px',
                          color: isRaw ? '#64c8ff' : tokens.colors.textMuted,
                          cursor: 'pointer',
                        }}
                      >
                        {isRaw ? 'Cleaned' : 'Original'}
                      </button>
                    )}
                    <div style={{ marginLeft: 'auto', display: 'flex', alignItems: 'center', gap: '2px' }}>
                      <button
                        onClick={() => onCopyToClipboard(displayText)}
                        title="Copy to clipboard"
                        style={{
                          width: '28px',
                          height: '28px',
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
                      {onDelete && (
                        <button
                          onClick={() => onDelete(item.id)}
                          title="Delete entry"
                          style={{
                            width: '28px',
                            height: '28px',
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
                            (e.currentTarget as HTMLElement).style.background = 'rgba(255, 80, 80, 0.15)';
                            (e.currentTarget as HTMLElement).style.color = '#ff6b6b';
                          }}
                          onMouseLeave={(e) => {
                            (e.currentTarget as HTMLElement).style.background = 'transparent';
                            (e.currentTarget as HTMLElement).style.color = '#a9acb5';
                          }}
                        >
                          <IconTrash size={16} />
                        </button>
                      )}
                    </div>
                  </div>
                </div>
              );
            })
          )}
        </div>
      </div>
    </div>
  );
}
