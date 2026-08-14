import { useSignal } from '@preact/signals';
import { ActionFooter } from '../components/ActionFooter.tsx';
import { AudioWave } from '../components/AudioWave.tsx';
import { BouncingDots } from '../components/BouncingDots.tsx';
import { GlassOrb } from '../components/GlassOrb.tsx';
import { JumpingDot } from '../components/JumpingDot.tsx';
import { ReadySweep } from '../components/ReadySweep.tsx';
import { Button } from '../components/Button.tsx';
import { ConfigField } from '../components/ConfigField.tsx';
import { ModelSelectionPanel } from '../components/ModelSelectionPanel.tsx';
import { Modal } from '../components/Modal.tsx';
import { ModeSwitcher } from '../components/ModeSwitcher.tsx';
import { NumberField } from '../components/NumberField.tsx';
import { SelectField } from '../components/SelectField.tsx';
import { SliderField } from '../components/SliderField.tsx';
import StatusIcon from '../StatusIcon.tsx';
import { StatusPage } from './StatusPage.tsx';
import { SurfaceCard } from '../components/SurfaceCard.tsx';
import { Switch } from '../components/Switch.tsx';
import { CollapsibleSection } from '../components/CollapsibleSection.tsx';
import { tabPanelContentStyle, tabPanelStyle } from '../theme/ui-primitives.ts';
import { tokens } from '../design-tokens.ts';
import type { ModelInfo } from '../types.ts';

const sampleModels: ModelInfo[] = [
  { size: 'distil-small.en', label: 'Distil Small', recommended: true, file_size: 260_000_000, description: 'Balanced quality and speed for most hardware.', engine: 'Whisper.cpp', download_url: '', sha256: '', category: 'transcription' },
  { size: 'base', label: 'Base', recommended: false, file_size: 145_000_000, description: 'Lightweight option with faster runtime.', engine: 'Whisper.cpp', download_url: '', sha256: '', category: 'transcription' },
];

interface UiLabPageProps {
  appVersion: string;
  onBackToSettings: () => void;
  onOpenUpdateModal: () => void;
}

