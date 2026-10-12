// The current Rust ControlBarAction declaration order is also the flag index.
export const ACTIONS = ['play_pause','stop','back','forward','previous_chapter','next_chapter',
  'video_track','audio_track','subtitle_track','fullscreen','pin','audio_settings','subtitle_settings','mute','preferences'] as const;
export type ControlAction = typeof ACTIONS[number];
export type ControlSurface = 'simple' | 'monitor';
export interface ControlLayout { order: ControlAction[]; hidden: boolean[]; end: boolean[] }
export interface ControlBarsConfig { simple: ControlLayout; monitor: ControlLayout }
export const actionIndex = (action: ControlAction) => ACTIONS.indexOf(action);
export function orderedActions(order: readonly ControlAction[]): ControlAction[] {
  return [...new Set([...order, ...ACTIONS])].filter((action) => ACTIONS.includes(action));
}
export function defaultLayout(surface: ControlSurface): ControlLayout {
  const visible: ControlAction[] = surface === 'simple'
    ? ['back','play_pause','forward','fullscreen','pin','audio_settings','subtitle_settings','mute']
    : ['play_pause','stop','back','forward','fullscreen','video_track','audio_track','subtitle_track','mute'];
  const end: ControlAction[] = ['fullscreen','pin','video_track','audio_track','subtitle_track','audio_settings','subtitle_settings','mute','preferences'];
  return { order: surface === 'simple' ? orderedActions(['back','play_pause','forward']) : [...ACTIONS],
    hidden: ACTIONS.map((a) => !visible.includes(a)), end: ACTIONS.map((a) => end.includes(a)) };
}
export function patchAction(layout: ControlLayout, action: ControlAction, field: 'hidden' | 'end', value: boolean): ControlLayout {
  const flags = [...layout[field]]; flags[actionIndex(action)] = value;
  return { ...layout, [field]: flags };
}
export function moveAction(layout: ControlLayout, action: ControlAction, target: ControlAction, after: boolean): ControlLayout {
  if (action === target) return layout;
  const order = orderedActions(layout.order).filter((a) => a !== action);
  order.splice(order.indexOf(target) + Number(after), 0, action);
  return { ...patchAction(layout, action, 'end', layout.end[actionIndex(target)]), order };
}
// Browser controls do not own a native floating window or native settings panes.
export const browserActions = ACTIONS.filter((a) => !['pin','audio_settings','subtitle_settings'].includes(a));
export const actionLabels: Record<ControlAction,string> = {
  play_pause:'Play / Pause',stop:'Stop',back:'Back',forward:'Forward',previous_chapter:'Previous chapter',
  next_chapter:'Next chapter',video_track:'Video track',audio_track:'Audio track',subtitle_track:'Subtitles track',
  fullscreen:'Fullscreen',pin:'Pin controls',audio_settings:'Audio Settings...',subtitle_settings:'Subtitle Settings...',
  mute:'Mute',preferences:'Preferences',
};
