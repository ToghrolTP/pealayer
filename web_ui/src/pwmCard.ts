export type PwmCardPreferences = { show_on: boolean; show_off: boolean; show_decrease: boolean;
  show_increase: boolean; show_other: boolean; indicator_action: 'toggle' | 'on' | 'off' };
type Action = { id: string; verb: string };
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
