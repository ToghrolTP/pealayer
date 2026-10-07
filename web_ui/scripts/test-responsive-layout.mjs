// Cheap build-time guardrails for the mobile regressions. These complement,
// not replace, browser geometry and screenshot checks documented in docs.
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

const css = readFileSync(new URL('../src/styles.css', import.meta.url), 'utf8')
  .replace(/\/\*[\s\S]*?\*\//g, '');
const rules = [...css.matchAll(/([^{}]+)\{([^{}]*)\}/g)];
function declaration(selector, property, value) {
  assert(rules.some(([, selectors, body]) => selectors.split(',').map(s => s.trim()).includes(selector)
    && body.split(';').some(d => d.trim() === `${property}: ${value}`)),
  `${selector} must retain ${property}: ${value}`);
}

// Four classes outrank Ant Design's three-class has-sider width: 0 rule.
declaration('.app-body.ant-layout-has-sider > .app-content.ant-layout-content', 'width', '100%');
declaration('.app-body', 'min-width', '0');
declaration('.player-panel-web', 'grid-template-columns', 'minmax(0, 1fr)');
declaration('.program-viewer', 'min-width', '0');
declaration('.remote-player', 'grid-template-columns', 'minmax(0, 1fr)');
declaration('.remote-player__preview', 'min-height', '0');
declaration('.timeline-grid', 'overflow', 'auto');
declaration('.timeline-content', 'min-width', 'max(100%, 680px)');
declaration('.studio-scrubber', 'grid-row', '2');
declaration('.studio-timecode--muted', 'grid-column', '5');
declaration('.hardware-control__pwm', 'grid-column', '1 / -1');
declaration('.preferences-rail', 'overflow-x', 'auto');
declaration('.media-library__breadcrumbs a', 'overflow-wrap', 'anywhere');
declaration('.info-surface .ant-descriptions-view table', 'table-layout', 'fixed');
declaration('.app-sider .app-menu.ant-menu-inline-collapsed > .ant-menu-item', 'display', 'flex');
declaration('.app-sider .app-menu.ant-menu-inline-collapsed > .ant-menu-item', 'align-items', 'center');
declaration('.app-sider .app-menu.ant-menu-inline-collapsed > .ant-menu-item', 'justify-content', 'center');
declaration('.app-sider .app-menu.ant-menu-inline-collapsed > .ant-menu-item .ant-menu-title-content', 'display', 'none !important');
declaration('.ant-modal-body', 'overflow', 'auto');
declaration('.ant-modal-footer', 'flex-wrap', 'wrap');
const library = readFileSync(new URL('../src/components/MediaLibraryTab.tsx', import.meta.url), 'utf8');
assert.match(library, /scroll=\{\{ x: 640 \}\}/, 'Wide library tables need their own horizontal scroller');
console.log('Responsive layout guardrails passed (21 contracts).');
