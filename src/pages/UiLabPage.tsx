import { useState } from 'preact/hooks';
import { IconAlertCircle, IconMinus, IconX } from '@tabler/icons-preact';
import { ActionFooter } from '../components/ActionFooter.tsx';
import { Button } from '../components/Button.tsx';
import { Card } from '../components/Card.tsx';
import { CollapsibleSection } from '../components/CollapsibleSection.tsx';
import { ConfigField } from '../components/ConfigField.tsx';
import { MicSetupPanel } from '../components/MicSetupPanel.tsx';
import { Modal } from '../components/Modal.tsx';
import { ModeSwitcher } from '../components/ModeSwitcher.tsx';
import { ModelSelectionPanel } from '../components/ModelSelectionPanel.tsx';
import { NumberField } from '../components/NumberField.tsx';
import { SelectField } from '../components/SelectField.tsx';
import { SliderField } from '../components/SliderField.tsx';
import { Switch } from '../components/Switch.tsx';
import StatusIcon from '../StatusIcon.tsx';
import { tokens } from '../design-tokens.ts';
import { inputBaseStyle, selectWrapperStyle, tabPanelContentStyle, tabPanelStyle } from '../theme/ui-primitives.ts';

interface UiLabPageProps {
  appVersion: string;
  onBackToConfig: () => void;
  onOpenUpdateModal: () => void;
}

const sampleModels = [
  {
    size: 'distil-small.en',
    label: 'Distil Small',
    recommended: true,
    file_size: 260_000_000,
    description: 'Balanced quality and speed for most hardware.',
    engine: 'Whisper.cpp',
  },
  {
    size: 'base',
    label: 'Base',
    recommended: false,
    file_size: 145_000_000,
    description: 'Lightweight option with faster runtime.',
    engine: 'Whisper.cpp',
  },
];

