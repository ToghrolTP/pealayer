import { useEffect, useRef } from 'react';
import type { PointerEvent, MouseEvent } from 'react';

export type MouseAction = 'none' | 'inherit' | 'seek_backward' | 'seek_forward' | 'previous_chapter' | 'next_chapter' |
  'previous_file' | 'next_file' | 'frame_backward' | 'frame_forward' | 'play_pause' | 'toggle_mute' | 'toggle_fullscreen' |
  'boss_key' | 'pan_left' | 'pan_right' | 'zoom_out' | 'zoom_in' | 'history_back' | 'history_forward' | 'parent_folder';
export interface MouseButtons {
  back: MouseAction; forward: MouseAction; ctrl_back: MouseAction; ctrl_forward: MouseAction;
  alt_back: MouseAction; alt_forward: MouseAction; shift_back: MouseAction; shift_forward: MouseAction;
}
export interface MouseBindings {
  video: MouseButtons; timeline: MouseButtons; effect_designer: MouseButtons; remote_browser: MouseButtons;
  ctrl_left_drag: 'none' | 'move_window' | 'seek' | 'temporary_fast_forward';
  alt_middle_drag: 'none' | 'move_window' | 'seek' | 'temporary_fast_forward';
  shift_right_context_menu: boolean;
}
export interface Modifiers { ctrlKey: boolean; metaKey: boolean; altKey: boolean; shiftKey: boolean }
export function extraMouseAction(buttons: MouseButtons | undefined, button: number, modifiers: Modifiers): MouseAction {
  if (!buttons || (button !== 3 && button !== 4)) return 'none';
  const side = button === 4 ? 'forward' : 'back';
  const modifier = modifiers.ctrlKey || modifiers.metaKey ? 'ctrl' : modifiers.altKey ? 'alt' : modifiers.shiftKey ? 'shift' : null;
  const action = modifier ? buttons[`${modifier}_${side}`] : buttons[side];
  return action === 'inherit' ? buttons[side] : action;
}
export function videoMousePolicy(button: number, modifiers: Modifiers, bindings: MouseBindings | undefined,
  click: string, drag: string) {
  if (button === 2 && modifiers.shiftKey && bindings?.shift_right_context_menu) return { click: 'context_menu', drag: 'none', forceMenu: true };
  if (button === 0 && (modifiers.ctrlKey || modifiers.metaKey)) return { click: 'none', drag: bindings?.ctrl_left_drag ?? 'move_window', forceMenu: false };
  if (button === 1 && modifiers.altKey) return { click: 'none', drag: bindings?.alt_middle_drag ?? 'seek', forceMenu: false };
  return { click, drag, forceMenu: false };
}
export function videoReleaseIntent(gesture: {forceMenu:boolean;dragging:boolean;moved:boolean;clickAction:string},
  cancelled:boolean,inside:boolean,sameSource:boolean): 'menu'|'click'|'none' {
  if(cancelled || !inside || !sameSource) return 'none';
  if(gesture.forceMenu) return 'menu';
  if(gesture.dragging || gesture.moved) return 'none';
  return gesture.clickAction==='context_menu'?'menu':'click';
}
/** Each visible surface owns only a press that starts there; no global pointer capture. */
export function useExtraMouse(buttons: MouseButtons | undefined, invoke: (action: MouseAction) => void) {
  const press = useRef<{ pointer: number; button: number; x: number; y: number; moved: boolean; action: MouseAction } | null>(null);
  useEffect(() => {
    const cancel = () => { press.current = null; };
    const escape = (event: KeyboardEvent) => { if (event.key === 'Escape') cancel(); };
    window.addEventListener('blur', cancel);
    window.addEventListener('keydown', escape);
    return () => { window.removeEventListener('blur', cancel); window.removeEventListener('keydown', escape); };
  }, []);
  const suppressBrowser = (event: MouseEvent<HTMLElement>) => { if (event.button === 3 || event.button === 4) event.preventDefault(); };
  return {
    onPointerDown: (event: PointerEvent<HTMLElement>) => {
      if (event.button !== 3 && event.button !== 4) return;
      event.preventDefault(); event.stopPropagation();
      if (press.current) return;
      press.current = { pointer: event.pointerId, button: event.button, x: event.clientX, y: event.clientY, moved: false, action: extraMouseAction(buttons, event.button, event) };
      event.currentTarget.setPointerCapture(event.pointerId);
    },
    onPointerMove: (event: PointerEvent<HTMLElement>) => { const p = press.current; if (p) p.moved ||= Math.hypot(event.clientX - p.x, event.clientY - p.y) >= 6; },
    onPointerUp: (event: PointerEvent<HTMLElement>) => {
      const p = press.current; if (!p || p.pointer !== event.pointerId || p.button !== event.button) return;
      event.preventDefault(); event.stopPropagation(); press.current = null;
      if (event.currentTarget.hasPointerCapture(event.pointerId)) event.currentTarget.releasePointerCapture(event.pointerId);
      const box = event.currentTarget.getBoundingClientRect();
      const hit=document.elementFromPoint(event.clientX,event.clientY);
      if ((!hit || event.currentTarget.contains(hit)) && !p.moved && Math.hypot(event.clientX - p.x, event.clientY - p.y) < 6 && event.clientX >= box.left && event.clientX <= box.right && event.clientY >= box.top && event.clientY <= box.bottom) invoke(p.action);
    },
    onPointerCancel: () => { press.current = null; },
    onLostPointerCapture: () => { press.current = null; },
    onAuxClick: suppressBrowser,
  };
}
