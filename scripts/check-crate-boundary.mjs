import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';

const library = await readFile(new URL('../src/lib.rs', import.meta.url), 'utf8');
const binary = await readFile(new URL('../src/main.rs', import.meta.url), 'utf8');
const build = await readFile(new URL('../build.rs', import.meta.url), 'utf8');
const modules = source => [...source.matchAll(/^\s*(?:pub\s+)?mod\s+(\w+)\s*;/gm)].map(match => match[1]);
const shared = new Set(modules(library));
assert.deepEqual(modules(binary).filter(name => shared.has(name)), [],
  'The binary must reuse library modules, not compile a second set of contracts and process globals');
assert.match(binary, /pealayer::startup::run\(\)/, 'The executable must delegate startup to the authoritative library');
assert.equal(modules(binary).length, 0, 'The executable must remain a thin entry point');
assert.doesNotMatch(build, /println!\([^\n]*rustc-link-arg-bin[^\n]*\)[\s\S]{0,100}resource\.lib/,
  'MSVC resources must not be linked twice through the library and binary');
assert.doesNotMatch(build, /join\("resource\.lib"\)/,
  'winres already links the MSVC resource object through the shared library');
console.log(`Single-crate contract guard passed (${shared.size} shared modules).`);
