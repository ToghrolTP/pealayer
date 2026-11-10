const folded = (value: string) => value.trim().toLowerCase();
function utf8Prefix(value: string, maxBytes: number): string {
  const encoder = new TextEncoder(); let bytes = 0; let result = '';
  for (const character of value) {
    bytes += encoder.encode(character).length;
    if (bytes > maxBytes) break;
    result += character;
  }
  return result;
}
export function uniqueRfBindingName(preferred: string, existing: string[]): string {
  const base = preferred.trim() || 'RF assignment';
  const used = new Set(existing.map(folded));
  for (let index = 0; index <= used.size + 1; index++) {
    const suffix = index === 0 ? '' : ` ${index + 1}`;
    const candidate = utf8Prefix(base, 64 - suffix.length).trimEnd() + suffix;
    if (!used.has(folded(candidate))) return candidate;
  }
  throw new Error('Unable to create an RF assignment name');
}
export function rfBindingNameConflicts(name: string, previousName: string, existing: string[]): boolean {
  return existing.some(old => folded(old) === folded(name) && (!previousName || folded(old) !== folded(previousName)));
}
