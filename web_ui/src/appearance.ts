export type ColorPalette = 'native' | 'studio' | 'pastel';

export type ControlDensity = 'extra_compact' | 'compact' | 'regular' | 'comfortable' | 'spacious';
export const controlDensityHeight = (choice: string): number => {
  switch (choice) {
    case 'extra_compact': return 18;
    case 'regular': return 28;
    case 'comfortable': return 32;
    case 'spacious': return 36;
    default: return 24;
  }
};

// Ant's default 22px text line otherwise forces inputs above the restored 18px
// height. Scope this to control components, not page text or headings.
export const controlDensityText = (choice: string) => choice === 'extra_compact'
  ? { fontSize: 14, fontSizeLG: 14, lineHeight: (controlDensityHeight(choice) - 2) / 14,
    lineHeightLG: (controlDensityHeight(choice) - 2) / 14 }
  : {};

export interface AppearanceState {
  ui_animation_mode: 'system' | 'enabled' | 'disabled';
  ui_control_density: ControlDensity;
  ui_font_family: 'system' | 'bundled' | 'vazirmatn' | 'custom';
  monospace_font_family: 'system' | 'bundled' | 'custom';
  ui_font_name: string;
  monospace_font_name: string;
  workspace_ui: 'craft_ui' | 'egui_dock';
  preferences_grouped_navigation: boolean;
  theme: 'system' | 'light' | 'dark';
  color_palette: ColorPalette;
  accent_color: string;
  custom_accent_color?: string | null;
  resolved_theme?: 'light' | 'dark';
  resolved_accent?: string;
}
const customFamily = (name: string) => JSON.stringify(name.replace(/[\x00-\x1f\x7f]/g, '')).replace(/</g, '\\3c ');
export const uiFontCss = (choice: string, name = ''): string => choice === 'custom' && name.trim()
  ? `${customFamily(name)}, ${uiFontCss('system')}` : choice === 'vazirmatn' ? 'Vazirmatn, sans-serif'
  : choice === 'bundled' ? 'PealayerUbuntu, Vazirmatn, sans-serif' : "PealayerPersian, system-ui, -apple-system, BlinkMacSystemFont, 'Segoe UI', Vazirmatn, sans-serif";
export const monoFontCss = (choice: string, name = ''): string => choice === 'custom' && name.trim()
  ? `${customFamily(name)}, ${monoFontCss('system')}` : choice === 'bundled' ? 'PealayerHack, monospace' : 'ui-monospace, Consolas, Menlo, monospace';

export const statusFontCss = (choice: string, name = '', platform = ''): string => {
  const fallback = /Windows|Win32|Win64/i.test(platform) ? 'PealayerInter, Inter, system-ui, Vazirmatn, sans-serif'
    : /Mac|iPhone|iPad/i.test(platform) ? '-apple-system, BlinkMacSystemFont, PealayerInter, Vazirmatn, sans-serif'
    : 'Roboto, system-ui, PealayerInter, Vazirmatn, sans-serif';
  return choice === 'custom' && name.trim() ? `${customFamily(name)}, ${fallback}` : fallback;
};

/** Only preference fields enter config patches; host resolution is live state. */
export function mergeAppearance<T extends Record<string, any>>(
  values: T,
  appearance: AppearanceState,
): T {
  return {
    ...values,
    ui_animation_mode: appearance.ui_animation_mode,
    ui_control_density: appearance.ui_control_density,
    ui_font_family: appearance.ui_font_family,
    monospace_font_family: appearance.monospace_font_family,
    ui_font_name: appearance.ui_font_name,
    monospace_font_name: appearance.monospace_font_name,
    workspace_ui: appearance.workspace_ui,
    preferences_grouped_navigation: appearance.preferences_grouped_navigation,
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

export function paletteName(config: Record<string, unknown> | null): ColorPalette {
  const palette = config?.color_palette;
  return palette === 'studio' || palette === 'pastel' ? palette : 'native';
}
