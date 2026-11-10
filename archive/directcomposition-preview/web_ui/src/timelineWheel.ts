export type TimelineWheelBehavior = 'zoom' | 'horizontal_scroll' | 'vertical_scroll' | 'none';
export interface TimelineWheelPreferences {
  plain: TimelineWheelBehavior;
  ctrl: TimelineWheelBehavior;
  shift: TimelineWheelBehavior;
  alt: TimelineWheelBehavior;
}
export const defaultTimelineWheelPreferences: TimelineWheelPreferences = {
  plain: 'zoom', ctrl: 'vertical_scroll', shift: 'horizontal_scroll', alt: 'zoom',
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
