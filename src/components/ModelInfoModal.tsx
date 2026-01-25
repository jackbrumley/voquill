import { IconX, IconRocket, IconTarget, IconScale, IconCpu, IconBolt } from '@tabler/icons-react';
import { Button } from './Button.tsx';
import { Card } from './Card.tsx';

interface ModelInfoModalProps {
  onClose: () => void;
}

export function ModelInfoModal({ onClose }: ModelInfoModalProps) {
  return (
    <div className="modal-overlay" onClick={() => onClose()}>
      <div onClick={(e: any) => e.stopPropagation()} style={{ width: '100%', maxWidth: '500px' }}>
        <Card className="modal-card">
          <div className="modal-header">
            <h2>Model Guide</h2>
            <Button variant="icon" onClick={() => onClose()} title="Close">
              <IconX size={20} />
            </Button>
          </div>

        <div className="modal-body">
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
                <p><strong>Tiny / Distil-Small</strong>: Best for older laptops. Very fast, uses minimal battery.</p>
              </div>
            </div>

            <div className="model-feature-item recommended">
              <div className="feature-icon balance">
                <IconScale size={24} />
              </div>
              <div className="feature-info">
                <h3>Perfect Balance</h3>
                <p><strong>Base / Distil-Small</strong>: The recommended choice. High accuracy with great speed.</p>
              </div>
            </div>

            <div className="model-feature-item">
              <div className="feature-icon accurate">
                <IconTarget size={24} />
              </div>
              <div className="feature-info">
                <h3>Highest Accuracy</h3>
                <p><strong>Medium</strong>: Best for complex vocabulary or thick accents. Requires a modern PC or GPU.</p>
              </div>
            </div>
          </div>

          <div className="modal-section turbo-section">
            <div className="section-header">
              <IconRocket size={20} className="turbo-icon" />
              <h3>Turbo Mode (GPU)</h3>
            </div>
            <p>
              If you have a dedicated graphics card (AMD or NVIDIA), enabling **Turbo Mode** in Advanced settings will significantly speed up transcription, allowing you to use larger models comfortably.
            </p>
          </div>
        </div>

        <div className="modal-footer">
          <Button variant="primary" onClick={() => onClose()}>Got it</Button>
        </div>
      </Card>
      </div>
    </div>
  );
}
