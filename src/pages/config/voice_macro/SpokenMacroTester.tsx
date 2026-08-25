import { useSignal } from '@preact/signals';
import { IconMicrophone, IconCheck, IconAlertTriangle, IconSparkles } from '@tabler/icons-preact';
import { invoke } from '@tauri-apps/api/core';
import { Button } from '../../../components/Button.tsx';
import type { MacroStep, VoiceMacroCommand } from '../../../types.ts';
import { tokens } from '../../../design-tokens.ts';
import { resolveMacroSteps } from './keyUtils.ts';
import { MacroStepChip } from './MacroStepChip.tsx';

export interface SpokenMacroTestResult {
  transcript: string;
  similarity: number;
  matched: boolean;
  matched_phrase?: string | null;
  matched_command?: VoiceMacroCommand | null;
}

interface SpokenMacroTesterProps {
  showToast?: (message: string, type: 'success' | 'error' | 'info' | 'saved') => void;
}

export function SpokenMacroTester({ showToast }: SpokenMacroTesterProps) {
  const isTesting = useSignal(false);
  const testResult = useSignal<SpokenMacroTestResult | null>(null);

  const handleStartSpokenTest = async () => {
    isTesting.value = true;
    testResult.value = null;
    try {
      const result = await invoke<SpokenMacroTestResult>('test_spoken_voice_macro');
      testResult.value = result;
      if (result.matched) {
        showToast?.(`Matched macro "${result.matched_phrase}"!`, 'success');
      } else if (result.transcript) {
        showToast?.(`Heard "${result.transcript}" (No match)`, 'info');
      } else {
        showToast?.('No speech detected during test.', 'info');
      }
    } catch (e) {
      showToast?.(`Spoken test failed: ${e}`, 'error');
    } finally {
      isTesting.value = false;
    }
  };

  const steps: MacroStep[] = testResult.value?.matched_command
    ? resolveMacroSteps(testResult.value.matched_command)
    : [];

  return (
    <div
      style={{
        display: 'flex',
        flexDirection: 'column',
        gap: tokens.spacing.sm,
        padding: tokens.spacing.sm,
        borderRadius: '8px',
        background: 'rgba(255, 255, 255, 0.03)',
        border: '1px solid rgba(255, 255, 255, 0.08)',
      }}
    >
      <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', flexWrap: 'wrap', gap: '8px' }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: '6px' }}>
          <IconMicrophone size={16} color="#9ba5ff" />
          <span style={{ fontSize: tokens.typography.sizeSm, color: tokens.colors.textPrimary, fontWeight: 600 }}>
            Test Spoken Recognition
          </span>
        </div>

        <Button
          variant="configAction"
          onClick={handleStartSpokenTest}
          disabled={isTesting.value}
          style={{ display: 'flex', alignItems: 'center', gap: '6px' }}
        >
          {isTesting.value ? (
            <span>Listening (Speak now)...</span>
          ) : (
            <>
              <IconMicrophone size={14} />
              <span>Speak to Test</span>
            </>
          )}
        </Button>
      </div>

      {testResult.value && (
        <div
          style={{
            padding: '10px',
            borderRadius: '6px',
            background: testResult.value.matched
              ? 'rgba(34, 197, 94, 0.1)'
              : 'rgba(234, 179, 8, 0.1)',
            border: testResult.value.matched
              ? '1px solid rgba(34, 197, 94, 0.3)'
              : '1px solid rgba(234, 179, 8, 0.3)',
            display: 'flex',
            flexDirection: 'column',
            gap: '6px',
          }}
        >
          <div style={{ display: 'flex', alignItems: 'center', gap: '6px' }}>
            {testResult.value.matched ? (
              <IconCheck size={16} color="#22c55e" />
            ) : (
              <IconAlertTriangle size={16} color="#eab308" />
            )}
            <span
              style={{
                fontSize: '12px',
                fontWeight: 600,
                color: testResult.value.matched ? '#22c55e' : '#fde047',
              }}
            >
              {testResult.value.matched
                ? `Matched: "${testResult.value.matched_phrase}" (${Math.round(testResult.value.similarity * 100)}% match)`
                : 'No Macro Matched'}
            </span>
          </div>

          <div style={{ fontSize: '12px', color: tokens.colors.textSecondary }}>
            <strong style={{ color: tokens.colors.textPrimary }}>Voquill Heard:</strong>{' '}
            <em>"{testResult.value.transcript || '(silence)'}"</em>
          </div>

          {steps.length > 0 && (
            <div style={{ display: 'flex', alignItems: 'center', gap: '4px', flexWrap: 'wrap', marginTop: '4px' }}>
              <span style={{ fontSize: '11px', color: tokens.colors.textMuted }}>Mapped Action:</span>
              {steps.map((step, idx) => (
                <div key={idx} style={{ display: 'inline-flex', alignItems: 'center', gap: '4px' }}>
                  <MacroStepChip step={step} />
                  {idx < steps.length - 1 && (
                    <span style={{ color: tokens.colors.textMuted, fontSize: '9px' }}>➔</span>
                  )}
                </div>
              ))}
            </div>
          )}
        </div>
      )}

      {/* Dictionary Pro-Tip */}
      <div
        style={{
          fontSize: '11px',
          color: tokens.colors.textMuted,
          lineHeight: 1.4,
          display: 'flex',
          alignItems: 'flex-start',
          gap: '6px',
          padding: '4px 0',
        }}
      >
        <IconSparkles size={14} style={{ flexShrink: 0, marginTop: '2px', color: '#f59e0b' }} />
        <span>
          <strong>Dictionary Tip:</strong> If Whisper mishears specific words (e.g., "Coal" instead of "Call" or game terms), adding them to <strong>Settings ➔ Dictionary</strong> primes the speech model to transcribe them with 100% accuracy.
        </span>
      </div>
    </div>
  );
}
