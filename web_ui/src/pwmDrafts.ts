export function reconcilePWMDrafts(
  drafts: Record<string, number>, editing: ReadonlySet<string>, sentAt: Record<string, number>,
  revision: number, observed: ReadonlyArray<{ key: string; percent?: number | null }>,
): Record<string, number> {
  const next = { ...drafts };
  for (const control of observed) {
    if (editing.has(control.key) || !(control.key in next) ||
        control.percent == null || !Number.isFinite(control.percent)) continue;
    if (revision > (sentAt[control.key] ?? revision) || Math.abs(control.percent - next[control.key]) < .11) delete next[control.key];
  }
  return next;
}
