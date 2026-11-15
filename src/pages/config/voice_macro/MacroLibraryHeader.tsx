import { useEffect } from 'preact/hooks';
import { useSignal } from '@preact/signals';
import { IconAdjustmentsHorizontal, IconDotsVertical, IconDownload, IconMicrophone, IconPlus, IconUpload } from '@tabler/icons-preact';
import { Button } from '../../../components/Button.tsx';
import { tokens } from '../../../design-tokens.ts';

interface MacroLibraryHeaderProps {
  macroCount: number;
  isTesterOpen: boolean;
  onToggleTester: () => void;
  onOpenVoiceStudio: () => void;
  onCreate: () => void;
  onImport: () => void;
  onExportAll: () => void;
}

const dropdownItemStyle = {
  display: 'flex',
  alignItems: 'center',
  gap: '8px',
  padding: '7px 10px',
  borderRadius: '6px',
  background: 'transparent',
  border: 'none',
  color: tokens.colors.textPrimary,
  fontSize: '11.5px',
  fontWeight: 500,
  cursor: 'pointer',
  width: '100%',
  textAlign: 'left' as const,
  transition: tokens.transitions.fast,
};

const highlight = (event: MouseEvent, isOn: boolean) => {
  (event.currentTarget as HTMLButtonElement).style.background = isOn ? 'rgba(255, 255, 255, 0.08)' : 'transparent';
};

/// The macro list's title row: macro count, Test, New, and a menu with
/// Voice Studio, import and export. Owns the menu's open state.
export function MacroLibraryHeader({ macroCount, isTesterOpen, onToggleTester, onOpenVoiceStudio, onCreate, onImport, onExportAll }: MacroLibraryHeaderProps) {
  const isMenuOpen = useSignal(false);
  const canExport = macroCount > 0;

  useEffect(() => {
    if (!isMenuOpen.value) return;
    const close = () => {
      isMenuOpen.value = false;
    };
    const closeOnEscape = (event: KeyboardEvent) => {
      if (event.key === 'Escape') close();
    };
    window.addEventListener('click', close);
    window.addEventListener('keydown', closeOnEscape);
    return () => {
      window.removeEventListener('click', close);
      window.removeEventListener('keydown', closeOnEscape);
    };
  }, [isMenuOpen.value]);

  return (
    <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', gap: '8px', position: 'relative' }}>
      <span style={{ fontSize: '13px', fontWeight: 600, color: tokens.colors.textSecondary }}>
        Your macros ({macroCount})
      </span>

      <div style={{ display: 'flex', alignItems: 'center', gap: '6px', position: 'relative' }}>
        <Button
          variant={isTesterOpen ? 'secondary' : 'ghost'}
          size="sm"
          onClick={onToggleTester}
          title="Say a phrase and see which macro it would run"
        >
          <IconMicrophone size={13} />
          <span>Test</span>
        </Button>

        <Button variant="configAction" size="sm" onClick={onCreate} title="Create a new voice macro" style={{ padding: '5px 10px' }}>
          <IconPlus size={14} />
          <span>New</span>
        </Button>

        <div style={{ position: 'relative' }}>
          <Button
            variant="ghost"
            size="sm"
            onClick={(event) => {
              event.stopPropagation();
              isMenuOpen.value = !isMenuOpen.value;
            }}
            title="More: Voice Studio, import and export"
            style={{ padding: '5px 8px', minWidth: '28px' }}
          >
            <IconDotsVertical size={14} />
          </Button>

          {isMenuOpen.value && (
            <div
              onClick={(event) => event.stopPropagation()}
              style={{
                position: 'absolute',
                top: 'calc(100% + 4px)',
                right: 0,
                zIndex: 100,
                minWidth: '160px',
                background: 'rgba(26, 29, 38, 0.95)',
                backdropFilter: 'blur(16px)',
                WebkitBackdropFilter: 'blur(16px)',
                border: '1px solid rgba(255, 255, 255, 0.12)',
                borderRadius: '10px',
                padding: '4px',
                boxShadow: tokens.shadows.lg,
                display: 'flex',
                flexDirection: 'column',
                gap: '2px',
              }}
            >
              <button
                type="button"
                onClick={() => {
                  isMenuOpen.value = false;
                  onOpenVoiceStudio();
                }}
                style={dropdownItemStyle}
                onMouseEnter={(event) => highlight(event, true)}
                onMouseLeave={(event) => highlight(event, false)}
              >
                <IconAdjustmentsHorizontal size={13} />
                <span>Voice Studio...</span>
              </button>

              <button
                type="button"
                onClick={() => {
                  isMenuOpen.value = false;
                  onImport();
                }}
                style={dropdownItemStyle}
                onMouseEnter={(event) => highlight(event, true)}
                onMouseLeave={(event) => highlight(event, false)}
              >
                <IconUpload size={13} />
                <span>Import Macros...</span>
              </button>

              <button
                type="button"
                onClick={() => {
                  isMenuOpen.value = false;
                  onExportAll();
                }}
                disabled={!canExport}
                style={{ ...dropdownItemStyle, opacity: canExport ? 1 : 0.4, cursor: canExport ? 'pointer' : 'not-allowed' }}
                onMouseEnter={(event) => canExport && highlight(event, true)}
                onMouseLeave={(event) => highlight(event, false)}
              >
                <IconDownload size={13} />
                <span>Export All ({macroCount})</span>
              </button>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
