// Real Effects Designer with an isolated, simulated controller snapshot.
import { createRoot } from 'react-dom/client';
import { ConfigProvider, theme } from 'antd';
import type { ImportedEffect } from '../src/assetImports';
import { EffectsTab } from '../src/components/EffectsTab';
import type { PlayerState } from '../src/components/RemoteControlTab';
import palettes from '../../assets/themes/palettes.json';
import '../src/styles.css';

const params = new URLSearchParams(location.search);
const rtl = params.has('fa');
const active = !params.has('inactive');
const deviceRetained = !params.has('host');
const overwritten = params.has('within-capacity') ? 0 : 5;
const palette = palettes.studio.dark;
document.documentElement.dataset.theme = 'dark';
document.documentElement.lang = rtl ? 'fa' : 'en';
document.documentElement.dir = rtl ? 'rtl' : 'ltr';
for (const [role, value] of Object.entries(palette)) {
  document.documentElement.style.setProperty(`--${role}`, value);
}
document.documentElement.style.setProperty('--accent', palette.blue);
window.fetch = async input => {
  if (input === '/api/icons/catalog') return Response.json([]);
  throw Error('Unexpected fixture network request');
};

const effect: ImportedEffect = {
  id: '12', reference: '', name: rtl ? 'آزمون نورپردازی' : 'Lighting rehearsal',
  icon: 'lamp', category: '', description: '', kind: 'sequence', color: '',
  duration_ms: 1600, default_fps: 0, default_pixels: 0,
  program: { steps: [
    { kind: 'relay', target: 5, value: 1, at_us: 0 },
    { kind: 'relay', target: 5, value: 0, at_us: 1_600_000 },
  ] },
};
const state = {
  controller_connected: true,
  hardware_connected: true,
  effect_recording: {
    active, id: 12, name: effect.name, mode: deviceRetained ? 'board-retained' : 'automatic',
    category: '', color: '', steps: 25, preview: [], device_retained: deviceRetained, overwritten,
    started_at: '', last_error: '', pending: false,
  },
} as PlayerState;

createRoot(document.getElementById('root')!).render(
  <ConfigProvider direction={rtl ? 'rtl' : 'ltr'} theme={{ algorithm: theme.darkAlgorithm, token: { colorPrimary: palette.blue } }}>
    <EffectsTab state={state} sendCmd={async () => { throw Error('Unexpected controller command'); }}
      locale={rtl ? 'fa' : 'en'} apiBaseUrl="" importedEffect={effect} />
  </ConfigProvider>,
);
