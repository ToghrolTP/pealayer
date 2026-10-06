import fs from 'node:fs';
import path from 'node:path';

const root = path.resolve(import.meta.dirname, '..');
const read = (file) => fs.readFileSync(path.join(root, file), 'utf8');
const hardware = read('src/components/HardwareTab.tsx');
const studio = read('src/components/StudioTab.tsx');
const remote = read('src/components/RemoteControlTab.tsx');
const media = read('src/components/MediaSurface.tsx');
const library = read('src/components/MediaLibraryTab.tsx');
const app = read('src/App.tsx');
const css = read('src/styles.css');

const contracts = [
  [hardware.includes("trigger={['contextMenu']}"), 'hardware cards expose a context menu'],
  [hardware.includes('onPointerDown') && hardware.includes("hardware.action.invoke"), 'hardware actions dispatch on pointer-down'],
  [hardware.includes('hardware-control__indicator') && hardware.includes('immediateToggle'), 'hardware indicators are actionable'],
  [css.includes('.hardware-estop.ant-btn-primary'), 'E-STOP owns its filled danger styling'],
  [remote.includes('<MediaSurface') && studio.includes('<MediaSurface'), 'both player surfaces share the native media element'],
  [media.includes('/api/fs/file?path=') && media.includes('<video'), 'browser playback consumes the range-serving endpoint'],
  [studio.includes('state.timeline_tracks') && studio.includes('timelineRows.map'), 'timeline renders the shared native track inventory'],
  [library.includes("replace(/^\\\\\\\\\\?\\\\/"), 'Windows namespace prefixes are removed from breadcrumbs'],
  [media.includes('state.osd') && css.includes('.media-osd--custom'), 'Web video surfaces render the native configurable OSD contract'],
  [app.includes('className="app-statusbar"') && app.includes('state.hardware_connected'), 'application footer renders live shared state'],
];

const failed = contracts.filter(([ok]) => !ok);
if (failed.length) {
  for (const [, message] of failed) console.error(`Missing parity contract: ${message}`);
  process.exit(1);
}
console.log(`Web application parity guardrails passed (${contracts.length} contracts).`);
