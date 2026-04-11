import { IconRocket, IconTarget, IconScale, IconBolt } from '@tabler/icons-preact';
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
      fullScreen
    >
      <p style={{ fontSize: tokens.typography.sizeMd, color: tokens.colors.textSecondary, lineHeight: 1.6, margin: 0 }}>
        Voquill uses AI models to transcribe your voice. Choose the one that best fits your computer's power.
      </p>

      <div style={{ display: 'flex', flexDirection: 'column', gap: tokens.spacing.md }}>
        <div style={{ display: 'flex', gap: tokens.spacing.md, padding: tokens.spacing.md, background: '#2f3136', borderRadius: '12px', border: '1px solid rgba(255, 255, 255, 0.08)' }}>
          <div style={{ width: '48px', height: '48px', borderRadius: '10px', display: 'flex', alignItems: 'center', justifyContent: 'center', flexShrink: 0, background: '#3a2f25', color: '#f1c40f' }}>
            <IconBolt size={24} />
          </div>
          <div>
            <h3 style={{ margin: '0 0 4px 0', fontSize: tokens.typography.sizeSm, fontWeight: 700, textTransform: 'uppercase', letterSpacing: '0.5px' }}>Lightning Fast</h3>
            <p style={{ margin: 0, fontSize: tokens.typography.sizeXs, color: tokens.colors.textMuted, lineHeight: 1.5 }}><strong>Tiny / Distil-Small</strong>: Fastest and lightest, great for older laptops.</p>
          </div>
        </div>

        <div style={{ display: 'flex', gap: tokens.spacing.md, padding: tokens.spacing.md, borderRadius: '12px', border: '1px solid rgba(88, 101, 242, 0.32)', background: '#313652' }}>
          <div style={{ width: '48px', height: '48px', borderRadius: '10px', display: 'flex', alignItems: 'center', justifyContent: 'center', flexShrink: 0, background: '#29413a', color: '#10b981' }}>
            <IconScale size={24} />
          </div>
          <div>
            <h3 style={{ margin: '0 0 4px 0', fontSize: tokens.typography.sizeSm, fontWeight: 700, textTransform: 'uppercase', letterSpacing: '0.5px' }}>Perfect Balance</h3>
            <p style={{ margin: 0, fontSize: tokens.typography.sizeXs, color: tokens.colors.textMuted, lineHeight: 1.5 }}><strong>Distil-Small</strong>: Recommended for most people. Great accuracy with excellent speed.</p>
          </div>
        </div>

        <div style={{ display: 'flex', gap: tokens.spacing.md, padding: tokens.spacing.md, background: '#2f3136', borderRadius: '12px', border: '1px solid rgba(255, 255, 255, 0.08)' }}>
          <div style={{ width: '48px', height: '48px', borderRadius: '10px', display: 'flex', alignItems: 'center', justifyContent: 'center', flexShrink: 0, background: '#2a3344', color: '#5865f2' }}>
            <IconTarget size={24} />
          </div>
          <div>
            <h3 style={{ margin: '0 0 4px 0', fontSize: tokens.typography.sizeSm, fontWeight: 700, textTransform: 'uppercase', letterSpacing: '0.5px' }}>Highest Accuracy</h3>
            <p style={{ margin: 0, fontSize: tokens.typography.sizeXs, color: tokens.colors.textMuted, lineHeight: 1.5 }}><strong>Small / Medium</strong>: Best for complex vocabulary or accents. Requires a modern PC or a GPU.</p>
          </div>
        </div>
      </div>

      <div style={{ padding: tokens.spacing.md, background: '#26282e', borderRadius: '12px', borderLeft: '4px solid #f1c40f' }}>
        <div style={{ display: 'flex', gap: tokens.spacing.sm, alignItems: 'center', marginBottom: '8px' }}>
          <IconRocket size={20} color="#f1c40f" />
          <h3 style={{ margin: 0, fontSize: tokens.typography.sizeSm, fontWeight: 700 }}>Turbo Mode (GPU)</h3>
        </div>
        <p style={{ margin: 0, fontSize: tokens.typography.sizeSm, color: tokens.colors.textSecondary, lineHeight: 1.6 }}>
          If you have a dedicated graphics card (AMD or NVIDIA), try <strong>Turbo Mode</strong> in Experimental settings. It can speed up transcription on some systems, but results vary by hardware and model.
        </p>
      </div>

      <div style={{ display: 'flex', justifyContent: 'center', marginTop: tokens.spacing.sm, paddingBottom: tokens.spacing.md }}>
        <Button variant="primary" pill onClick={onClose} style={{ minWidth: '180px' }}>Got it</Button>
      </div>
    </Modal>
  );
}
