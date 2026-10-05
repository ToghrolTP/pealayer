export interface AppearanceState {
  theme: 'system' | 'light' | 'dark';
  color_palette: 'studio' | 'native';
  accent_color: string;
  custom_accent_color?: string | null;
  resolved_theme?: 'light' | 'dark';
  resolved_accent?: string;
}

/** Only preference fields enter config patches; host resolution is live state. */
export function mergeAppearance<T extends Record<string, any>>(
  values: T,
  appearance: AppearanceState,
): T {
  return {
    ...values,
    theme: appearance.theme,
    color_palette: appearance.color_palette,
    accent_color: appearance.accent_color,
    custom_accent_color: appearance.custom_accent_color ?? null,
  };
}

export function resolvedAccent(
  runtime: { accentColor: string } | null,
  config: Record<string, any> | null,
  appearance?: AppearanceState,
): string {
  switch (config?.accent_color) {
    case 'pealayer_green': return '#38d27a';
    case 'windows_blue': return '#0078d4';
    case 'macos_blue': return '#0a84ff';
    case 'custom': return /^#[0-9a-f]{6}$/i.test(config?.custom_accent_color ?? '')
      ? config!.custom_accent_color
      : (runtime?.accentColor ?? '#0078d4');
    default: return appearance?.resolved_accent ?? runtime?.accentColor ?? '#0078d4';
  }
}

export function resolvedAppearanceTheme(
  preference: 'system' | 'light' | 'dark',
  browserIsLight: boolean,
  appearance?: AppearanceState,
): 'light' | 'dark' {
  if (preference !== 'system') return preference;
  // Connected clients follow the host. Browser OS preference is offline only.
  return appearance?.resolved_theme ?? (browserIsLight ? 'light' : 'dark');
}

export function accentForeground(accent: string): string {
  const match = /^#([0-9a-f]{2})([0-9a-f]{2})([0-9a-f]{2})$/i.exec(accent);
  if (!match) return '#ffffff';
  const [red, green, blue] = match.slice(1).map((value) => Number.parseInt(value, 16));
  return (red * 299 + green * 587 + blue * 114) > 150_000 ? '#141414' : '#ffffff';
}

export function paletteName(config: Record<string, any> | null): 'native' | 'studio' {
  return config?.color_palette === 'studio' ? 'studio' : 'native';
}
