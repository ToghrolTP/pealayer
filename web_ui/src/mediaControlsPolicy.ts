/** OS/browser routing remains OS-owned. This gates only commands dispatched
 * to Pealayer, not unrelated players or system volume keys. */
export function mediaControlsAllowed(enabled: boolean | undefined, background: boolean | undefined, foreground: boolean): boolean {
  return enabled === true && (background === true || foreground);
}
