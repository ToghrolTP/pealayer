import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {mediaControlsAllowed} from '../src/mediaControlsPolicy.ts';
for(const enabled of [false,true])for(const background of [false,true])for(const foreground of [false,true]){
  assert.equal(mediaControlsAllowed(enabled,background,foreground),enabled&&(background||foreground));
}
assert.equal(mediaControlsAllowed(undefined,true,true),false,'no invented policy before authoritative snapshot');
assert.equal(mediaControlsAllowed(true,true,false),true,'background/minimized control remains available');
assert.equal(mediaControlsAllowed(true,false,false),false);
assert.equal(mediaControlsAllowed(true,false,true),true);
const source=readFileSync(new URL('../src/webPlatform.ts',import.meta.url),'utf8');
assert(source.includes('mediaPolicyRef.current')&&source.includes("document.visibilityState==='visible'&&document.hasFocus()"),'dispatch reads live preference/focus instead of a stale installed callback');
assert(source.includes('active=false')&&source.includes('setActionHandler(action,null)'),'cleanup invalidates retained callbacks');
assert(source.includes("['stop', () => dispatchCommand('stop')]"),'OS Stop is a real Stop, not Pause');
assert(source.includes("['previoustrack', () => dispatchCommand('previous')]")&&source.includes("['nexttrack', () => dispatchCommand('next')]"),'track actions match native OS media-session semantics');
console.log('Media controls: master/background/focus policy, stale callbacks and shared commands passed.');
