/** UI numbering only; controller channel/action IDs remain unchanged. */
export function hardwareControlIdentity(control: { key: string; kind: string; channel?: number | null }): string {
  if (/relay/i.test(control.kind) && control.channel != null) return `R${control.channel}`;
  if (/pwm|mosfet/i.test(control.kind) && control.channel != null) return `CH${control.channel + 1}`;
  if (control.key === 'seat.a') return 'A';
  if (control.key === 'seat.b') return 'B';
  const parts = control.key.split('.');
  return parts[parts.length - 1]?.toUpperCase() || control.key;
}
