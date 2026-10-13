export type Signal = { code: number; bits: number; protocol: number };
export type RfControl = { key: string; name: string; step_percent?: number; actions?: { id: string; name?: string; label?: string; verb?: string }[] };
export type RfAction = { type: string; action_id?: string; app_target?: string; app_kind?: string; app_value?: string; macro?: string; command?: string; virtual_key?: string; executable?: string; script?: string; event?: string; rf?: { code: number } };
export type BoardAssignment = { action: string; target?: string; behavior?: string; label: string };
export type LearnedSignal = Signal & { id: number; code_display: string; last_seen?: string | null; board_assignment?: BoardAssignment };
export const human = (key: string) => key.replace(/[._-]/g, ' ').replace(/^pealayer /, '').replace(/^./, s => s.toUpperCase());
export const sameSignal = (match: { rf_code: number; rf_bits: number; rf_protocol: number }, entry: Signal) => match.rf_code === entry.code && match.rf_bits === entry.bits && match.rf_protocol === entry.protocol;
export const controlOptions = (controls: RfControl[]) => controls.flatMap(control => (control.actions ?? []).map(action => ({ value: action.id, label: `${control.name} · ${action.label || action.name || human(action.verb || action.id)}${['increase', 'decrease'].includes(action.verb ?? '') && control.step_percent !== undefined ? ` (${control.step_percent}%)` : ''}` })));
export function actionSummary(action: RfAction, controls: RfControl[], effects: { reference: string; name: string }[]) {
  switch (action.type) {
    case 'control': return controlOptions(controls).find(option => option.value === action.action_id)?.label ?? `Unavailable control · ${action.action_id ?? ''}`;
    case 'app': return `${action.app_target ?? ''} · ${human(action.app_kind ?? '')} ${action.app_value ?? ''}`.trim();
    case 'effect': case 'macro': return effects.find(effect => effect.reference === action.macro)?.name ?? action.macro ?? 'Choose an effect';
    case 'board': return `Controller · ${action.command ?? ''}`;
    case 'virtual-key': return `Keyboard · ${action.virtual_key ?? ''}`;
    case 'host': return `Program · ${action.executable ?? ''}`;
    case 'script': return `Script · ${action.script ?? ''}`;
    case 'rf': return `Transmit · ${action.rf?.code ?? ''}`;
    case 'emit': return `Event · ${action.event ?? ''}`;
    default: return 'Choose an action';
  }
}
export const executionLabel = (actions: RfAction[]) => actions.some(action => action.type === 'app') ? 'PCController → application' : 'PCController · Pealayer is only the editor';
export function actionReady(action: RfAction, controls: RfControl[], effects: { reference: string; name: string }[], applications: {id:string;surface:string;values?:{app_actions?:string}}[], allowedKeys: string[]) {
  switch (action.type) {
    case 'control': return controlOptions(controls).some(option=>option.value===action.action_id);
    case 'app': return applications.some(app=>(app.id===action.app_target||app.surface===action.app_target)&&(app.values?.app_actions ?? '').split(',').includes(action.app_kind ?? ''));
    case 'effect': case 'macro': return effects.some(effect=>effect.reference===action.macro);
    case 'virtual-key': return allowedKeys.includes(action.virtual_key ?? '');
    case 'board': return Boolean(action.command?.trim());
    case 'host': return Boolean(action.executable?.trim());
    case 'script': return Boolean(action.script?.trim());
    case 'emit': return Boolean(action.event?.trim());
    case 'rf': return Boolean(action.rf && action.rf.code>0);
    default: return false;
  }
}
export const seenTime = (entry: LearnedSignal) => entry.last_seen && Number.isFinite(Date.parse(entry.last_seen)) ? Date.parse(entry.last_seen) : 0;
export const lastSeen = (entry: LearnedSignal) => seenTime(entry) ? new Date(entry.last_seen!).toLocaleString() : 'Not seen in this controller session';
export type MappingOption = { action: string; targets: string[]; behaviors: string[] };
export type BoardMap = { id: number; action: string; target?: string; behavior?: string };
export const editBoardMap = (entry: LearnedSignal): BoardMap => ({ id: entry.id, action: entry.board_assignment?.action ?? 'none', target: entry.board_assignment?.target, behavior: entry.board_assignment?.behavior });
export const boardMapValid = (map: BoardMap, entries: LearnedSignal[], options: MappingOption[]) => {
  const option = options.find(option => option.action === map.action);
  return Boolean(entries.some(entry => entry.id === map.id) && option && (!option.targets.length || option.targets.includes(map.target ?? '')) && (!option.behaviors.length || option.behaviors.includes(map.behavior ?? '')));
};
export const targetOptions = (option: MappingOption | undefined, controls: RfControl[]) => (option?.targets ?? []).map(value => {
  const kind = option?.action === 'side' ? 'seat' : option?.action;
  return { value, label: controls.find(control => control.key === `${kind}.${value}`)?.name ?? human(value) };
});
