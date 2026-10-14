export type PwmCardPreferences = { show_on: boolean; show_off: boolean; show_decrease: boolean;
  show_increase: boolean; show_other: boolean; indicator_action: 'toggle' | 'on' | 'off' };
type Action = { id: string; verb: string };
export function observedControlActive(control: {active?:boolean|null}): boolean | undefined {
  return typeof control.active === 'boolean' ? control.active : undefined;
}
export function powerActionSelected(verb: string, active: boolean | undefined) {
  return (verb.toLowerCase() === 'on' && active === true) || (verb.toLowerCase() === 'off' && active === false);
}
/** rc-slider starts changes on any mousedown. Stop non-primary starts in capture,
 * before its rail/handle handlers, without cancelling the separate contextmenu. */
export function guardHardwarePointerStart(event: {button:number;stopPropagation:()=>void}) {
  if (event.button !== 0) event.stopPropagation();
}
export const defaultPwmCard: PwmCardPreferences = { show_on: true, show_off: true,
  show_decrease: true, show_increase: true, show_other: true, indicator_action: 'toggle' };
export function pwmActionVisible(preferences: PwmCardPreferences, verb: string): boolean {
  switch (verb.toLowerCase()) {
    case 'on': return preferences.show_on;
    case 'off': return preferences.show_off;
    case 'increase': return preferences.show_increase;
    case 'decrease': return preferences.show_decrease;
    default: return preferences.show_other;
  }
}
export function pwmIndicatorAction<T extends Action>(preferences: PwmCardPreferences, actions: T[], active?: boolean): T | undefined {
  const find = (verb: string) => actions.find(action => action.verb.toLowerCase() === verb);
  if (preferences.indicator_action !== 'toggle') return find(preferences.indicator_action);
  return find('toggle') ?? (active === undefined ? undefined : find(active ? 'off' : 'on'));
}
export function pwmRowActions<T extends Action>(preferences: PwmCardPreferences, actions: T[]): T[] {
  return actions.filter(action => !['increase', 'decrease'].includes(action.verb.toLowerCase())
    && pwmActionVisible(preferences, action.verb));
}
