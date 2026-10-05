import assert from 'node:assert/strict';
import { test } from 'node:test';
import { accentForeground, mergeAppearance, paletteName, resolvedAccent, resolvedAppearanceTheme } from '../src/appearance.ts';

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
