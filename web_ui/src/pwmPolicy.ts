export type PwmOnBehavior = 'full' | 'restore';
export function pwmMemoryLabel(persisted: unknown) {
  return persisted === true ? 'Confirmed brightness is saved in the selected board profile'
    : 'PCController manages confirmed brightness memory';
}
/** Configure only policy: omitted curve/step/icon fields must remain untouched. */
export function pwmPolicyPayload(channel: number, on_behavior: PwmOnBehavior) {
  return { channel, on_behavior };
}
