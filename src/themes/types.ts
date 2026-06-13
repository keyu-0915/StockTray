export type ThemeId = 'terminal-dark' | 'light-terminal' | 'liquid-glass';

export type ThemeMode =
  | 'system'
  | 'terminal-dark'
  | 'light-terminal'
  | 'liquid-glass';

export interface ThemeColors {
  surface: string;
  surfaceDim: string;
  surfaceBright: string;
  surfaceContainerLowest: string;
  surfaceContainerLow: string;
  surfaceContainer: string;
  surfaceContainerHigh: string;
  surfaceContainerHighest: string;
  onSurface: string;
  onSurfaceVariant: string;
  outline: string;
  outlineVariant: string;
  primary: string;
  onPrimary: string;
  secondary: string;
  onSecondary: string;
  error: string;
  up: string;
  down: string;
  flat: string;
}

export interface ThemeGlass {
  fill: string;
  blur: string;
  border: string;
  innerHighlight?: string;
  shadow?: string;
  glow?: {
    up: string;
    down: string;
  };
  noiseOverlay?: string;
}

export interface ThemeFonts {
  ui: string;
  data: string;
  dataUsesUi: boolean;
}

export interface ThemeDefinition {
  id: ThemeId;
  displayName: string;
  colorScheme: 'dark' | 'light';
  colors: ThemeColors;
  glass: ThemeGlass;
  fonts: ThemeFonts;
  baseRadius: number;
  backgroundGradient: string;
  cssClass: string;
}

export interface ResolvedTheme {
  theme: ThemeDefinition;
  mode: ThemeMode;
  resolution: ThemeId;
}
