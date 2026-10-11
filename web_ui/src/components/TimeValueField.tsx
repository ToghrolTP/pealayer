import { useEffect, useRef, useState } from 'react';
import { Input } from 'antd';
import { formatTimeMs, parseTimeMs } from '../cueAuthoring';

export function TimeValueField({ value, onChange, min = 0, max = 3600000, human = true, label }: {
  value: number; onChange: (value: number) => void; min?: number; max?: number; human?: boolean; label?: string;
}) {
  const [text, setText] = useState(formatTimeMs(value, human));
  const [invalid, setInvalid] = useState(false);
  const editing = useRef(false);
  const canceled = useRef(false);
  useEffect(() => { if (!editing.current) setText(formatTimeMs(value, human)); }, [value, human]);
  const commit = () => {
    editing.current = false;
    if (canceled.current) { canceled.current = false; setText(formatTimeMs(value, human)); setInvalid(false); return; }
    const parsed = parseTimeMs(text);
    if (parsed === null || parsed < min || parsed > max) { setInvalid(true); return; }
    onChange(parsed); setText(formatTimeMs(parsed, human)); setInvalid(false);
  };
  return <Input aria-label={label} value={text} status={invalid ? 'error' : undefined}
    onFocus={() => { editing.current = true; }} onChange={event => { setText(event.target.value); setInvalid(false); }} onBlur={commit}
    onPressEnter={event => event.currentTarget.blur()} onKeyDown={event => {
      if (event.key === 'Escape') { canceled.current = true; event.currentTarget.blur(); event.stopPropagation(); }
    }} />;
}
