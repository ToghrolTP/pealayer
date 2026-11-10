import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {extraMouseAction,videoMousePolicy,videoReleaseIntent} from '../src/mouseBindings.ts';
const no={ctrlKey:false,metaKey:false,altKey:false,shiftKey:false};
const buttons={back:'seek_backward',forward:'seek_forward',ctrl_back:'previous_chapter',ctrl_forward:'next_chapter',
  alt_back:'frame_backward',alt_forward:'frame_forward',shift_back:'previous_file',shift_forward:'next_file'};
const config={video:buttons,timeline:buttons,effect_designer:buttons,remote_browser:buttons,
  ctrl_left_drag:'move_window',alt_middle_drag:'seek',shift_right_context_menu:true};
assert.equal(extraMouseAction(buttons,3,no),'seek_backward');
assert.equal(extraMouseAction(buttons,4,no),'seek_forward');
assert.equal(extraMouseAction(buttons,4,{...no,ctrlKey:true,altKey:true,shiftKey:true}),'next_chapter');
assert.equal(extraMouseAction({...buttons,ctrl_back:'inherit'},3,{...no,metaKey:true}),'seek_backward');
assert.equal(extraMouseAction({...buttons,ctrl_back:'none'},3,{...no,ctrlKey:true}),'none');
assert.equal(extraMouseAction(buttons,1,no),'none');assert.equal(extraMouseAction(undefined,4,no),'none');
assert.deepEqual(videoMousePolicy(0,{...no,ctrlKey:true},config,'play_pause','seek'),{click:'none',drag:'move_window',forceMenu:false});
assert.deepEqual(videoMousePolicy(1,{...no,altKey:true},config,'toggle_fullscreen','none'),{click:'none',drag:'seek',forceMenu:false});
assert.deepEqual(videoMousePolicy(2,{...no,shiftKey:true,ctrlKey:true},config,'toggle_mute','temporary_fast_forward'),{click:'context_menu',drag:'none',forceMenu:true});
assert.deepEqual(videoMousePolicy(1,no,config,'toggle_fullscreen','none'),{click:'toggle_fullscreen',drag:'none',forceMenu:false});
for(const forceMenu of [false,true])for(const dragging of [false,true])for(const moved of [false,true])
for(const cancelled of [false,true])for(const inside of [false,true])for(const sameSource of [false,true])
for(const clickAction of ['context_menu','toggle_fullscreen']){
  const expected=cancelled||!inside||!sameSource?'none':forceMenu?'menu':dragging||moved?'none':clickAction==='context_menu'?'menu':'click';
  assert.equal(videoReleaseIntent({forceMenu,dragging,moved,clickAction},cancelled,inside,sameSource),expected);
}
const media=readFileSync(new URL('../src/components/MediaSurface.tsx',import.meta.url),'utf8');
assert(media.includes('useSeekbar(state,sendCmd)')&&media.includes('seek.commitSeconds(gesture.targetTime)'),'video drag uses ordered decoder-confirmed seek path');
assert(media.includes('onAuxClick:')&&media.includes('event.preventDefault()'),'extra/middle clicks suppress browser navigation/autoscroll');
assert(media.includes('videoReleaseIntent(gesture')&&media.includes('open={contextMenuOpen}'),'context menu is release-classified, not automatically opened on every right gesture');
for(const file of ['StudioTab.tsx','SequenceTimeline.tsx','RemoteLocationDialog.tsx'])
  assert(readFileSync(new URL(`../src/components/${file}`,import.meta.url),'utf8').includes('useExtraMouse('),`${file} uses shared capture`);
console.log('Mouse bindings: scoped extra buttons, modifier priority, override pinning, movement/cancel/source guards, exact scrub path and controlled menus passed.');
