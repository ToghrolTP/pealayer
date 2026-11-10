export type TimelineWheelBehavior = 'zoom' | 'horizontal_scroll' | 'vertical_scroll' | 'none';
export interface TimelineWheelPreferences {
  plain: TimelineWheelBehavior;
  ctrl: TimelineWheelBehavior;
  shift: TimelineWheelBehavior;
  alt: TimelineWheelBehavior;
}
export const defaultTimelineWheelPreferences: TimelineWheelPreferences = {
  plain: 'vertical_scroll', ctrl: 'zoom', shift: 'horizontal_scroll', alt: 'zoom',
};

export function timelineWheelAction(
  event: Pick<WheelEvent, 'deltaX' | 'deltaY' | 'shiftKey' | 'ctrlKey' | 'metaKey' | 'altKey'>,
  preferences: TimelineWheelPreferences,
): { action: TimelineWheelBehavior; delta: number } {
  // Preserve physical horizontal input even when Ctrl/Shift/Alt is held.
  if (event.deltaX !== 0) return { action: 'horizontal_scroll', delta: event.deltaX };
  const action = event.shiftKey ? preferences.shift
    : event.ctrlKey || event.metaKey ? preferences.ctrl
      : event.altKey ? preferences.alt : preferences.plain;
  return { action, delta: event.deltaY };
}

export function timelineZoomAtPointer(zoom: number, delta: number, scrollLeft: number, pointerX: number) {
  const nextZoom = Math.max(1, Math.min(25, zoom * Math.exp(-delta * .002)));
  return { zoom: nextZoom, scrollLeft: Math.max(0, (scrollLeft + pointerX) * nextZoom / zoom - pointerX) };
}

export function sequenceWheelNavigation(scale: number, action: TimelineWheelBehavior, delta: number,
  left: number, top: number, pointerX: number, viewportWidth: number, viewportHeight: number,
  contentHeight: number, durationMs: number) {
  const nextScale = action === 'zoom' ? Math.max(.03, Math.min(.32, scale * Math.exp(-delta * .002))) : scale;
  const pointer = Math.max(0, Math.min(viewportWidth, pointerX));
  const timeX = Math.max(0, left + pointer - 140);
  const anchored = nextScale !== scale && timeX > 0 ? timeX * nextScale / scale + 140 - pointer : left;
  const horizontal = action === 'horizontal_scroll' || (action === 'vertical_scroll' && contentHeight <= viewportHeight);
  const width = Math.max(600, durationMs * nextScale + 160);
  return { scale: nextScale, width,
    left: Math.max(0, Math.min(width + 140 - viewportWidth, anchored + (horizontal ? delta : 0))),
    top: Math.max(0, Math.min(contentHeight - viewportHeight, top + (action === 'vertical_scroll' && !horizontal ? delta : 0))) };
}