export function UiLabPage({ appVersion, onBackToSettings, onOpenUpdateModal }: UiLabPageProps) {
  const showUpdateBadge = useSignal(true);
  const updateAvailableCopy = useSignal(true);
  const isModalOpen = useSignal(false);
  const isCollapsibleOpen = useSignal(true);
  const isDemoSwitchOn = useSignal(true);
  const outputMethod = useSignal<'Typewriter' | 'Clipboard'>('Typewriter');
  const numberValue = useSignal(12);
  const sliderValue = useSignal(1.0);
  const selectValue = useSignal('default');
  const modelSize = useSignal('distil-small.en');
  const modelStatus = useSignal<Record<string, boolean>>({ 'distil-small.en': true, base: false });
  const isDownloading = useSignal(false);
  const downloadProgress = useSignal(0);
  const activeStatus = useSignal<'Ready' | 'Recording' | 'Transcribing'>('Ready');

  return (
    <div style={{ ...tabPanelStyle, position: 'relative' }}>
      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '12px' }}>
        <h2 style={{ margin: 0, fontSize: tokens.typography.sizeMd, fontWeight: 700 }}>UI Lab</h2>
        <Button variant="ghost" pill onClick={onBackToSettings}>Back to Settings</Button>
      </div>

      <div style={{ ...tabPanelContentStyle, maxWidth: '100%', margin: 0, gap: tokens.spacing.md, padding: tokens.spacing.sm }}>
        <style>{`
              @keyframes voquill-bounce-dot {
                0% { transform: translateY(calc(-1 * var(--jump, 14px))); }
                50% { transform: translateY(var(--jump, 14px)); }
                100% { transform: translateY(calc(-1 * var(--jump, 14px))); }
              }
              @keyframes voquill-sweep {
                0% { width: 12px; left: 0; }
                25% { width: 100%; left: 0; }
                50% { width: 12px; left: calc(100% - 12px); }
                75% { width: 100%; left: 0; }
                100% { width: 12px; left: 0; }
              }
            `}</style>
        <ConfigField label="StatusIcon" description="Live status indicator with animations.">
          <div style={{ display: 'flex', gap: tokens.spacing.sm, alignItems: 'center' }}>
            <StatusIcon status={activeStatus.value} />
            <SelectField
              value={activeStatus.value}
              options={[
                { value: 'Ready', label: 'Ready' },
                { value: 'Recording', label: 'Recording' },
                { value: 'Transcribing', label: 'Transcribing' },
              ]}
              onChange={(v) => { activeStatus.value = v as 'Ready' | 'Recording' | 'Transcribing'; }}
              style={{ width: '180px' }}
            />
          </div>
        </ConfigField>

        <ConfigField label="Modal" description="Demo modal dialog">
          <Button variant="primary" pill onClick={() => { isModalOpen.value = true; }}>Open Modal</Button>
          {isModalOpen.value && (
            <Modal title="Demo Modal" onClose={() => { isModalOpen.value = false; }} maxWidth="400px">
              <p>This is a demo modal for testing purposes.</p>
              <div style={{ marginTop: '16px' }}>
                <Button variant="primary" pill onClick={() => { isModalOpen.value = false; }}>Close</Button>
              </div>
            </Modal>
          )}
        </ConfigField>

        <ConfigField label="Update Badge" description="Show update-available indicator on Status tab.">
          <Switch name="Update Badge" checked={showUpdateBadge.value} onChange={(v) => { showUpdateBadge.value = v; }} />
        </ConfigField>

        <ConfigField label="Status Page" description="Live preview of the Status page inside a SurfaceCard.">
          <SurfaceCard>
            <StatusPage
              appVersion={appVersion}
              modelStatus={modelStatus.value}
              config={{
                hotkey: 'ctrl+shift+space',
                hotkey_mode: 'Toggle',
                output_method: outputMethod.value,
                transcription_mode: 'Local',
                local_model_size: modelSize.value,
              }}
              isSystemManagedShortcut={false}
              onToggleOutputMethod={(method) => { outputMethod.value = method; }}
              hasUpdateAvailable={updateAvailableCopy.value}
              onOpenUpdateModal={onOpenUpdateModal}
            />
          </SurfaceCard>
        </ConfigField>

        <ConfigField label="Model Selection Panel" description="Model picker, description, and download action.">
          <ModelSelectionPanel
            availableModels={sampleModels}
            localEngine="Whisper.cpp"
            localModelSize={modelSize.value}
            modelStatus={modelStatus.value}
            isDownloading={isDownloading.value}
            downloadProgress={downloadProgress.value}
            downloadPhase="downloading"
            onChangeModel={(size) => { modelSize.value = size; }}
            onShowModelGuide={() => {}}
            onDownloadModel={(size) => {
              isDownloading.value = true;
              downloadProgress.value = 67;
              modelStatus.value = { ...modelStatus.value, [size]: true };
              setTimeout(() => {
                isDownloading.value = false;
                downloadProgress.value = 0;
              }, 2000);
            }}
            onRetryModels={() => {}}
          />
        </ConfigField>

        <ConfigField label="SelectField" description="Default dropdown selector.">
          <SelectField
            value={selectValue.value}
            options={[
              { value: 'default', label: 'Default option' },
              { value: 'option1', label: 'First option' },
              { value: 'option2', label: 'Second option' },
              { value: 'option3', label: 'Disabled option', disabled: true },
            ]}
            onChange={(v) => { selectValue.value = v; }}
            placeholder="Choose an option..."
          />
        </ConfigField>

        <ConfigField label="SelectField (searchable)" description="Searchable dropdown.">
          <SelectField
            value={selectValue.value}
            options={[
              { value: 'alpha', label: 'Alpha', searchText: 'a α' },
              { value: 'beta', label: 'Beta', searchText: 'b β' },
              { value: 'gamma', label: 'Gamma', searchText: 'g γ' },
              { value: 'delta', label: 'Delta', searchText: 'd δ' },
              { value: 'epsilon', label: 'Epsilon', searchText: 'e ε' },
              { value: 'zeta', label: 'Zeta', searchText: 'z ζ' },
              { value: 'eta', label: 'Eta', searchText: 'h η' },
              { value: 'theta', label: 'Theta', searchText: 'th θ' },
              { value: 'iota', label: 'Iota', searchText: 'i ι' },
              { value: 'kappa', label: 'Kappa', searchText: 'k κ' },
              { value: 'disabled-kappa', label: 'Kappa (disabled)', searchText: 'disabled', disabled: true },
            ]}
            onChange={(v) => { selectValue.value = v; }}
            placeholder="Search..."
            searchable
            searchPlaceholder="Type Greek letter..."
          />
        </ConfigField>

        <ConfigField label="NumberField" description="Numeric input with min/max bounds.">
          <div style={{ maxWidth: '200px' }}>
            <NumberField
              value={numberValue.value}
              min={0}
              max={100}
              step={1}
              onChange={(v) => { numberValue.value = v; }}
            />
          </div>
        </ConfigField>

        <ConfigField label="SliderField" description="Range slider with label and display value.">
          <SliderField
            value={sliderValue.value}
            min={0}
            max={5}
            step={0.1}
            onChange={(v) => { sliderValue.value = v; }}
          />
        </ConfigField>

        <ConfigField label="ModeSwitcher" description="Toggle between Typewriter and Clipboard modes.">
          <ModeSwitcher
            value={outputMethod.value}
            onToggle={(v) => { outputMethod.value = v; }}
            options={[
              { value: 'Typewriter', label: 'Typewriter', title: 'Typewriter mode' },
              { value: 'Clipboard', label: 'Clipboard', title: 'Clipboard mode' },
            ]}
          />
        </ConfigField>

        <ConfigField label="SurfaceCard" description="Card with glass-morphism surface styling.">
          <SurfaceCard>
            <p style={{ color: tokens.colors.textSecondary, fontSize: tokens.typography.sizeSm }}>
              This is a SurfaceCard component. It can contain any content.
            </p>
          </SurfaceCard>
        </ConfigField>

        <ConfigField label="Buttons" description="All button variants.">
          <div style={{ display: 'flex', gap: tokens.spacing.sm, flexWrap: 'wrap', alignItems: 'center' }}>
            <Button variant="primary" pill>Primary</Button>
            <Button variant="ghost" pill onClick={() => {}}>Ghost</Button>
            <Button variant="secondary" pill>Secondary</Button>
            <Button variant="danger" pill>Danger</Button>
            <Button variant="primary" pill disabled>Disabled</Button>
          </div>
        </ConfigField>

        <ConfigField label="Switch" description="Toggle switch.">
          <Switch name="Demo Switch" checked={isDemoSwitchOn.value} onChange={(v) => { isDemoSwitchOn.value = v; }} />
        </ConfigField>

        <ConfigField label="CollapsibleSection" description="Expandable/collapsible panel.">
          <CollapsibleSection
            title="Expandable section"
            isOpen={isCollapsibleOpen.value}
            onToggle={() => { isCollapsibleOpen.value = !isCollapsibleOpen.value; }}
          >
            <div style={{ padding: tokens.spacing.sm }}>
              <p style={{ color: tokens.colors.textSecondary, fontSize: tokens.typography.sizeSm }}>
                This content can be shown or hidden.
              </p>
            </div>
          </CollapsibleSection>
        </ConfigField>

        <ConfigField label="Toast" description="Toast notification demo.">
          <div style={{ display: 'flex', gap: tokens.spacing.sm, flexWrap: 'wrap' }}>
            <Button variant="primary" pill>Success</Button>
            <Button variant="ghost" pill>Error</Button>
          </div>
        </ConfigField>

        <ConfigField label="Ready Sweep" description="Horizontal sweeping line animation for ready state.">
          <div style={{
            width: '120px',
            height: '120px',
            border: '1px dashed rgba(255,255,255,0.2)',
            borderRadius: '8px',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
          }}>
            <ReadySweep />
          </div>
        </ConfigField>

        <ConfigField label="Jumping Dot" description="Brand-gradient jumping dot for the ready state.">
          <div style={{
            width: '120px',
            height: '120px',
            border: '1px dashed rgba(255,255,255,0.2)',
            borderRadius: '8px',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
          }}>
            <GlassOrb>
              <JumpingDot />
            </GlassOrb>
          </div>
        </ConfigField>

        <ConfigField label="Audio Wave" description="Animated audio visualizer using Voquill brand colors.">
          <div style={{
            width: '120px',
            height: '120px',
            border: '1px dashed rgba(255,255,255,0.2)',
            borderRadius: '8px',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
          }}>
            <GlassOrb>
              <AudioWave containerHeight={120} />
            </GlassOrb>
          </div>
        </ConfigField>

        <ConfigField label="Bouncing Dots" description="Bouncing dots animation for transcribing state.">
          <div style={{
            width: '120px',
            height: '120px',
            border: '1px dashed rgba(255,255,255,0.2)',
            borderRadius: '8px',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
          }}>
            <GlassOrb>
              <BouncingDots />
            </GlassOrb>
          </div>
        </ConfigField>

        <ConfigField label="Status Icon Comparison" description="Compare the old status orb, the new audio wave design, and the overlay pill.">
          <div style={{ display: 'flex', flexDirection: 'column', gap: tokens.spacing.md, width: '100%' }}>
            <div style={{ display: 'flex', gap: tokens.spacing.sm, alignItems: 'center', justifyContent: 'center' }}>
              <SelectField
                value={activeStatus.value}
                options={[
                  { value: 'Ready', label: 'Ready' },
                  { value: 'Recording', label: 'Recording' },
                  { value: 'Transcribing', label: 'Transcribing' },
                ]}
                onChange={(v) => { activeStatus.value = v as 'Ready' | 'Recording' | 'Transcribing'; }}
                style={{ width: '180px' }}
              />
            </div>