export function UiLabPage({ appVersion, onBackToConfig, onOpenUpdateModal }: UiLabPageProps) {
  const [showUpdateBadge, setShowUpdateBadge] = useState(true);
  const [updateAvailableCopy, setUpdateAvailableCopy] = useState(true);
  const [isModalOpen, setIsModalOpen] = useState(false);
  const [isCollapsibleOpen, setIsCollapsibleOpen] = useState(true);
  const [isDemoSwitchOn, setIsDemoSwitchOn] = useState(true);
  const [outputMethod, setOutputMethod] = useState<'Typewriter' | 'Clipboard'>('Typewriter');
  const [numberValue, setNumberValue] = useState(12);
  const [sliderValue, setSliderValue] = useState(1.0);
  const [selectValue, setSelectValue] = useState('default');
  const [modelSize, setModelSize] = useState('distil-small.en');
  const [modelStatus, setModelStatus] = useState<Record<string, boolean>>({ 'distil-small.en': true, base: false });
  const [isDownloading, setIsDownloading] = useState(false);
  const [downloadProgress, setDownloadProgress] = useState(0);
  const [micTestStatus, setMicTestStatus] = useState<'idle' | 'recording' | 'playing' | 'processing'>('idle');
  const [micVolume, setMicVolume] = useState(0.75);
  const [activeStatus, setActiveStatus] = useState<'Ready' | 'Recording' | 'Transcribing'>('Ready');

  return (
    <div style={{ ...tabPanelStyle, overflow: 'auto', padding: 0 }}>
      <div style={{ ...tabPanelContentStyle, maxWidth: '100%', margin: 0, gap: tokens.spacing.md, padding: tokens.spacing.sm }}>
        <Card>
          <div style={{ display: 'flex', flexDirection: 'column', gap: tokens.spacing.xs }}>
            <div style={{ fontSize: tokens.typography.sizeLg, fontWeight: 800, color: tokens.colors.textPrimary }}>
              UI Lab (internal)
            </div>
            <div style={{ fontSize: tokens.typography.sizeSm, color: tokens.colors.textSecondary }}>
              Hidden route for visual QA. Open directly with <code>#/ui-lab</code>.
            </div>
            <div style={{ display: 'flex', gap: tokens.spacing.sm, flexWrap: 'wrap', marginTop: tokens.spacing.xs }}>
              <Button variant="ghost" pill onClick={onBackToConfig}>Back to Config</Button>
              <Button variant="primary" pill onClick={() => setIsModalOpen(true)}>Open Internal Modal</Button>
              <Button variant="secondary" pill onClick={onOpenUpdateModal}>Open Real Update Modal</Button>
            </div>
          </div>
        </Card>

        <Card>
          <div style={{ display: 'grid', gap: tokens.spacing.sm }}>
            <div style={{ fontSize: tokens.typography.sizeMd, fontWeight: 700 }}>Status / Update Preview</div>
            <div style={{ display: 'flex', flexWrap: 'wrap', gap: tokens.spacing.sm }}>
              <Button variant="ghost" pill onClick={() => setActiveStatus('Ready')}>Ready</Button>
              <Button variant="ghost" pill onClick={() => setActiveStatus('Recording')}>Recording</Button>
              <Button variant="ghost" pill onClick={() => setActiveStatus('Transcribing')}>Transcribing</Button>
            </div>
            <div style={{ display: 'flex', gap: tokens.spacing.sm, flexWrap: 'wrap' }}>
              <Switch checked={showUpdateBadge} onChange={setShowUpdateBadge} label="Show update badge" />
            </div>
            <div style={{ display: 'flex', gap: tokens.spacing.sm, flexWrap: 'wrap' }}>
              <Switch checked={updateAvailableCopy} onChange={setUpdateAvailableCopy} label="Use update-available copy" />
            </div>
            <div style={{ display: 'flex', alignItems: 'center', gap: tokens.spacing.md, flexWrap: 'wrap' }}>
              <StatusIcon status={activeStatus} size={64} />
              <div style={{ display: 'flex', flexDirection: 'column', gap: tokens.spacing.xs }}>
                <div style={{ fontSize: tokens.typography.sizeXs, color: tokens.colors.textMuted, fontFamily: tokens.typography.fontMono }}>
                  v{appVersion || '1.3.x'}
                </div>
                {showUpdateBadge && (
                  <button
                    type="button"
                    onClick={onOpenUpdateModal}
                    style={{
                      border: '1px solid rgba(255, 255, 255, 0.16)',
                      background: tokens.colors.accentPrimary,
                      cursor: 'pointer',
                      padding: '4px 10px',
                      borderRadius: '999px',
                      color: '#ffffff',
                      fontSize: tokens.typography.sizeXs,
                      fontWeight: 800,
                      letterSpacing: '0.01em',
                      boxShadow: '0 6px 16px rgba(0, 0, 0, 0.3)',
                      display: 'inline-flex',
                      alignItems: 'center',
                      gap: '6px',
                    }}
                  >
                    <IconAlertCircle size={13} stroke={2.2} />
                    <span>Update available</span>
                  </button>
                )}
              </div>
            </div>
          </div>
        </Card>

        <Card>
          <div style={{ display: 'grid', gap: tokens.spacing.sm }}>
            <div style={{ fontSize: tokens.typography.sizeMd, fontWeight: 700 }}>Buttons</div>
            <div style={{ display: 'flex', gap: tokens.spacing.sm, flexWrap: 'wrap', alignItems: 'center' }}>
              <Button variant="primary">Primary</Button>
              <Button variant="secondary">Secondary</Button>
              <Button variant="configAction">Config Action</Button>
              <Button variant="ghost" pill>Ghost Pill</Button>
              <Button variant="danger">Danger</Button>
              <Button variant="icon" title="Icon">i</Button>
              <Button variant="titlebarIcon" title="Minimize"><IconMinus size={14} stroke={2.2} /></Button>
              <Button variant="titlebarClose" title="Close"><IconX size={14} stroke={2.2} /></Button>
            </div>
          </div>
        </Card>

        <Card>
          <div style={{ display: 'grid', gap: tokens.spacing.sm }}>
            <div style={{ fontSize: tokens.typography.sizeMd, fontWeight: 700 }}>Inputs & Selectors</div>

            <ConfigField label="Text Input" description="Base text input style.">
              <input style={inputBaseStyle} value="Demo input" onInput={() => {}} />
            </ConfigField>

            <ConfigField label="Select Field" description="Custom select dropdown.">
              <div style={selectWrapperStyle}>
                <SelectField
                  value={selectValue}
                  onChange={setSelectValue}
                  options={[
                    { value: 'default', label: 'Default device' },
                    { value: 'usb', label: 'USB microphone' },
                    { value: 'virtual', label: 'Virtual sink' },
                  ]}
                  searchable
                  ariaLabel="Demo select"
                />
              </div>
            </ConfigField>

            <ConfigField label="Number Field" description="Direct numeric input.">
              <NumberField value={numberValue} onChange={setNumberValue} min={0} max={200} />
            </ConfigField>

            <ConfigField label="Slider Field" description="Slider plus percentage input.">
              <SliderField value={sliderValue} min={0.1} max={2.0} step={0.05} onChange={setSliderValue} ariaLabel="Demo slider" />
            </ConfigField>

            <ConfigField label="Switch" description="On/off toggle switch.">
              <Switch checked={isDemoSwitchOn} onChange={setIsDemoSwitchOn} label="Demo switch" />
            </ConfigField>

            <ConfigField label="Mode Switcher" description="Typewriter vs clipboard pill switcher.">
              <ModeSwitcher
                value={outputMethod}
                onToggle={setOutputMethod}
                options={[
                  { value: 'Typewriter', label: 'Typewriter', title: 'Typewriter mode' },
                  { value: 'Clipboard', label: 'Clipboard', title: 'Clipboard mode' },
                ]}
              />
            </ConfigField>
          </div>
        </Card>

        <Card>
          <div style={{ display: 'grid', gap: tokens.spacing.sm }}>
            <div style={{ fontSize: tokens.typography.sizeMd, fontWeight: 700 }}>Panels</div>

            <ConfigField label="Mic Setup Panel" description="Mic sensitivity and test controls.">
              <MicSetupPanel
                inputSensitivity={sliderValue}
                onInputSensitivityChange={setSliderValue}
                micTestStatus={micTestStatus}
                micVolume={micVolume}
                onStartMicTest={() => {
                  setMicVolume(0.72);
                  setMicTestStatus('recording');
                }}
                onStopMicTest={() => {
                  setMicTestStatus('playing');
                }}
                onStopMicPlayback={() => {
                  setMicTestStatus('idle');
                }}
              />
            </ConfigField>

            <ConfigField label="Model Selection Panel" description="Model picker, description, and download action.">
              <ModelSelectionPanel
                availableModels={sampleModels}
                localEngine="Whisper.cpp"
                localModelSize={modelSize}
                modelStatus={modelStatus}
                isDownloading={isDownloading}
                downloadProgress={downloadProgress}
                onChangeModel={setModelSize}
                onShowModelGuide={() => {}}
                onDownloadModel={(size) => {
                  setIsDownloading(true);
                  setDownloadProgress(67);
                  setModelStatus((previous) => ({ ...previous, [size]: true }));
                  setIsDownloading(false);
                  setDownloadProgress(0);
                }}
                onRetryModels={() => {}}
              />
            </ConfigField>
          </div>
        </Card>

        <Card>
          <div style={{ display: 'grid', gap: tokens.spacing.sm }}>
            <div style={{ fontSize: tokens.typography.sizeMd, fontWeight: 700 }}>Structural Components</div>
            <CollapsibleSection
              title="Collapsible Section"
              isOpen={isCollapsibleOpen}
              onToggle={() => setIsCollapsibleOpen((prev) => !prev)}
            >
              <div style={{ color: tokens.colors.textSecondary, fontSize: tokens.typography.sizeSm }}>
                This is content inside a collapsible section.
              </div>
            </CollapsibleSection>

            <div style={{ position: 'relative', minHeight: '110px', border: '1px dashed rgba(255,255,255,0.14)', borderRadius: tokens.radii.panel }}>
              <ActionFooter>
                <Button variant="danger" pill floating>Action Footer Button</Button>
              </ActionFooter>
            </div>
          </div>
        </Card>
      </div>

      {isModalOpen && (
        <Modal
          title={updateAvailableCopy ? 'Update Available' : 'Voquill is Up to Date'}
          onClose={() => setIsModalOpen(false)}
          maxWidth="560px"
          footerAlign="center"
          footer={
            <>
              <Button variant="ghost" pill onClick={() => setIsModalOpen(false)}>Later</Button>
              <Button variant="primary" pill onClick={() => setIsModalOpen(false)}>Download Latest</Button>
            </>
          }
        >
          <p style={{ color: tokens.colors.textSecondary, fontSize: tokens.typography.sizeSm }}>
            {updateAvailableCopy
              ? `A newer Voquill version is available. Current: v${appVersion || '1.3.7'} -> Latest: v1.3.8.`
              : `You are on the latest version (v${appVersion || '1.3.7'}).`}
          </p>
          <p style={{ color: tokens.colors.textMuted, fontSize: tokens.typography.sizeXs }}>
            Updates are currently installed manually by downloading the latest release package.
          </p>
        </Modal>
      )}
    </div>
  );
}
