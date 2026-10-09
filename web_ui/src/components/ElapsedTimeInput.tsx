import { useEffect, useLayoutEffect, useRef, useState } from 'react';
import { tr, UiLocale } from '../i18n';
import { editTimecode, formatEditTime, parseEditTime, shouldCommitTime, timecodeGroups } from '../timecodeEditor';
import type { TimecodeDraft } from '../timecodeEditor';

export function ElapsedTimeInput({ seconds, disabled, mediaIdentity, locale, onCommit, className = '' }: {
  seconds: number; disabled: boolean; mediaIdentity?: string | null; locale: UiLocale;
  onCommit: (seconds: number) => void; className?: string;
}) {
  const input = useRef<HTMLInputElement>(null);
  const original = useRef('');
  const editing = useRef<TimecodeDraft | null>(null);
  const [draft, setDraft] = useState<TimecodeDraft | null>(null);
  const update = (next: TimecodeDraft | null) => { editing.current = next; setDraft(next); };
  const begin = () => {
    if (disabled || editing.current) return;
    original.current = formatEditTime(seconds);
    update({ value: original.current, group: 0, digits: 0 });
  };
  const finish = (enter: boolean, escape = false) => {
    const current = editing.current;
    // Clear synchronously before blur so Enter/Escape cannot commit twice.
    update(null);
    if (current && shouldCommitTime(original.current, current.value, enter, !enter, escape)) {
      const value = parseEditTime(current.value);
      if (value !== null) onCommit(value);
    }
  };
  useEffect(() => { update(null); }, [mediaIdentity, disabled]);
  useLayoutEffect(() => {
    if (draft && document.activeElement === input.current) {
      const [start, end] = timecodeGroups[draft.group];
      input.current?.setSelectionRange(start, end);
    }
  }, [draft]);
  return <input
    ref={input}
    className={`elapsed-time-input ${className}`}
    aria-label={tr(locale, 'Elapsed time')}
    title={tr(locale, 'Type digits in each time group; Left/Right switches groups. Enter seeks; Escape cancels.')}
    inputMode="numeric"
    dir="ltr"
    spellCheck={false}
    autoComplete="off"
    disabled={disabled}
    readOnly={!draft}
    value={draft?.value ?? formatEditTime(seconds)}
    onFocus={begin}
    onBlur={() => finish(false)}
    onClick={() => {
      begin();
      const current = editing.current;
      if (current) {
        const index = input.current?.selectionStart ?? 0;
        const group = timecodeGroups.findIndex(([, end]) => index <= end);
        update({ ...current, group: group < 0 ? 3 : group, digits: 0 });
      }
    }}
    onKeyDown={(event) => {
      if (!editing.current || event.key === 'Tab') return;
      event.stopPropagation();
      if ((event.ctrlKey || event.metaKey) && ['c', 'v', 'a'].includes(event.key.toLowerCase())) return;
      event.preventDefault();
      if (event.key === 'Escape' || event.key === 'Enter') {
        finish(event.key === 'Enter', event.key === 'Escape'); input.current?.blur();
      } else if (!event.nativeEvent.isComposing && (event.key.length === 1 ||
        ['ArrowLeft', 'ArrowRight', 'ArrowUp', 'ArrowDown', 'Home', 'End', 'Backspace', 'Delete'].includes(event.key))) {
        update(editTimecode(editing.current, event.key));
      }
    }}
    onBeforeInput={(event) => {
      event.preventDefault();
      const native = event.nativeEvent as InputEvent;
      if (editing.current && native.data && !native.isComposing) update(editTimecode(editing.current, native.data));
    }}
    onCompositionEnd={(event) => { if (editing.current) update(editTimecode(editing.current, event.data)); }}
    onChange={(event) => { event.target.value = editing.current?.value ?? formatEditTime(seconds); }}
    onCut={(event) => { event.preventDefault(); event.clipboardData.setData('text/plain', editing.current?.value ?? formatEditTime(seconds)); }}
    onPaste={(event) => {
      event.preventDefault(); begin();
      const value = parseEditTime(event.clipboardData.getData('text/plain'));
      if (value !== null) update({ value: formatEditTime(value), group: 3, digits: 0 });
    }}
  />;
}
