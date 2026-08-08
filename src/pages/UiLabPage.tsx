import { useSignal } from '@preact/signals';
import { ActionFooter } from '../components/ActionFooter.tsx';
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
  { size: 'distil-small.en', label: 'Distil Small', recommended: true, file_size: 260_000_000, description: 'Balanced quality and speed for most hardware.', engine: 'Whisper.cpp', download_url: '', sha256: '' },
  { size: 'base', label: 'Base', recommended: false, file_size: 145_000_000, description: 'Lightweight option with faster runtime.', engine: 'Whisper.cpp', download_url: '', sha256: '' },
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
          <Switch checked={showUpdateBadge.value} onChange={(v) => { showUpdateBadge.value = v; }} />
        </ConfigField>

        <ConfigField label="Status Page" description="Live preview of the Status page inside a SurfaceCard.">
          <SurfaceCard>
            <StatusPage
              currentStatus={activeStatus.value}
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
              label="Quantity"
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
            label="Sensitivity"
            displayValue={sliderValue.value.toFixed(1)}
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
          <Switch checked={isDemoSwitchOn.value} onChange={(v) => { isDemoSwitchOn.value = v; }} />
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
      </div>

      <ActionFooter>
        <p style={{ color: tokens.colors.textSecondary, fontSize: tokens.typography.sizeXs }}>
          UI Lab lets you test components in isolation.
        </p>
      </ActionFooter>
    </div>
  );
}