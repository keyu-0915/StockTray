import React, {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useRef,
  useState,
} from 'react';
import type { ResolvedTheme, ThemeDefinition, ThemeId, ThemeMode } from './types';
import { THEMES } from './definitions';

interface ThemeContextValue {
  resolved: ResolvedTheme;
  theme: ThemeDefinition;
}

const ThemeContext = createContext<ThemeContextValue | null>(null);

function getSystemTheme(): 'dark' | 'light' {
  if (typeof window === 'undefined') return 'dark';
  return window.matchMedia('(prefers-color-scheme: dark)').matches
    ? 'dark'
    : 'light';
}

function resolveThemeId(themeMode: string, systemDark: boolean): ThemeId {
  if (themeMode === 'system') {
    return systemDark ? 'terminal-dark' : 'light-terminal';
  }
  if (themeMode === 'dark' || themeMode === 'terminal-dark') {
    return 'terminal-dark';
  }
  if (themeMode === 'light' || themeMode === 'light-terminal') {
    return 'light-terminal';
  }
  if (themeMode === 'liquid-glass') {
    return 'liquid-glass';
  }
  return 'terminal-dark';
}

function injectCSSVariables(theme: ThemeDefinition) {
  const root = document.documentElement;
  const c = theme.colors;
  const g = theme.glass;
  const f = theme.fonts;

  root.style.setProperty('--theme-surface', c.surface);
  root.style.setProperty('--theme-surface-dim', c.surfaceDim);
  root.style.setProperty('--theme-surface-bright', c.surfaceBright);
  root.style.setProperty('--theme-surface-container-lowest', c.surfaceContainerLowest);
  root.style.setProperty('--theme-surface-container-low', c.surfaceContainerLow);
  root.style.setProperty('--theme-surface-container', c.surfaceContainer);
  root.style.setProperty('--theme-surface-container-high', c.surfaceContainerHigh);
  root.style.setProperty('--theme-surface-container-highest', c.surfaceContainerHighest);
  root.style.setProperty('--theme-on-surface', c.onSurface);
  root.style.setProperty('--theme-on-surface-variant', c.onSurfaceVariant);
  root.style.setProperty('--theme-outline', c.outline);
  root.style.setProperty('--theme-outline-variant', c.outlineVariant);
  root.style.setProperty('--theme-primary', c.primary);
  root.style.setProperty('--theme-on-primary', c.onPrimary);
  root.style.setProperty('--theme-secondary', c.secondary);
  root.style.setProperty('--theme-error', c.error);
  root.style.setProperty('--theme-glass-fill', g.fill);
  root.style.setProperty('--theme-glass-blur', g.blur);
  root.style.setProperty('--theme-glass-border', g.border);
  if (g.innerHighlight) root.style.setProperty('--theme-glass-inner-highlight', g.innerHighlight);
  else root.style.removeProperty('--theme-glass-inner-highlight');
  if (g.noiseOverlay) root.style.setProperty('--theme-glass-noise', g.noiseOverlay);
  else root.style.removeProperty('--theme-glass-noise');
  if (g.shadow) root.style.setProperty('--theme-glass-shadow', g.shadow);
  else root.style.removeProperty('--theme-glass-shadow');
  root.style.setProperty('--theme-font-ui', f.ui);
  root.style.setProperty('--theme-font-data', f.data);
  root.style.setProperty('--theme-base-radius', `${theme.baseRadius}px`);
  if (theme.backgroundGradient) {
    root.style.setProperty('--theme-bg-gradient', theme.backgroundGradient);
  } else {
    root.style.removeProperty('--theme-bg-gradient');
  }

  root.dataset.theme = theme.cssClass;

  root.style.colorScheme = theme.colorScheme;
}

interface ThemeProviderProps {
  themeMode: ThemeMode;
  children: React.ReactNode;
}

export function ThemeProvider({ themeMode, children }: ThemeProviderProps) {
  const [systemDark, setSystemDark] = useState(getSystemTheme);

  useEffect(() => {
    const mq = window.matchMedia('(prefers-color-scheme: dark)');
    const handler = (e: MediaQueryListEvent) => setSystemDark(e.matches ? 'dark' : 'light');
    mq.addEventListener('change', handler);
    return () => mq.removeEventListener('change', handler);
  }, []);

  const themeId = useMemo(
    () => resolveThemeId(themeMode, systemDark === 'dark'),
    [themeMode, systemDark],
  );

  const theme = useMemo(() => THEMES[themeId] ?? THEMES['terminal-dark'], [themeId]);

  const prevThemeIdRef = useRef(themeId);
  useEffect(() => {
    if (prevThemeIdRef.current !== themeId) {
      prevThemeIdRef.current = themeId;
    }
    injectCSSVariables(theme);
  }, [theme, themeId]);

  const value = useMemo(
    () => ({
      resolved: { theme, mode: themeMode, resolution: themeId } as ResolvedTheme,
      theme,
    }),
    [theme, themeMode, themeId],
  );

  return <ThemeContext.Provider value={value}>{children}</ThemeContext.Provider>;
}

export function useTheme(): ThemeContextValue {
  const ctx = useContext(ThemeContext);
  if (!ctx) {
    throw new Error('useTheme() must be used within <ThemeProvider>');
  }
  return ctx;
}
