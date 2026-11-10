export type ChannelFolder = { kind: string; name: string; icon: string };
export type FolderControl = { key: string; kind: string; group: string; control: string; channel?: number | null; hidden: boolean; order: number };

export const channelFolderKind = (kind: string) => ['seat', 'side', 'motion'].includes(kind) ? 'motion'
  : kind === 'relay' ? 'relay' : ['pwm', 'mosfet'].includes(kind) ? 'pwm' : 'board';

// Wiring comes from the controller, never from a name or relay number alone.
export const isRawSeatRelay = (control: FolderControl) => control.kind === 'relay'
  && control.control === 'seat-internal' && control.channel != null && control.channel >= 1 && control.channel <= 4;

export function channelFolderGroups<T extends FolderControl>(controls: T[], folders: ChannelFolder[], section: string) {
  const groups = folders.filter((folder) => folder.kind === section)
    .map((folder) => ({ ...folder, raw: false, controls: [] as T[] }));
  for (const control of [...controls].sort((a, b) => a.order - b.order || a.key.localeCompare(b.key))) {
    if (control.hidden) continue;
    const raw = isRawSeatRelay(control);
    const name = raw ? 'Raw relays' : control.group.trim();
    let group = groups.find((folder) => folder.raw === raw && folder.name.toLowerCase() === name.toLowerCase());
    if (!group) { group = { kind: section, name, icon: '', raw, controls: [] }; groups.push(group); }
    group.controls.push(control);
  }
  if (groups.some((folder) => !folder.raw && folder.name) && !groups.some((folder) => !folder.name)) {
    groups.push({ kind: section, name: '', icon: '', raw: false, controls: [] });
  }
  return groups.sort((a, b) => Number(!a.name) - Number(!b.name) || Number(!a.raw) - Number(!b.raw) || a.name.localeCompare(b.name));
}
