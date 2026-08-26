import { useSignal, useComputed } from '@preact/signals';
import {
  IconClipboard,
  IconCheck,
  IconAlertTriangle,
  IconFileCode,
} from '@tabler/icons-preact';
import { open } from '@tauri-apps/plugin-dialog';
import { Modal } from '../../../components/Modal.tsx';
import { Button } from '../../../components/Button.tsx';
import type { VoiceMacroCommand } from '../../../types.ts';
import { inputBaseStyle } from '../../../theme/ui-primitives.ts';
import { tokens } from '../../../design-tokens.ts';
import { MacroStepChip } from './MacroStepChip.tsx';
import {
  parseAndValidateMacros,
  sanitizeImportedPhrases,
} from './macroSharing.ts';

interface MacroImportModalProps {
  existingMacros: VoiceMacroCommand[];
  onImport: (macros: VoiceMacroCommand[]) => void;
  onClose: () => void;
}

export function MacroImportModal({
  existingMacros,
  onImport,
  onClose,
}: MacroImportModalProps) {
  const jsonInput = useSignal('');
  const isLoadingFile = useSignal(false);
  const fileError = useSignal<string | null>(null);

  const parseResult = useComputed(() => {
    const raw = jsonInput.value.trim();
    if (!raw) return { valid: [], errors: [] };
    return parseAndValidateMacros(raw);
  });

  const handleOpenFile = async () => {
    fileError.value = null;
    try {
      const selected = await open({
        multiple: false,
        filters: [{ name: 'JSON Macro Files', extensions: ['json'] }],
      });

      if (selected && typeof selected === 'string') {
        isLoadingFile.value = true;
        // Read file using browser fetch on file url or tauri core plugin
        const response = await fetch(
          selected.startsWith('http') ? selected : `asset://${selected}`
        ).catch(async () => {
          // Fallback reading via invoke if asset protocol isn't bound for text
          return null;
        });

        if (response && response.ok) {
          const text = await response.text();
          jsonInput.value = text;
        } else {
          fileError.value = 'Please paste the JSON content directly into the box below.';
        }
      }
    } catch (e) {
      fileError.value = `Could not open file: ${(e as Error).message}`;
    } finally {
      isLoadingFile.value = false;
    }
  };

  const handlePasteFromClipboard = async () => {
    fileError.value = null;
    try {
      const text = await navigator.clipboard.readText();
      if (text && text.trim()) {
        jsonInput.value = text.trim();
      }
    } catch {
      fileError.value = 'Could not access clipboard. Please paste manually into the text area.';
    }
  };

  const handleConfirmImport = () => {
    const valid = parseResult.value.valid;
    if (valid.length === 0) return;

    const sanitized = sanitizeImportedPhrases(valid, existingMacros);
    onImport(sanitized);
  };

  const validCount = parseResult.value.valid.length;
  const errorList = parseResult.value.errors;

  return (
    <Modal
      title="Import Voice Macros"
      onClose={onClose}
      maxWidth="620px"
      footer={
        <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', width: '100%' }}>
          <Button variant="ghost" onClick={onClose} style={{ padding: '6px 12px', fontSize: '12px' }}>
            Cancel
          </Button>

          <Button
            variant="configAction"
            onClick={handleConfirmImport}
            disabled={validCount === 0}
            style={{ display: 'flex', alignItems: 'center', gap: '5px', padding: '6px 14px', fontSize: '12px' }}
          >
            <IconCheck size={14} />
            <span>
              {validCount === 0
                ? 'Import Macros'
                : validCount === 1
                  ? 'Import 1 Macro'
                  : `Import ${validCount} Macros`}
            </span>
          </Button>
        </div>
      }
    >
      <div style={{ display: 'flex', flexDirection: 'column', gap: '10px', minHeight: 0 }}>
        {/* Quick action bar */}
        <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', gap: '8px' }}>
          <span style={{ fontSize: '11px', color: tokens.colors.textMuted, fontWeight: 600, textTransform: 'uppercase', letterSpacing: '0.4px' }}>
            Paste Macro JSON or File Content
          </span>

          <div style={{ display: 'flex', alignItems: 'center', gap: '6px' }}>
            <Button
              variant="ghost"
              onClick={handlePasteFromClipboard}
              style={{ padding: '3px 8px', fontSize: '11px', display: 'flex', alignItems: 'center', gap: '4px' }}
            >
              <IconClipboard size={12} />
              <span>Paste Clipboard</span>
            </Button>
            <Button
              variant="ghost"
              onClick={handleOpenFile}
              disabled={isLoadingFile.value}
              style={{ padding: '3px 8px', fontSize: '11px', display: 'flex', alignItems: 'center', gap: '4px' }}
            >
              <IconFileCode size={12} />
              <span>{isLoadingFile.value ? 'Loading...' : 'Select .json'}</span>
            </Button>
          </div>
        </div>

        {/* Text Area */}
        <textarea
          value={jsonInput.value}
          onInput={(e) => {
            jsonInput.value = (e.target as HTMLTextAreaElement).value;
          }}
          placeholder='Paste shared JSON snippet here (e.g. {"phrase": "whats my hostname", "steps": [...]})'
          rows={6}
          style={{
            ...inputBaseStyle,
            width: '100%',
            fontFamily: 'monospace',
            fontSize: '11.5px',
            lineHeight: 1.4,
            resize: 'vertical',
            minHeight: '100px',
            maxHeight: '200px',
          }}
        />

        {fileError.value && (
          <div
            style={{
              padding: '6px 10px',
              borderRadius: '5px',
              background: 'rgba(239, 68, 68, 0.12)',
              border: '1px solid rgba(239, 68, 68, 0.3)',
              fontSize: '11px',
              color: '#fca5a5',
            }}
          >
            {fileError.value}
          </div>
        )}

        {/* Validation & Preview Summary */}
        {jsonInput.value.trim() && (
          <div
            style={{
              display: 'flex',
              flexDirection: 'column',
              gap: '6px',
              padding: '10px',
              borderRadius: '6px',
              background: 'rgba(0, 0, 0, 0.25)',
              border: '1px solid rgba(255, 255, 255, 0.08)',
              maxHeight: '200px',
              overflowY: 'auto',
            }}
          >
            {/* Header message */}
            <div style={{ display: 'flex', alignItems: 'center', gap: '6px' }}>
              {validCount > 0 ? (
                <>
                  <IconCheck size={14} color="#34d399" />
                  <span style={{ fontSize: '11.5px', fontWeight: 600, color: '#34d399' }}>
                    Found {validCount} valid {validCount === 1 ? 'macro' : 'macros'} to import
                  </span>
                </>
              ) : (
                <>
                  <IconAlertTriangle size={14} color="#f87171" />
                  <span style={{ fontSize: '11.5px', fontWeight: 600, color: '#f87171' }}>
                    No valid macros detected in JSON
                  </span>
                </>
              )}
            </div>

            {/* Error notifications if any */}
            {errorList.length > 0 && (
              <div style={{ display: 'flex', flexDirection: 'column', gap: '2px', marginTop: '2px' }}>
                {errorList.map((err, idx) => (
                  <span key={idx} style={{ fontSize: '10.5px', color: '#fca5a5' }}>
                    • {err}
                  </span>
                ))}
              </div>
            )}

            {/* Preview of valid macros */}
            {validCount > 0 && (
              <div style={{ display: 'flex', flexDirection: 'column', gap: '6px', marginTop: '4px' }}>
                {parseResult.value.valid.map((cmd, idx) => (
                  <div
                    key={idx}
                    style={{
                      display: 'flex',
                      flexDirection: 'column',
                      gap: '4px',
                      padding: '6px 8px',
                      borderRadius: '4px',
                      background: 'rgba(255, 255, 255, 0.03)',
                      border: '1px solid rgba(255, 255, 255, 0.06)',
                    }}
                  >
                    <div style={{ display: 'flex', alignItems: 'center', gap: '6px', flexWrap: 'wrap' }}>
                      <span style={{ fontSize: '12px', fontWeight: 600, color: tokens.colors.textPrimary }}>
                        "{cmd.phrase}"
                      </span>
                      {cmd.phrases && cmd.phrases.length > 0 && (
                        <span style={{ fontSize: '9.5px', color: '#cbd5e1', opacity: 0.8 }}>
                          (+{cmd.phrases.length} aliases)
                        </span>
                      )}
                      <span style={{ fontSize: '10.5px', color: tokens.colors.textMuted }}>
                        • {cmd.steps?.length || 0} steps
                      </span>
                    </div>

                    {cmd.steps && cmd.steps.length > 0 && (
                      <div style={{ display: 'flex', alignItems: 'center', gap: '4px', flexWrap: 'wrap' }}>
                        {cmd.steps.slice(0, 8).map((step, sIdx) => (
                          <MacroStepChip key={sIdx} step={step} />
                        ))}
                        {cmd.steps.length > 8 && (
                          <span style={{ fontSize: '10px', color: tokens.colors.textMuted }}>
                            +{cmd.steps.length - 8} more...
                          </span>
                        )}
                      </div>
                    )}
                  </div>
                ))}
              </div>
            )}
          </div>
        )}
      </div>
    </Modal>
  );
}