<div style={{ display: 'flex', justifyContent: 'center', alignItems: 'center', width: '100%' }}>
              <div style={{ display: 'flex', flexDirection: 'column', alignItems: 'center', gap: tokens.spacing.sm }}>
                <div style={{ fontSize: '11px', fontWeight: 700, textTransform: 'uppercase', letterSpacing: '0.5px', color: tokens.colors.textSecondary }}>Overlay Pill</div>
                <div style={{
                  display: 'inline-flex',
                  alignItems: 'center',
                  gap: '10px',
                  isolation: 'isolate',
                  contain: 'paint',
                  overflow: 'hidden',
                  background: `linear-gradient(135deg, ${tokens.colors.bgGradientWarm} 0%, ${tokens.colors.bgPrimary} 50%, ${tokens.colors.bgGradientCool} 100%)`,
                  border: '1px solid rgba(255, 255, 255, 0.1)',
                  borderRadius: '999px',
                  padding: '6px 12px 6px 8px',
                  minWidth: '194px',
                }}>
                  <div style={{
                    display: 'flex',
                    alignItems: 'center',
                    justifyContent: 'center',
                    gap: '3px',
                    height: '32px',
                    width: '40px',
                  }}>
                    {activeStatus.value === 'Transcribing' ? (
                      <BouncingDots dotSize={8} gap={3} jumpHeight={8} />
                    ) : activeStatus.value === 'Ready' ? (
                      <ReadySweep />
                    ) : (
                      <AudioWave barWidth={4} containerHeight={32} gap={2} />
                    )}
                  </div>
                  <span style={{ color: '#fff', fontFamily: tokens.typography.fontMain, fontSize: '18px', fontWeight: 500, textAlign: 'center', flex: 1, whiteSpace: 'nowrap' }}>
                    {activeStatus.value}
                  </span>
                </div>
              </div>
            </div>
          </div>
        </ConfigField>
      </div>

      <ActionFooter>
        <></>
      </ActionFooter>
    </div>
  );
}