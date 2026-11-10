export interface FileLocation { name: string; path: string; kind: 'drive' | 'removable' | 'network' | 'folder' | 'shortcut' | 'recent' }

/** Paths belong to the selected server, never to the browser's OS. */
export function filesystemBreadcrumbs(value: string): { name: string; path: string }[] {
  const ordinary = value.replace(/^\\\\\?\\UNC\\/i, '\\\\').replace(/^\\\\\?\\/, '');
  if (/^(\\\\|\/\/)/.test(ordinary)) {
    const parts = ordinary.split(/[\\/]+/).filter(Boolean);
    if (parts.length < 2) return [{ name: ordinary, path: ordinary }];
    const root = `\\\\${parts[0]}\\${parts[1]}\\`;
    return [{ name: root.slice(0, -1), path: root }, ...parts.slice(2).map((name, index) => ({
      name, path: root + parts.slice(2, index + 3).join('\\'),
    }))];
  }
  const windows = /^[A-Za-z]:[\\/]/.test(ordinary);
  const root = windows ? `${ordinary.slice(0, 2)}\\` : '/';
  const parts = ordinary.slice(windows ? 2 : 0).split(windows ? /[\\/]+/ : /\/+/).filter(Boolean);
  return [{ name: windows ? ordinary.slice(0, 2) : 'Root', path: root }, ...parts.map((name, index) => ({
    name, path: root + parts.slice(0, index + 1).join(windows ? '\\' : '/'),
  }))];
}

export function parseFileLocations(value: unknown): FileLocation[] {
  const records = (value as { locations?: unknown })?.locations;
  if (!Array.isArray(records) || records.length > 1024) throw new Error('Invalid folder locations response');
  const kinds = new Set(['drive', 'removable', 'network', 'folder', 'shortcut', 'recent']);
  return records.map(record => {
    if (!record || typeof record.name !== 'string' || typeof record.path !== 'string'
      || !record.path || !kinds.has(record.kind)) throw new Error('Invalid folder location');
    return record as FileLocation;
  });
}
