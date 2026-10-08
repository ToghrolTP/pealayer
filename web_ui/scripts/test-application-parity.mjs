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
const rfManager = read('src/components/RfManager.tsx');
const effects = read('src/components/EffectsTab.tsx');
const effectRecorder = read('src/components/EffectRecorder.tsx');
const effectGroupDialog = read('src/components/EffectGroupDialog.tsx');
const css = read('src/styles.css');
const preferences = read('src/components/PreferencesTab.tsx');
const filePicker = read('src/components/ServerFilePicker.tsx');
const authority = read('src/components/PublishingAuthority.tsx');

const contracts = [
  [authority.includes('Connect to authority') && authority.includes('authority.owner_endpoint') && authority.includes('disabled={!authority.owner_endpoint}'), 'conflicts offer the validated remote authority without inventing an endpoint'],
  [app.includes('<PublishingAuthority') && hardware.includes('<PublishingAuthority') && authority.includes("sendCmd('pealayer.hardware.authority'") && authority.includes('authority.owner_id === actor'), 'publishing authority uses the same server reservation in controls and the global conflict dialog'],
  [authority.includes('disabled={!paused || authority.exclusive}') && authority.includes("change('accept', request.client_id)") && authority.includes("change('unlock'") === false && authority.includes("'unlock' : 'lock'"), 'owner-consented handoff requires paused playback and production can be explicitly unlocked'],
  [preferences.includes('icon: string') && preferences.includes('controlIcons[control.icon]') && !preferences.includes('controlIcons[control.kind]'), 'preference row icons use semantic Rust metadata instead of input type'],
  [preferences.includes("control.kind === 'file'") && preferences.includes('<ServerFilePicker') && preferences.includes('FolderOpenOutlined'), 'file preferences expose a real Browse action'],
  [preferences.includes('contract?.groups.filter') && preferences.includes('group.default_open') && preferences.includes('<Collapse'), 'preference cards use shared ordering and optional disclosure'],
  [filePicker.includes('/api/fs/browse') && filePicker.includes('AbortController') && filePicker.includes('setDirectory(null)'), 'preference file picker browses authoritative server files without stale failed listings'],
  [hardware.includes("trigger={['contextMenu']}"), 'hardware cards expose a context menu'],
  [hardware.includes('onPointerDown') && hardware.includes("hardware.action.invoke"), 'hardware actions dispatch on pointer-down'],
  [hardware.includes('hardware-control__indicator') && hardware.includes('immediateToggle'), 'hardware indicators are actionable'],
  [hardware.includes('optimisticActive') && hardware.includes('invokeAction'), 'hardware actions acknowledge pointer-down immediately while awaiting board state'],
  [hardware.includes('controlIcon(control.kind, control.icon)'), 'hardware controls honor the shared custom icon contract'],
  [hardware.includes('Buzzer & melodies') && hardware.includes("hardware.buzzer.melody") && hardware.includes("hardware.buzzer.tone") && hardware.includes("hardware.buzzer.stop"), 'hardware monitor exposes live melody, tone, and stop controls'],
  [hardware.includes("onOpenChange={(open) => { if (open) void sendCmd('hardware.catalog.refresh'); }}"), 'opening a melody picker refreshes the controller catalog'],
  [css.includes('.hardware-estop.ant-btn-primary'), 'E-STOP owns its filled danger styling'],
  [sevenSegment.includes("mask & (1 << bit)") && hardware.includes('<SevenSegmentDisplay'), 'front-panel masks render as live seven-segment glyphs'],
  [remote.includes('<MediaSurface') && studio.includes('<MediaSurface'), 'both player surfaces share the native media element'],
  [media.includes('/api/fs/file?path=') && media.includes('<video'), 'browser playback consumes the range-serving endpoint'],
  [media.includes('onPointerDown') && media.includes('onPointerMove') && media.includes("trigger={['contextMenu']}"), 'video surfaces share configured mouse, pen, touch, and context-menu gestures'],
  [studio.includes('state.timeline_tracks') && studio.includes('timelineRows.map'), 'timeline renders the shared native track inventory'],
  [studio.includes("kind: 'pinch'") && studio.includes("event.button !== 1") && css.includes('.timeline-grid.is-panning'), 'timeline supports two-finger pinch/pan and middle-button panning'],
  [studio.includes("trigger={['contextMenu']}") && studio.includes("sendCmd('timeline.track.update'") && studio.includes("sendCmd('timeline.track.manage'"), 'timeline track menus use shared selection, routing and management commands'],
  [studio.includes('anyTimelineTrackSoloed') && studio.includes('is-solo-filtered') && css.includes('.timeline-row.is-muted') && css.includes('.timeline-row.is-soloed') && css.includes('.timeline-row.is-locked'), 'timeline tracks render semantic mute, solo-isolation, and lock states'],
  [library.includes("replace(/^\\\\\\\\\\?\\\\/"), 'Windows namespace prefixes are removed from breadcrumbs'],
  [library.includes("trigger={['contextMenu']}") && library.includes('Copy full path'), 'media entries expose application-style context actions'],
  [media.includes('state.osd') && css.includes('.media-osd--custom'), 'Web video surfaces render the native configurable OSD contract'],
  [app.includes('<ApplicationStatusBar') && statusBar.includes("trigger={['contextMenu']}") && statusBar.includes('status_rgb'), 'application footer renders live shared state and persisted context-menu visibility'],
  [css.includes('.studio-transport__play.ant-btn {') && css.includes('border-radius: 50%'), 'timeline play control keeps a circular hit target'],
  [css.includes('color-mix(in srgb, var(--accent) 74%') && !css.includes('.effect-profile__icon { width: 36px; height: 36px; display: grid; place-items: center; border-radius: 8px; color: var(--green)'), 'decorative Web UI surfaces use the shared accent rather than Pealayer green'],
  [!rfManager.includes('setInterval'), 'RF manager uses push events and explicit refresh instead of polling'],
  [rfManager.includes("command('catalog', { read_board: true })") && rfManager.includes('dataSource={catalog.entries ?? []}'), 'RF manager loads and renders authoritative learned board codes'],
  [effectRecorder.includes('<Button type="primary" danger') && !effectRecorder.includes('recordingColors'), 'effect recording is always a solid red live-state action'],
  [effects.includes("controller_effect.group.save") && effects.includes('original_name: groupDraft.original_name') && effectGroupDialog.includes('<EffectIconPicker'), 'effect parent groups expose editable icons through the current save contract'],
  [effects.includes("setData('application/x-pealayer-effect'") && effects.includes('void moveEffectToGroup(effect, group.name)') && effects.includes('category,') && css.includes('.effect-group-title.is-drop-target'), 'effect cards can move between authoritative parent groups by drag and drop'],
  [effects.includes("showArrow: group.items.length > 0") && effects.includes("collapsible: group.items.length === 0 ? 'icon'") && css.includes('.effect-group.is-empty'), 'empty effect groups are dimmed and do not expose a meaningless chevron'],
];

const failed = contracts.filter(([ok]) => !ok);
if (failed.length) {
  for (const [, message] of failed) console.error(`Missing parity contract: ${message}`);
  process.exit(1);
}
console.log(`Web application parity guardrails passed (${contracts.length} contracts).`);
