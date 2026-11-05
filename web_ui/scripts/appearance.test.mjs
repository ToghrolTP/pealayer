import assert from 'node:assert/strict';
import { test } from 'node:test';
import { accentForeground, controlDensityHeight, controlDensityText, mergeAppearance, paletteName, resolvedAccent, resolvedAppearanceTheme, uiFontCss, monoFontCss } from '../src/appearance.ts';

test('the original 18-point density remains available without reinterpreting saved choices', () => {
  for (const [choice, height] of Object.entries({ extra_compact: 18, compact: 24, regular: 28, comfortable: 32, spacious: 36 })) {
    assert.equal(controlDensityHeight(choice), height);
    assert.equal(mergeAppearance({}, { ui_control_density: choice }).ui_control_density, choice);
  }
  const text = controlDensityText('extra_compact');
  assert.equal(text.fontSize * text.lineHeight + 2, 18);
  assert.deepEqual(controlDensityText('compact'), {});
});

test('native workspace and font previews share one appearance contract', () => {
  const preview = { ui_font_family: 'custom', ui_font_name: 'Segoe UI',
    monospace_font_family: 'custom', monospace_font_name: 'Cascadia Mono',
    workspace_ui: 'egui_dock', preferences_grouped_navigation: false };
  const next = mergeAppearance({ volume: 42 }, preview);
  for (const [key, value] of Object.entries(preview)) assert.equal(next[key], value);
  assert.equal(next.volume, 42);
  assert.match(uiFontCss('custom', 'Segoe UI'), /^"Segoe UI", system-ui/);
  assert.match(monoFontCss('custom', 'Cascadia Mono'), /^"Cascadia Mono", ui-monospace/);
  assert.equal(monoFontCss('custom', ''), monoFontCss('system'));
  assert.match(uiFontCss('custom', 'A"B'), /^"A\\"B",/);
});

test('Neutral is the default; Studio remains an explicit palette', () => {
  assert.equal(paletteName(null), 'native');
  assert.equal(paletteName({}), 'native');
  assert.equal(paletteName({ color_palette: 'studio' }), 'studio');
});

test('native preview updates shared config without losing unrelated settings or leaking resolution fields', () => {
  const original = { volume: 37, color_palette: 'native' };
  const appearance = { theme: 'dark', color_palette: 'studio', accent_color: 'pealayer_green',
    custom_accent_color: null, resolved_theme: 'dark', resolved_accent: '#38d27a' };
  const next = mergeAppearance(original, appearance);
  assert.equal(next.volume, 37);
  assert.equal(next.color_palette, 'studio');
  assert.equal(next.accent_color, 'pealayer_green');
  assert.equal(next.resolved_accent, undefined);
  assert.equal(original.color_palette, 'native');
  assert.equal(resolvedAccent(null, next, appearance), '#38d27a');
  assert.equal(accentForeground('#38d27a'), '#141414');
});

test('System theme and accent follow the host, with offline fallbacks', () => {
  const appearance = { resolved_theme: 'dark', resolved_accent: '#9e5cf4' };
  assert.equal(resolvedAppearanceTheme('system', true, appearance), 'dark');
  assert.equal(resolvedAppearanceTheme('light', false, appearance), 'light');
  assert.equal(resolvedAppearanceTheme('system', true), 'light');
  assert.equal(resolvedAccent({ accentColor: '#0078d4' }, { accent_color: 'system' }, appearance), '#9e5cf4');
  assert.equal(resolvedAccent(null, { accent_color: 'custom', custom_accent_color: '#123456' }, appearance), '#123456');
});

test('live native theme events switch connected clients without changing the System preference', () => {
  const config = { theme: 'system', color_palette: 'native', accent_color: 'system' };
  for (const scheme of ['light', 'dark', 'light']) {
    const event = { ...config, resolved_theme: scheme, resolved_accent: '#6362c7' };
    const synced = mergeAppearance(config, event);
    assert.equal(synced.theme, 'system');
    assert.equal(resolvedAppearanceTheme(synced.theme, scheme !== 'light', event), scheme);
    assert.equal(resolvedAppearanceTheme('light', false, event), 'light');
    assert.equal(resolvedAppearanceTheme('dark', true, event), 'dark');
  }
});
