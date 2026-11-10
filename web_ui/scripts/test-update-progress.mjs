import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';

// Contract wiring: Rust tests exercise all update phases and serialized values.
// Web must consume that value, never derive phase progress from transfer bytes.
const component=readFileSync(new URL('../src/components/PlayerInfoTab.tsx',import.meta.url),'utf8');
assert.match(component,/updateStatus\?\.progress_percent \?\? undefined/);
assert.doesNotMatch(component,/bytes_done\s*\*\s*100/);
assert.match(component,/updateStatus.active && updatePercent === undefined && <Spin/);
assert.match(component,/updatePercent !== undefined && <Progress/);
assert.match(component,/if \(!updateStatus\?\.active\) return/);
const contract=readFileSync(new URL('../src/components/RemoteControlTab.tsx',import.meta.url),'utf8');
assert.match(contract,/progress_percent: number \| null/);
assert.match(contract,/active: boolean/);
console.log('Updater Web contract: server phase progress, indeterminate spinner, no stale transfer percentage.');
