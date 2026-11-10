import React, { useEffect, useState } from 'react';
import { Input } from 'antd';
import './shortcutInput.css';

export function shortcutKeys(value: string): string[] {
  const keys = value.split('+').map(key => key.trim()).filter(Boolean);
  if (value.trimEnd().endsWith('++')) keys.push('+');
  return keys;
}

/** The actual text input retains keyboard, paste and accessible value semantics. */
export function ShortcutInput({ value, label, placeholder, onCommit }: {
  value: string; label: string; placeholder?: string; onCommit: (value: string) => void;
}) {
  const [text, setText] = useState(value);
  const [editing, setEditing] = useState(false);
  useEffect(() => { if (!editing) setText(value); }, [value, editing]);
  return <span className={`shortcut-input${text.trim() ? ' has-value' : ''}`}>
    <Input value={text} aria-label={label} placeholder={placeholder} spellCheck={false}
      title={text} onFocus={() => setEditing(true)} onChange={event => setText(event.target.value)}
      onPressEnter={event => event.currentTarget.blur()}
      onBlur={() => { setEditing(false); if (text !== value) onCommit(text.trim()); }} />
    <span className="shortcut-input__caps" aria-hidden="true" dir="ltr">
      {shortcutKeys(text).map((key, index) => <React.Fragment key={index}>
        {index > 0 && <span className="shortcut-input__join">+</span>}<kbd>{key}</kbd>
      </React.Fragment>)}
    </span>
  </span>;
}
