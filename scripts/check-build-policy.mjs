import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';

const cargo = await readFile(new URL('../Cargo.toml', import.meta.url), 'utf8');
for (const name of ['dev', 'test', 'release']) {
  const profile = cargo.match(new RegExp(`^\\[profile\\.${name}\\]\\s*\\n([\\s\\S]*?)(?=^\\[|$(?![\\s\\S]))`, 'm'))?.[1];
  assert.ok(profile, `Missing explicit ${name} build profile`);
  assert.match(profile, /^debug\s*=\s*0\s*$/m, `${name} must not generate routine debug symbols`);
  assert.match(profile, /^incremental\s*=\s*false\s*$/m, `${name} must not accumulate incremental snapshots`);
  assert.doesNotMatch(profile, /^(debug-assertions|overflow-checks|panic)\s*=/m,
    'Low-write defaults must not weaken test assertions, overflow checks or panic behavior');
}
assert.match(cargo, /^strip\s*=\s*"symbols"\s*$/m, 'Release symbols must stay stripped');
console.log('Low-write build profiles passed; no compilation performed.');
