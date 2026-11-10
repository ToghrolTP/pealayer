/** Current VT key encoding, shared by the Web terminal's prompt and tables. */
export type TerminalKey = {key:string;shiftKey?:boolean;altKey?:boolean;ctrlKey?:boolean;metaKey?:boolean};
export function terminalKey(event:TerminalKey,applicationCursor=false):string|null {
  const {key,shiftKey,altKey,ctrlKey,metaKey}=event;
  if(metaKey)return null;
  const modifier=1+(shiftKey?1:0)+(altKey?2:0)+(ctrlKey?4:0);
  const arrows:Record<string,string>={ArrowUp:'A',ArrowDown:'B',ArrowRight:'C',ArrowLeft:'D',Home:'H',End:'F'};
  if(arrows[key])return modifier>1?`\x1b[1;${modifier}${arrows[key]}`:`\x1b${applicationCursor?'O':'['}${arrows[key]}`;
  const functions:Record<string,string>={F1:'P',F2:'Q',F3:'R',F4:'S'};
  if(functions[key])return modifier>1?`\x1b[1;${modifier}${functions[key]}`:`\x1bO${functions[key]}`;
  const tilde:Record<string,number>={Insert:2,Delete:3,PageUp:5,PageDown:6,F5:15,F6:17,F7:18,F8:19,F9:20,F10:21,F11:23,F12:24};
  if(tilde[key])return `\x1b[${tilde[key]}${modifier>1?`;${modifier}`:''}~`;
  if(key==='Tab')return shiftKey?'\x1b[Z':'\t';
  if(key==='Enter')return '\r';
  if(key==='Escape')return '\x1b';
  if(key==='Backspace')return altKey?'\x1b\x7f':ctrlKey?'\x17':'\x7f';
  if(ctrlKey && key===' ')return '\0';
  if(ctrlKey && /^[a-z@[\\\]^_]$/i.test(key))return `${altKey?'\x1b':''}${String.fromCharCode(key.toUpperCase().charCodeAt(0)&31)}`;
  if(!ctrlKey && [...key].length===1)return `${altKey?'\x1b':''}${key}`;
  return null;
}
