import { IconRocket, IconTarget, IconScale, IconBolt } from '@tabler/icons-preact';
import { Button } from './Button.tsx';
import { Modal } from './Modal.tsx';

interface ModelInfoModalProps {
  onClose: () => void;
}

export function ModelInfoModal({ onClose }: ModelInfoModalProps) {
  return (
    <Modal
      title="Model Guide"
      onClose={onClose}
      maxWidth="500px"
      footer={<Button variant="primary" onClick={onClose}>Got it</Button>}
    >
      <p className="modal-intro">
        Voquill uses AI models to transcribe your voice. Choose the one that best fits your computer's power.
      </p>

      <div className="model-feature-list">
        <div className="model-feature-item">
          <div className="feature-icon fast">
            <IconBolt size={24} />
          </div>
          <div className="feature-info">
            <h3>Lightning Fast</h3>
            <p><strong>Tiny / Distil-Small</strong>: Fastest and lightest, great for older laptops.</p>
          </div>
        </div>

        <div className="model-feature-item recommended">
          <div className="feature-icon balance">
            <IconScale size={24} />
          </div>
          <div className="feature-info">
            <h3>Perfect Balance</h3>
            <p><strong>Distil-Small</strong>: Recommended for most people. Great accuracy with excellent speed.</p>
          </div>
        </div>

        <div className="model-feature-item">
          <div className="feature-icon accurate">
            <IconTarget size={24} />
          </div>
          <div className="feature-info">
            <h3>Highest Accuracy</h3>
            <p><strong>Small / Medium</strong>: Best for complex vocabulary or accents. Requires a modern PC or a GPU.</p>
          </div>
        </div>
      </div>

      <div className="modal-section turbo-section">
        <div className="section-header">
          <IconRocket size={20} className="turbo-icon" />
          <h3>Turbo Mode (GPU)</h3>
        </div>
        <p>
          If you have a dedicated graphics card (AMD or NVIDIA), enabling **Turbo Mode** in Advanced settings will dramatically speed up transcription. With Turbo Mode, the **Small** model is a great accuracy upgrade while still feeling fast.
        </p>
      </div>
    </Modal>
  );
}
