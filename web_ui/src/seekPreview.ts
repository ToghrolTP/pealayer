/** Slider rail coordinates, not a chapter boundary or the surrounding toolbar. */
export function hoveredSeekTime(left: number, width: number, x: number, duration: number, rtl = false) {
  if (![left, width, x, duration].every(Number.isFinite) || width <= 0 || duration <= 0) return null;
  const fraction = Math.min(1, Math.max(0, (x - left) / width));
  return Math.min(duration * (rtl ? 1 - fraction : fraction), Math.max(0, duration - .001));
}

export function previewPosition(x: number, barTop: number, barBottom: number, width: number, height: number, viewportWidth: number, viewportHeight: number) {
  const gap = 8;
  const above = barTop - height - gap;
  return {
    left: Math.min(Math.max(x - width / 2, gap), Math.max(gap, viewportWidth - width - gap)),
    top: Math.min(Math.max(above >= gap ? above : barBottom + gap, gap), Math.max(gap, viewportHeight - height - gap)),
  };
}

export function formatPreviewTime(seconds: number, subseconds: boolean, includeHours: boolean) {
  const ms = Math.max(0, Math.round(seconds * 1000));
  const whole = Math.floor(ms / 1000);
  return `${includeHours ? `${String(Math.floor(whole / 3600)).padStart(2, '0')}:` : ''}${String(Math.floor(whole / 60) % 60).padStart(2, '0')}:${String(whole % 60).padStart(2, '0')}${subseconds ? `.${String(ms % 1000).padStart(3, '0')}` : ''}`;
}
