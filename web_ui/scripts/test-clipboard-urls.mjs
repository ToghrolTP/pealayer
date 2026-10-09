import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

const file = readFileSync(new URL('../src/clipboardUrls.ts', import.meta.url), 'utf8');
const helper = file.slice(file.indexOf('export function clipboardUrl'), file.indexOf('// Browsers'));
// This pure helper has no TS syntax beyond its signature. The production tsc
// gate validates types; exercise the exact body, not a copied implementation.
const js = helper.replace('export function', 'function').replace('(text: string): string | undefined', '(text)');
const normalize = new Function(`${js}; return clipboardUrl;`)();
assert.equal(normalize(' https://files.example/show/#top '), 'https://files.example/show/');
for (const value of ['plain text', 'file:///secret', 'https://name:secret@files.example/', 'ftp://files.example/', 'https://files.example/\nhttps://other.example/', 'https://files.example/' + 'x'.repeat(8192)]) assert.equal(normalize(value), undefined);
assert(file.includes("permission.state !== 'granted'"), 'automatic polling must not prompt for browser permission');
const dialog = readFileSync(new URL('../src/components/RemoteLocationDialog.tsx', import.meta.url), 'utf8');
assert(dialog.includes('delay: 250'));
assert(dialog.includes('Back · Parent folder'));
assert(dialog.includes('override.current'));
assert(dialog.includes('}, 500)'));
console.log('Clipboard URL validation, permission boundary and remote-dialog guardrails passed.');
