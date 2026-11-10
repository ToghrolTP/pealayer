import assert from 'node:assert/strict';
import { preferenceFileAllowed, preferenceFileParent } from '../src/preferenceFiles.ts';
const allowed = ['png', 'jpg', 'jpeg', 'webp', 'ico'];
for (const name of ['icon.PNG', 'pause.jpg', 'play.jpeg', 'stopped.webp', 'app.ico']) {
  assert.equal(preferenceFileAllowed(name, allowed), true, name);
}
for (const name of ['image.png.exe', 'file', 'image.svg', 'icon.png/child']) {
  assert.equal(preferenceFileAllowed(name, allowed), false, name);
}
assert.equal(preferenceFileParent('C:\\icon.png'), 'C:\\');
assert.equal(preferenceFileParent('C:\\images\\icon.PNG'), 'C:\\images\\');
assert.equal(preferenceFileParent('\\\\server\\share\\icon.png'), '\\\\server\\share\\');
assert.equal(preferenceFileParent('/icon.png'), '/');
assert.equal(preferenceFileParent('/images/icon.png'), '/images/');
assert.equal(preferenceFileParent('icon.png'), '');
assert.equal(preferenceFileParent(undefined), '');
assert.equal(preferenceFileAllowed('icon.png', []), false);
console.log('Preference file picker: extension filtering and Windows/Unix parent paths passed.');
