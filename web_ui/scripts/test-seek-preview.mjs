import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { hoveredSeekTime, previewPosition, formatPreviewTime } from '../src/seekPreview.ts';
const read = (path) => readFileSync(new URL(path, import.meta.url), 'utf8');
assert.equal(hoveredSeekTime(108, 384, 300, 120), 60);
assert.equal(hoveredSeekTime(108, 384, 100, 120), 0);
assert.equal(hoveredSeekTime(108, 384, 600, 120), 119.999);
assert.equal(hoveredSeekTime(108, 384, 204, 120, true), 90);
for (const duration of [0, -1, NaN, Infinity]) assert.equal(hoveredSeekTime(0, 100, 50, duration), null);
assert.equal(hoveredSeekTime(0, 0, 50, 120), null);
assert.equal(formatPreviewTime(1845.1234, true, false), '30:45.123');
assert.equal(formatPreviewTime(1845.1234, false, false), '30:45');
assert.equal(formatPreviewTime(1.9996, true, true), '00:00:02.000');
assert.deepEqual(previewPosition(400, 220, 240, 254, 144, 800, 600), { left: 273, top: 68 });
for (const x of [0, 200, 400]) {
  const pos = previewPosition(x, 0, 20, 254, 144, 400, 300);
  assert(pos.left >= 8 && pos.left + 254 <= 392 && pos.top >= 28 && pos.top + 144 <= 292);
}
const preview = read('../src/components/SeekThumbnailPreview.tsx');
assert.equal((preview.match(/createPortal\(/g) ?? []).length, 1);
assert(!preview.includes('title=') && !preview.includes('hoverLabel'), 'no independent browser hover tooltips');
assert(preview.includes('const preview = !seeking && hover ?') && preview.includes('{showImage &&'), 'image preference does not disable cursor/chapter card');
assert(preview.includes("querySelector('.ant-slider-rail')") && preview.includes('chapterAt(chapters, seconds)'), 'card uses actual rail and cursor chapter, not marker start');
assert(preview.includes('formatPreviewTime(hover.seconds, showSubseconds') && preview.includes('Math.floor(hover.seconds)'), 'exact display time is separate from the decode bucket');
assert(preview.includes("addEventListener('scroll', clear, true)") && preview.includes('useLayoutEffect'), 'measured card fits the viewport and stale geometry is cleared');
for (const path of ['RemoteControlTab', 'StudioTab']) {
  const surface = read(`../src/components/${path}.tsx`);
  assert(surface.includes('<SeekThumbnailPreview') && surface.includes('tooltip={{ open: false }}'), `${path} has one shared hover owner`);
}
const markers = read('../src/components/seekbar.tsx').split('export const SeekbarMarkers')[1];
assert(!markers.includes('title=') && markers.includes('aria-label=') && markers.includes('onChapter(chapter.time_seconds)'), 'accessible chapter clicks remain without duplicate titles');
console.log('Seek preview: exact cursor time, one card, optional image, rail/RTL mapping and viewport fitting passed.');
