import type { ThemeDefinition } from './types';

export const terminalDark: ThemeDefinition = {
  id: 'terminal-dark',
  displayName: 'Terminal Dark',
  colorScheme: 'dark',
  colors: {
    surface: '#131315',
    surfaceDim: '#131315',
    surfaceBright: '#39393b',
    surfaceContainerLowest: '#0e0e10',
    surfaceContainerLow: '#1b1b1d',
    surfaceContainer: '#1f1f21',
    surfaceContainerHigh: '#2a2a2b',
    surfaceContainerHighest: '#353436',
    onSurface: '#e4e2e4',
    onSurfaceVariant: '#c6c6cd',
    outline: '#909097',
    outlineVariant: '#45464d',
    primary: '#bec6e0',
    onPrimary: '#283044',
    secondary: '#7bd0ff',
    onSecondary: '#00354a',
    error: '#ffb4ab',
    up: '#10B981',
    down: '#EF4444',
    flat: '#64748B',
  },
  glass: {
    fill: 'rgba(14, 14, 16, 0.8)',
    blur: '16px',
    border: '1px solid rgba(255, 255, 255, 0.12)',
    shadow: '0 8px 32px rgba(0, 0, 0, 0.5)',
  },
  fonts: {
    ui: "'Hanken Grotesk', 'Microsoft YaHei UI', sans-serif",
    data: "'JetBrains Mono', monospace",
    dataUsesUi: false,
  },
  baseRadius: 8,
  backgroundGradient:
    'linear-gradient(180deg, rgba(255,255,255,0.03) 0%, transparent 42%)',
  cssClass: 'theme-terminal-dark',
};

export const lightTerminal: ThemeDefinition = {
  id: 'light-terminal',
  displayName: 'Light Terminal',
  colorScheme: 'light',
  colors: {
    surface: '#f7f9fb',
    surfaceDim: '#d8dadc',
    surfaceBright: '#f7f9fb',
    surfaceContainerLowest: '#ffffff',
    surfaceContainerLow: '#f2f4f6',
    surfaceContainer: '#eceef0',
    surfaceContainerHigh: '#e6e8ea',
    surfaceContainerHighest: '#e0e3e5',
    onSurface: '#191c1e',
    onSurfaceVariant: '#45464d',
    outline: '#76777d',
    outlineVariant: '#c6c6cd',
    primary: '#000000',
    onPrimary: '#ffffff',
    secondary: '#0058be',
    onSecondary: '#ffffff',
    error: '#ba1a1a',
    up: '#059669',
    down: '#E11D48',
    flat: '#64748B',
  },
  glass: {
    fill: 'rgba(255, 255, 255, 0.7)',
    blur: '20px',
    border: '1px solid #E2E8F0',
    innerHighlight: 'inset 0 1px 0 rgba(255,255,255,0.5)',
    shadow:
      '0 10px 15px -3px rgba(15, 23, 42, 0.05), 0 4px 6px -2px rgba(15, 23, 42, 0.02)',
  },
  fonts: {
    ui: "'Hanken Grotesk', 'Microsoft YaHei UI', sans-serif",
    data: "'JetBrains Mono', monospace",
    dataUsesUi: false,
  },
  baseRadius: 8,
  backgroundGradient: 'none',
  cssClass: 'theme-light-terminal',
};

export const liquidGlass: ThemeDefinition = {
  id: 'liquid-glass',
  displayName: 'Liquid Glass',
  colorScheme: 'dark',
  colors: {
    surface: '#051424',
    surfaceDim: '#051424',
    surfaceBright: '#2c3a4c',
    surfaceContainerLowest: '#010f1f',
    surfaceContainerLow: '#0d1c2d',
    surfaceContainer: '#122131',
    surfaceContainerHigh: '#1c2b3c',
    surfaceContainerHighest: '#273647',
    onSurface: '#d4e4fa',
    onSurfaceVariant: '#c6c6cd',
    outline: '#909097',
    outlineVariant: '#45464d',
    primary: '#bec6e0',
    onPrimary: '#283044',
    secondary: '#4edea3',
    onSecondary: '#003824',
    error: '#ffb4ab',
    up: '#4edea3',
    down: '#ffb3ad',
    flat: '#909097',
  },
  glass: {
    fill: 'rgba(255, 255, 255, 0.05)',
    blur: '20px',
    border: '1px solid rgba(255, 255, 255, 0.1)',
    shadow: '0 8px 32px rgba(0, 0, 0, 0.3)',
    glow: {
      up: '0 0 10px rgba(78, 222, 163, 0.4)',
      down: '0 0 10px rgba(255, 179, 173, 0.4)',
    },
    noiseOverlay:
      'url("data:image/svg+xml,%3Csvg viewBox=%220 0 256 256%22 xmlns=%22http://www.w3.org/2000/svg%22%3E%3Cfilter id=%22n%22%3E%3CfeTurbulence type=%22fractalNoise%22 baseFrequency=%220.9%22 numOctaves=%224%22 stitchTiles=%22stitch%22/%3E%3C/filter%3E%3Crect width=%22100%25%22 height=%22100%25%22 filter=%22url(%23n)%22 opacity=%220.03%22/%3E%3C/svg%3E")',
  },
  fonts: {
    ui: "'Hanken Grotesk', 'Microsoft YaHei UI', sans-serif",
    data: "'Hanken Grotesk', 'Microsoft YaHei UI', sans-serif",
    dataUsesUi: true,
  },
  baseRadius: 16,
  backgroundGradient:
    'radial-gradient(circle at 50% 0%, #0f172a 0%, #051424 100%)',
  cssClass: 'theme-liquid-glass',
};

export const THEMES: Record<string, ThemeDefinition> = {
  'terminal-dark': terminalDark,
  'light-terminal': lightTerminal,
  'liquid-glass': liquidGlass,
};

export const THEME_OPTIONS: Array<{
  value: string;
  label: string;
  mode: 'system' | ThemeDefinition;
}> = [
  { value: 'terminal-dark', label: 'Terminal Dark', mode: terminalDark },
  { value: 'light-terminal', label: 'Light Terminal', mode: lightTerminal },
  { value: 'liquid-glass', label: 'Liquid Glass', mode: liquidGlass },
  { value: 'system', label: '跟随系统', mode: 'system' },
];
