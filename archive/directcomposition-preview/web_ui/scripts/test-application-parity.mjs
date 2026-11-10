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
const statusBar = read('src/components/ApplicationStatusBar.tsx');
const sevenSegment = read('src/components/SevenSegmentDisplay.tsx');
const css = read('src/styles.css');

const contracts = [
  [hardware.includes("trigger={['contextMenu']}"), 'hardware cards expose a context menu'],
  [hardware.includes('onPointerDown') && hardware.includes("hardware.action.invoke"), 'hardware actions dispatch on pointer-down'],
  [hardware.includes('hardware-control__indicator') && hardware.includes('immediateToggle'), 'hardware indicators are actionable'],
  [hardware.includes('optimisticActive') && hardware.includes('invokeAction'), 'hardware actions acknowledge pointer-down immediately while awaiting board state'],
  [hardware.includes('controlIcon(control.kind, control.icon)'), 'hardware controls honor the shared custom icon contract'],
  [css.includes('.hardware-estop.ant-btn-primary'), 'E-STOP owns its filled danger styling'],
  [sevenSegment.includes("mask & (1 << bit)") && hardware.includes('<SevenSegmentDisplay'), 'front-panel masks render as live seven-segment glyphs'],
  [remote.includes('<MediaSurface') && studio.includes('<MediaSurface'), 'both player surfaces share the native media element'],
  [media.includes('/api/fs/file?path=') && media.includes('<video'), 'browser playback consumes the range-serving endpoint'],
  [media.includes('onPointerDown') && media.includes('onPointerMove') && media.includes("trigger={['contextMenu']}"), 'video surfaces share configured mouse, pen, touch, and context-menu gestures'],
  [studio.includes('state.timeline_tracks') && studio.includes('timelineRows.map'), 'timeline renders the shared native track inventory'],
  [studio.includes("kind: 'pinch'") && studio.includes("event.button !== 1") && css.includes('.timeline-grid.is-panning'), 'timeline supports two-finger pinch/pan and middle-button panning'],
  [studio.includes("trigger={['contextMenu']}") && studio.includes("sendCmd('timeline.track.update'") && studio.includes("sendCmd('timeline.track.manage'"), 'timeline track menus use shared selection, routing and management commands'],
  [library.includes("replace(/^\\\\\\\\\\?\\\\/"), 'Windows namespace prefixes are removed from breadcrumbs'],
  [library.includes("trigger={['contextMenu']}") && library.includes('Copy full path'), 'media entries expose application-style context actions'],
  [media.includes('state.osd') && css.includes('.media-osd--custom'), 'Web video surfaces render the native configurable OSD contract'],
  [app.includes('<ApplicationStatusBar') && statusBar.includes("trigger={['contextMenu']}") && statusBar.includes('status_rgb'), 'application footer renders live shared state and persisted context-menu visibility'],
  [css.includes('.studio-transport__play.ant-btn {') && css.includes('border-radius: 50%'), 'timeline play control keeps a circular hit target'],
  [css.includes('color-mix(in srgb, var(--accent) 74%') && !css.includes('.effect-profile__icon { width: 36px; height: 36px; display: grid; place-items: center; border-radius: 8px; color: var(--green)'), 'decorative Web UI surfaces use the shared accent rather than Pealayer green'],
];

const failed = contracts.filter(([ok]) => !ok);
if (failed.length) {
  for (const [, message] of failed) console.error(`Missing parity contract: ${message}`);
  process.exit(1);
}
console.log(`Web application parity guardrails passed (${contracts.length} contracts).`);
