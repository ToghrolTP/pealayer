import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { createServer } from 'vite';
import react from '@vitejs/plugin-react';
import { createElement } from 'react';
import { renderToStaticMarkup } from 'react-dom/server';

const source=readFileSync(new URL('../src/components/RfSavedRuleCards.tsx',import.meta.url),'utf8');
const css=readFileSync(new URL('../src/rf-saved-rules.css',import.meta.url),'utf8');
// Use the project's actual Vite TSX/CSS/translation pipeline, not the removed
// TypeScript compiler JS API. No HTTP, WebSocket, watcher or production proxy.
const server=await createServer({root:fileURLToPath(new URL('..',import.meta.url)),configFile:false,
  appType:'custom',plugins:[react()],server:{middlewareMode:true,hmr:false,ws:false,watch:null},logLevel:'error'});
try {
assert.equal(server.httpServer,null,'SSR fixture must not listen on a production/UI port');
const {RfSavedRuleCards}=await server.ssrLoadModule('/src/components/RfSavedRuleCards.tsx');
const calls=[];
const gestures=['Tap','Hold','Repeat','Hold end'];
const rules=gestures.map((gestureLabel,index)=>({name:`rf.synthetic.${'x'.repeat(50)}${index}`,enabled:index%2===0,gestureLabel,
  summaries:[`Synthetic channel · ${index===3?'Remember brightness':index===0?'Toggle':'Roll brightness'}`]}));
const markup=renderToStaticMarkup(createElement(RfSavedRuleCards,{rules,disabled:true,locale:'en',
  onEdit:name=>calls.push(name),onEnabled:name=>calls.push(name)}));
assert.equal((markup.match(/class="[^"]*rf-saved-rule-card/g)??[]).length,4,'Four internal gesture rules must remain four editable rows');
const inputs=(markup.match(/<input\b[^>]*>/g)??[]).filter(input=>input.includes('type="checkbox"'));
assert.equal(inputs.length,4);
assert.equal(inputs.filter(input=>/\bchecked(?:=|\s|>)/.test(input)).length,2,'Both enabled and disabled bindings must retain actual checked state');
for(const input of inputs)assert.match(input,/\bdisabled(?:=|\s|>)/,'Pending/disconnected mutation guard must reach each actual checkbox');
for(const rule of rules){assert(markup.includes(rule.name));assert(markup.includes(rule.gestureLabel));assert(markup.includes(`Edit ${rule.name}`));assert(markup.includes(`Enabled ${rule.name}`));for(const summary of rule.summaries)assert(markup.includes(summary));}
const liveMarkup=renderToStaticMarkup(createElement(RfSavedRuleCards,{rules,disabled:false,locale:'en',
  onEdit:name=>calls.push(name),onEnabled:name=>calls.push(name)}));
const liveInputs=(liveMarkup.match(/<input\b[^>]*>/g)??[]).filter(input=>input.includes('type="checkbox"'));
assert.equal(liveInputs.length,4);
assert.equal(liveInputs.filter(input=>/\bchecked(?:=|\s|>)/.test(input)).length,2);
assert.equal(liveInputs.filter(input=>/\bdisabled(?:=|\s|>)/.test(input)).length,0,'A live non-pending manager must not lock all mutation controls');
const buttons=markup.match(/<button\b[^>]*>/g)??[];
assert.equal(buttons.length,4);
assert.equal(buttons.filter(button=>/\bdisabled(?:=|\s|>)/.test(button)).length,0,'Edit remains a local draft action while mutation is disabled');
assert.equal(calls.length,0,'Rendering cannot enable, save, learn or edit a binding');
assert.match(css,/grid-template-columns: minmax\(0, 1fr\) auto/,'Edit gets an independent reachable column');
assert.match(css,/\.rf-saved-rule-summary, \.rf-saved-rule-id[^}]*white-space: normal;[^}]*overflow-wrap: anywhere/,'Readable actions and long IDs must wrap');
assert.doesNotMatch(css,/overflow:\s*hidden|text-overflow:\s*ellipsis|width:\s*520px/,'Do not hide data or force the wide-table width');
assert.doesNotMatch(source,/sendCmd|fetch\(|setInterval|localStorage/,'Standalone presentation must not introduce transport or persistence');
console.log('RF saved-rule SSR: full four-rule contents, reachable Edit, wrapping contract and no side effects passed.');
} finally {
  await server.close();
}
