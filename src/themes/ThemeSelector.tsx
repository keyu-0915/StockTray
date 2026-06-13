import React from 'react';
import type { ThemeMode } from './types';
import { terminalDark, lightTerminal, liquidGlass } from './definitions';

const SWATCH_VALUES: Record<string, { up: string; down: string }> = {
  'terminal-dark': { up: terminalDark.colors.up, down: terminalDark.colors.down },
  'light-terminal': { up: lightTerminal.colors.up, down: lightTerminal.colors.down },
  'liquid-glass': { up: liquidGlass.colors.up, down: liquidGlass.colors.down },
  system: { up: terminalDark.colors.up, down: terminalDark.colors.down },
};

interface ThemeSelectorProps {
  value: ThemeMode;
  onChange: (value: ThemeMode) => void;
}

const OPTIONS: Array<{ value: ThemeMode; label: string; sublabel: string }> = [
  { value: 'terminal-dark', label: 'Terminal', sublabel: 'Dark' },
  { value: 'light-terminal', label: 'Light', sublabel: 'Terminal' },
  { value: 'liquid-glass', label: 'Liquid', sublabel: 'Glass' },
  { value: 'system', label: '跟随', sublabel: '系统' },
];

export function ThemeSelector({ value, onChange }: ThemeSelectorProps) {
  return (
    <div className="theme-selector">
      {OPTIONS.map(({ value: optVal, label, sublabel }) => {
        const swatch = SWATCH_VALUES[optVal];
        return (
          <button
            className={value === optVal ? 'active' : ''}
            key={optVal}
            onClick={() => onChange(optVal)}
            type="button"
            title={label}
          >
            <span className="theme-swatch">
              <span style={{ background: swatch.up }} />
              <span style={{ background: swatch.down }} />
            </span>
            <span>
              {label}
              <br />
              {sublabel}
            </span>
          </button>
        );
      })}
    </div>
  );
}
