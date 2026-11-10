/** Omit only leading zero clock units; never remove interior zero fields. */
export function formatTimelineTime(seconds = 0, showMilliseconds = false): string {
  if (!Number.isFinite(seconds) || seconds < 0) return '—';
  const totalMillis = Math.round(seconds * 1000);
  const whole = Math.floor(totalMillis / 1000);
  const hours = Math.floor(whole / 3600);
  const minutes = Math.floor(whole / 60) % 60;
  const remaining = whole % 60;
  const pad = (value: number) => String(value).padStart(2, '0');
  const clock = hours > 0 ? `${hours}:${pad(minutes)}:${pad(remaining)}`
    : minutes > 0 ? `${minutes}:${pad(remaining)}` : String(remaining);
  const millis = totalMillis % 1000;
  return showMilliseconds && millis > 0 ? `${clock}.${String(millis).padStart(3, '0')}` : clock;
}
