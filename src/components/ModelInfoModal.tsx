import { IconTarget, IconScale, IconBolt } from '@tabler/icons-preact';
import { Button } from './Button.tsx';
import { Modal } from './Modal.tsx';
import { tokens } from '../design-tokens.ts';

interface ModelInfoModalProps {
  onClose: () => void;
}

export function ModelInfoModal({ onClose }: ModelInfoModalProps) {
  return (
    <Modal
      title="Model Guide"
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
        Pick an engine in Settings first, then choose a model size below. The right model depends on your computer and how fast you need results.
      </p>

      <div style={{ display: 'flex', flexDirection: 'column', gap: tokens.spacing.md }}>
        <div style={{ display: 'flex', gap: tokens.spacing.md, padding: tokens.spacing.md, background: 'rgba(47, 49, 54, 0.65)', borderRadius: '12px', border: '1px solid rgba(255, 255, 255, 0.08)' }}>
          <div style={{ width: '48px', height: '48px', borderRadius: '10px', display: 'flex', alignItems: 'center', justifyContent: 'center', flexShrink: 0, background: '#3a2f25', color: '#f1c40f' }}>
            <IconBolt size={24} />
          </div>
          <div>
            <h3 style={{ margin: '0 0 4px 0', fontSize: tokens.typography.sizeSm, fontWeight: 700, textTransform: 'uppercase', letterSpacing: '0.5px' }}>Lightning Fast</h3>
            <p style={{ margin: 0, fontSize: tokens.typography.sizeXs, color: tokens.colors.textMuted, lineHeight: 1.5 }}><strong>Tiny / Distil-Small</strong>: Fastest and lightest, great for older laptops. Works with both CPU and GPU Whisper engines.</p>
          </div>
        </div>

        <div style={{ display: 'flex', gap: tokens.spacing.md, padding: tokens.spacing.md, borderRadius: '12px', border: '1px solid rgba(88, 101, 242, 0.32)', background: 'rgba(49, 54, 82, 0.65)' }}>
          <div style={{ width: '48px', height: '48px', borderRadius: '10px', display: 'flex', alignItems: 'center', justifyContent: 'center', flexShrink: 0, background: '#29413a', color: '#10b981' }}>
            <IconScale size={24} />
          </div>
          <div>
            <h3 style={{ margin: '0 0 4px 0', fontSize: tokens.typography.sizeSm, fontWeight: 700, textTransform: 'uppercase', letterSpacing: '0.5px' }}>Perfect Balance</h3>
            <p style={{ margin: 0, fontSize: tokens.typography.sizeXs, color: tokens.colors.textMuted, lineHeight: 1.5 }}><strong>Distil-Small</strong>: Recommended for most people. Great accuracy with excellent speed.</p>
          </div>
        </div>

        <div style={{ display: 'flex', gap: tokens.spacing.md, padding: tokens.spacing.md, background: 'rgba(47, 49, 54, 0.65)', borderRadius: '12px', border: '1px solid rgba(255, 255, 255, 0.08)' }}>
          <div style={{ width: '48px', height: '48px', borderRadius: '10px', display: 'flex', alignItems: 'center', justifyContent: 'center', flexShrink: 0, background: '#2a3344', color: '#5865f2' }}>
            <IconTarget size={24} />
          </div>
          <div>
            <h3 style={{ margin: '0 0 4px 0', fontSize: tokens.typography.sizeSm, fontWeight: 700, textTransform: 'uppercase', letterSpacing: '0.5px' }}>Highest Accuracy</h3>
            <p style={{ margin: 0, fontSize: tokens.typography.sizeXs, color: tokens.colors.textMuted, lineHeight: 1.5 }}><strong>Small / Medium</strong>: Best for complex vocabulary or accents. Needs a modern PC.</p>
          </div>
        </div>
      </div>
    </Modal>
  );
}