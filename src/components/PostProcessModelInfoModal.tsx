import { IconCpu, IconBolt, IconScale } from '@tabler/icons-preact';
import { Button } from './Button.tsx';
import { Modal } from './Modal.tsx';
import { tokens } from '../design-tokens.ts';

interface PostProcessModelInfoModalProps {
  onClose: () => void;
}

export function PostProcessModelInfoModal({ onClose }: PostProcessModelInfoModalProps) {
  return (
    <Modal
      title="Post-Processing Models"
      onClose={onClose}
      maxWidth="680px"
      footerAlign="center"
      footer={
        <Button variant="primary" pill onClick={onClose} style={{ minWidth: '180px' }}>
          Got it
        </Button>
      }
    >
      <p style={{ fontSize: tokens.typography.sizeMd, color: tokens.colors.textSecondary, lineHeight: 1.6, margin: 0 }}>
        Post-processing uses a local language model to clean up your transcriptions — fixing punctuation, capitalization, and removing filler words. Pick a model below that fits your hardware.
      </p>

      <div style={{ display: 'flex', flexDirection: 'column', gap: tokens.spacing.md }}>
        <div style={{ display: 'flex', gap: tokens.spacing.md, padding: tokens.spacing.md, borderRadius: '12px', border: '1px solid rgba(88, 101, 242, 0.32)', background: 'rgba(49, 54, 82, 0.65)' }}>
          <div style={{ width: '48px', height: '48px', borderRadius: '10px', display: 'flex', alignItems: 'center', justifyContent: 'center', flexShrink: 0, background: '#29413a', color: '#10b981' }}>
            <IconScale size={24} />
          </div>
          <div>
            <h3 style={{ margin: '0 0 4px 0', fontSize: tokens.typography.sizeSm, fontWeight: 700, textTransform: 'uppercase', letterSpacing: '0.5px' }}>Recommended</h3>
            <p style={{ margin: 0, fontSize: tokens.typography.sizeXs, color: tokens.colors.textMuted, lineHeight: 1.5 }}><strong>Qwen 2.5 1.5B</strong>: Best balance of speed and quality. Runs well on modern CPUs (~3-5s per transcription). The recommended choice for most users.</p>
          </div>
        </div>

        <div style={{ display: 'flex', gap: tokens.spacing.md, padding: tokens.spacing.md, background: 'rgba(47, 49, 54, 0.65)', borderRadius: '12px', border: '1px solid rgba(255, 255, 255, 0.08)' }}>
          <div style={{ width: '48px', height: '48px', borderRadius: '10px', display: 'flex', alignItems: 'center', justifyContent: 'center', flexShrink: 0, background: '#3a2f25', color: '#f1c40f' }}>
            <IconBolt size={24} />
          </div>
          <div>
            <h3 style={{ margin: '0 0 4px 0', fontSize: tokens.typography.sizeSm, fontWeight: 700, textTransform: 'uppercase', letterSpacing: '0.5px' }}>Lightweight</h3>
            <p style={{ margin: 0, fontSize: tokens.typography.sizeXs, color: tokens.colors.textMuted, lineHeight: 1.5 }}><strong>Llama 3.2 1B</strong>: Smaller and faster than the recommended model. Great for older hardware or when you need quicker results.</p>
          </div>
        </div>

        <div style={{ display: 'flex', gap: tokens.spacing.md, padding: tokens.spacing.md, background: 'rgba(47, 49, 54, 0.65)', borderRadius: '12px', border: '1px solid rgba(255, 255, 255, 0.08)' }}>
          <div style={{ width: '48px', height: '48px', borderRadius: '10px', display: 'flex', alignItems: 'center', justifyContent: 'center', flexShrink: 0, background: '#2a3344', color: '#5865f2' }}>
            <IconCpu size={24} />
          </div>
          <div>
            <h3 style={{ margin: '0 0 4px 0', fontSize: tokens.typography.sizeSm, fontWeight: 700, textTransform: 'uppercase', letterSpacing: '0.5px' }}>How It Works</h3>
            <p style={{ margin: 0, fontSize: tokens.typography.sizeXs, color: tokens.colors.textMuted, lineHeight: 1.5 }}>After transcription, your text is sent to a local llama-server process running the selected GGUF model. It runs entirely on your machine — no internet needed after download.</p>
          </div>
        </div>
      </div>
    </Modal>
  );
}