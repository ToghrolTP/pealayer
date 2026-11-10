export interface ToastSnapshot {
  instance_id: string;
  revision: number;
  toasts: Array<{ id: string; title: string; message: string; severity: 'info' | 'success' | 'warning' | 'error'; source: string; created_at_ms: number; expires_at_ms: number | null }>;
}
// A snapshot replaces the active set, never appends/replays an event history.
export function visibleToasts(snapshot: ToastSnapshot | undefined, now: number, connected: boolean) {
  if (!connected) return [];
  return (snapshot?.toasts || []).filter(t => t.expires_at_ms === null || t.expires_at_ms > now).slice(-4).reverse();
}
