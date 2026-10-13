export type PwmOnBehavior = 'full' | 'restore';
/** Configure only policy: omitted curve/step/icon fields must remain untouched. */
export function pwmPolicyPayload(channel: number, on_behavior: PwmOnBehavior) {
  return { channel, on_behavior };
}
