import type { JSX } from 'preact';
import { useRef } from 'preact/hooks';
import { useSignal, useSignalEffect, useComputed } from '@preact/signals';
import { IconCheck, IconChevronDown } from '@tabler/icons-preact';
import { tokens } from '../design-tokens.ts';

export interface SelectOption {
  value: string;
  label: string;
  disabled?: boolean;
  searchText?: string;
}

interface SelectFieldProps {
  value: string;
  options: SelectOption[];
  onChange: (value: string) => void;
  placeholder?: string;
  disabled?: boolean;
  searchable?: boolean;
  searchPlaceholder?: string;
  emptyMessage?: string;
  className?: string;
  style?: JSX.CSSProperties;
  ariaLabel?: string;
}

export function SelectField({
  value,
  options,
  onChange,
  placeholder = 'Select an option',
  disabled = false,
  searchable = false,
  searchPlaceholder = 'Search...',
  emptyMessage = 'No options found',
  className = '',
  style,
  ariaLabel,
}: SelectFieldProps) {
  const isOpen = useSignal(false);
  const searchQuery = useSignal('');
  const highlightedIndex = useSignal(-1);
  const isTriggerHovered = useSignal(false);
  const isTriggerFocused = useSignal(false);

  const containerRef = useRef<HTMLDivElement | null>(null);
  const triggerRef = useRef<HTMLButtonElement | null>(null);
  const searchInputRef = useRef<HTMLInputElement | null>(null);
  const listboxIdRef = useRef(`voquill-select-listbox-${Math.random().toString(36).slice(2, 11)}`);

  const selectedOption = options.find((option) => option.value === value) || null;

  const filteredOptions = useComputed(() => {
    if (!searchable) {
      return options;
    }

    const query = searchQuery.value.trim().toLowerCase();
    if (!query) {
      return options;
    }

    return options.filter((option) => {
      const searchPool = `${option.label} ${option.value} ${option.searchText || ''}`.toLowerCase();
      return searchPool.includes(query);
    });
  });

  const findNextEnabledIndex = (startIndex: number, direction: 1 | -1) => {
    const filtered = filteredOptions.value;
    if (filtered.length === 0 || filtered.every((option) => option.disabled)) {
      return -1;
    }

    let index = startIndex;
    for (let step = 0; step < filtered.length; step += 1) {
      index = (index + direction + filtered.length) % filtered.length;
      if (!filtered[index].disabled) {
        return index;
      }
    }

    return -1;
  };

  const closeDropdown = (focusTrigger: boolean) => {
    isOpen.value = false;
    searchQuery.value = '';
    highlightedIndex.value = -1;
    if (focusTrigger) {
      triggerRef.current?.focus();
    }
  };

  const openDropdown = () => {
    if (disabled) {
      return;
    }
    isOpen.value = true;
  };

  const selectOption = (optionValue: string) => {
    const option = options.find((candidate) => candidate.value === optionValue);
    if (!option || option.disabled) {
      return;
    }
    onChange(optionValue);
    closeDropdown(true);
  };

  // Outside click handler
  useSignalEffect(() => {
    if (!isOpen.value) return;

    const handleOutsidePointer = (event: PointerEvent) => {
      const target = event.target as Node | null;
      if (!target || !containerRef.current?.contains(target)) {
        closeDropdown(false);
      }
    };

    window.addEventListener('pointerdown', handleOutsidePointer);
    return () => {
      window.removeEventListener('pointerdown', handleOutsidePointer);
    };
  });

  // Focus search input on open, set initial highlight
  useSignalEffect(() => {
    if (!isOpen.value) return;

    if (searchable) {
      requestAnimationFrame(() => {
        searchInputRef.current?.focus();
      });
      return;
    }

    const selectedIndex = filteredOptions.value.findIndex((o) => o.value === value && !o.disabled);
    if (selectedIndex >= 0) {
      highlightedIndex.value = selectedIndex;
      return;
    }

    highlightedIndex.value = findNextEnabledIndex(-1, 1);
  });

  // Re-highlight on search query change
  useSignalEffect(() => {
    if (!isOpen.value || !searchable) return;

    const selectedIndex = filteredOptions.value.findIndex((o) => o.value === value && !o.disabled);
    if (selectedIndex >= 0) {
      highlightedIndex.value = selectedIndex;
      return;
    }

    highlightedIndex.value = findNextEnabledIndex(-1, 1);
  });

  // Scroll highlighted option into view
  useSignalEffect(() => {
    if (!isOpen.value || highlightedIndex.value < 0) return;

    const el = containerRef.current?.querySelector<HTMLButtonElement>(`[data-option-index="${highlightedIndex.value}"]`);
    el?.scrollIntoView({ block: 'nearest' });
  });

  const handleKeyDown = (event: KeyboardEvent) => {
    if (disabled) {
      return;
    }

    if (!isOpen.value) {
      if (event.key === 'ArrowDown' || event.key === 'ArrowUp' || event.key === 'Enter' || event.key === ' ') {
        event.preventDefault();
        openDropdown();
      }
      return;
    }

    if (event.key === 'Escape') {
      event.preventDefault();
      closeDropdown(true);
      return;
    }

    if (event.key === 'Tab') {
      closeDropdown(false);
      return;
    }

    if (event.key === 'ArrowDown') {
      event.preventDefault();
      highlightedIndex.value = findNextEnabledIndex(highlightedIndex.value, 1);
      return;
    }

    if (event.key === 'ArrowUp') {
      event.preventDefault();
      highlightedIndex.value = findNextEnabledIndex(highlightedIndex.value, -1);
      return;
    }

    if (event.key === 'Home') {
      event.preventDefault();
      highlightedIndex.value = findNextEnabledIndex(-1, 1);
      return;
    }

    if (event.key === 'End') {
      event.preventDefault();
      highlightedIndex.value = findNextEnabledIndex(0, -1);
      return;
    }

    if (event.key === 'Enter') {
      event.preventDefault();
      if (highlightedIndex.value < 0) {
        return;
      }
      const option = filteredOptions.value[highlightedIndex.value];
      if (!option?.disabled) {
        selectOption(option.value);
      }
    }
  };

  const triggerStyle: JSX.CSSProperties = {
    width: '100%',
    background: isTriggerHovered.value && !disabled ? 'rgba(255, 255, 255, 0.08)' : 'rgba(255, 255, 255, 0.05)',
    color: tokens.colors.textPrimary,
    border: `1px solid ${(isOpen.value || isTriggerFocused.value) ? tokens.colors.accentPrimary : 'rgba(255, 255, 255, 0.1)'}`,
    borderRadius: tokens.radii.input,
    padding: '10px 12px',
    fontSize: tokens.typography.sizeSm,
    textAlign: 'left',
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'space-between',
    gap: tokens.spacing.sm,
    cursor: disabled ? 'not-allowed' : 'pointer',
    transition: 'border-color 0.2s ease, box-shadow 0.2s ease, background-color 0.2s ease',
    opacity: disabled ? 0.55 : 1,
    boxShadow: isOpen.value || isTriggerFocused.value ? '0 0 0 2px rgba(88, 101, 242, 0.22)' : 'none',
  };

  const menuStyle: JSX.CSSProperties = {
    position: 'absolute',
    top: 'calc(100% + 6px)',
    left: 0,
    width: '100%',
    zIndex: 120,
    border: '1px solid rgba(255, 255, 255, 0.12)',
    borderRadius: '10px',
    background: 'rgba(36, 39, 45, 0.98)',
    boxShadow: '0 14px 26px rgba(0, 0, 0, 0.34)',
    backdropFilter: 'blur(14px)',
    WebkitBackdropFilter: 'blur(14px)',
    overflow: 'hidden',
  };

  const optionBaseStyle: JSX.CSSProperties = {
    width: '100%',
    border: '1px solid transparent',
    borderRadius: '8px',
    background: 'transparent',
    color: tokens.colors.textPrimary,
    padding: '8px 10px',
    fontSize: tokens.typography.sizeSm,
    textAlign: 'left',
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'space-between',
    gap: tokens.spacing.sm,
    cursor: 'pointer',
    transition: 'background-color 0.14s ease, border-color 0.14s ease',
  };

  return (
    <div
      ref={containerRef}
      className={className}
      style={{ position: 'relative', width: '100%', minWidth: 0, flex: '1 1 auto', ...style }}
      onKeyDown={handleKeyDown}
    >
      <button
        ref={triggerRef}
        type="button"
        role="combobox"
        aria-expanded={isOpen.value}
        aria-haspopup="listbox"
        aria-controls={listboxIdRef.current}
        aria-label={ariaLabel}
        disabled={disabled}
        style={triggerStyle}
        onClick={() => {
          if (isOpen.value) {
            closeDropdown(false);
            return;
          }
          openDropdown();
        }}
        onMouseEnter={() => { isTriggerHovered.value = true; }}
        onMouseLeave={() => { isTriggerHovered.value = false; }}
        onFocus={() => { isTriggerFocused.value = true; }}
        onBlur={() => { isTriggerFocused.value = false; }}
      >
        <span
          style={{
            display: 'block',
            flex: 1,
            minWidth: 0,
            whiteSpace: 'nowrap',
            overflow: 'hidden',
            textOverflow: 'ellipsis',
            color: selectedOption ? tokens.colors.textPrimary : tokens.colors.textMuted,
          }}
        >
          {selectedOption?.label || placeholder}
        </span>
        <IconChevronDown
          size={16}
          style={{
            color: tokens.colors.textSecondary,
            flexShrink: 0,
            transform: isOpen.value ? 'rotate(180deg)' : 'rotate(0deg)',
            transition: 'transform 0.2s ease',
          }}
        />
      </button>

      {isOpen.value && (
        <div role="listbox" id={listboxIdRef.current} style={menuStyle}>
          {searchable && (
            <div style={{ padding: tokens.spacing.sm, borderBottom: '1px solid rgba(255, 255, 255, 0.08)' }}>
              <input
                ref={searchInputRef}
                type="text"
                value={searchQuery.value}
                onInput={(event) => { searchQuery.value = (event.target as HTMLInputElement).value; }}
                placeholder={searchPlaceholder}
                style={{
                  width: '100%',
                  background: 'rgba(255, 255, 255, 0.05)',
                  color: tokens.colors.textPrimary,
                  border: '1px solid rgba(255, 255, 255, 0.12)',
                  borderRadius: '8px',
                  padding: '8px 10px',
                  fontSize: tokens.typography.sizeSm,
                  outline: 'none',
                }}
              />
            </div>
          )}

          <div style={{ maxHeight: '260px', overflow: 'auto', padding: '6px' }}>
            {filteredOptions.value.length === 0 ? (
              <div
                style={{
                  color: tokens.colors.textSecondary,
                  fontSize: tokens.typography.sizeSm,
                  textAlign: 'center',
                  padding: '10px 8px',
                }}
              >
                {emptyMessage}
              </div>
            ) : (
              filteredOptions.value.map((option, index) => {
                const isSelected = option.value === value;
                const isHighlighted = index === highlightedIndex.value;
                const isInteractive = !option.disabled;

                const optionStyle: JSX.CSSProperties = {
                  ...optionBaseStyle,
                  cursor: isInteractive ? 'pointer' : 'not-allowed',
                  opacity: isInteractive ? 1 : 0.5,
                  background: isSelected
                    ? 'rgba(88, 101, 242, 0.2)'
                    : isHighlighted && isInteractive
                      ? 'rgba(88, 101, 242, 0.14)'
                      : 'transparent',
                  borderColor: isSelected
                    ? 'rgba(88, 101, 242, 0.52)'
                    : isHighlighted && isInteractive
                      ? 'rgba(88, 101, 242, 0.42)'
                      : 'transparent',
                };

                return (
                  <button
                    key={option.value}
                    type="button"
                    role="option"
                    aria-selected={isSelected}
                    data-option-index={index}
                    disabled={option.disabled}
                    style={optionStyle}
                    onMouseEnter={() => {
                      if (!option.disabled) {
                        highlightedIndex.value = index;
                      }
                    }}
                    onClick={() => selectOption(option.value)}
                  >
                    <span
                      style={{
                        minWidth: 0,
                        overflow: 'hidden',
                        textOverflow: 'ellipsis',
                        whiteSpace: 'nowrap',
                      }}
                    >
                      {option.label}
                    </span>
                    {isSelected && <IconCheck size={14} />}
                  </button>
                );
              })
            )}
          </div>
        </div>
      )}
    </div>
  );
}