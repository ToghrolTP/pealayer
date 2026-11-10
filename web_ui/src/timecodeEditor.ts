export const timecodeGroups = [[0, 2], [3, 5], [6, 8], [9, 12]] as const;
export interface TimecodeDraft { value: string; group: number; digits: number; }
const maxima = [99, 59, 59, 999];
export const normalizeTimecodeDigits = (text: string) => text.replace(/[۰-۹٠-٩٫]/g, (character) =>
  character === '٫' ? '.' : String(character.charCodeAt(0) - (character >= '۰' ? 0x6f0 : 0x660)));

export function formatEditTime(seconds: number) {
  const milliseconds = Math.round(Math.max(0, Math.min(359999.999, Number.isFinite(seconds) ? seconds : 0)) * 1000);
  const pad = (value: number, width = 2) => String(value).padStart(width, '0');
  return `${pad(Math.floor(milliseconds / 3600000))}:${pad(Math.floor(milliseconds / 60000) % 60)}:${pad(Math.floor(milliseconds / 1000) % 60)}.${pad(milliseconds % 1000, 3)}`;
}

export function parseEditTime(text: string): number | null {
  const value = normalizeTimecodeDigits(text).trim();
  if (!/^\d+(?::\d+){0,2}(?:\.\d+)?$/.test(value)) return null;
  const fields = value.split(':').map(Number);
  if (fields.length > 1 && fields[fields.length - 1] >= 60) return null;
  if (fields.length === 3 && fields[1] >= 60) return null;
  const seconds = fields.reduce((total, field) => total * 60 + field, 0);
  return Number.isFinite(seconds) && seconds <= 359999.999 ? seconds : null;
}

export const shouldCommitTime = (original: string, value: string, enter: boolean, blur: boolean, escape: boolean) =>
  !escape && (enter || (blur && value !== original));

export function editTimecode(draft: TimecodeDraft, key: string): TimecodeDraft {
  let { value, group, digits } = draft;
  const replace = (next: number) => {
    const [start, end] = timecodeGroups[group];
    value = value.slice(0, start) + String(next).padStart(end - start, '0') + value.slice(end);
  };
  if (key === 'ArrowLeft' || key === 'ArrowRight' || key === 'Home' || key === 'End') {
    group = key === 'Home' ? 0 : key === 'End' ? 3 : Math.max(0, Math.min(3, group + (key === 'ArrowLeft' ? -1 : 1)));
    digits = 0;
  } else if (key === 'Delete') { replace(0); digits = 0; }
  else if (key === 'Backspace') {
    if (digits === -1) { group = Math.max(0, group - 1); digits = timecodeGroups[group][1] - timecodeGroups[group][0]; }
    const [start, end] = timecodeGroups[group];
    replace(digits > 0 ? Math.floor(Number(value.slice(start, end)) / 10) : 0);
    digits = Math.max(0, digits - 1);
  } else if (key === 'ArrowUp' || key === 'ArrowDown') {
    const [start, end] = timecodeGroups[group];
    replace(Math.max(0, Math.min(maxima[group], Number(value.slice(start, end)) + (key === 'ArrowUp' ? 1 : -1))));
    digits = 0;
  } else {
    for (const character of normalizeTimecodeDigits(key)) {
      if (character === ':' || character === '.') {
        if (digits !== -1) group = Math.min(3, group + 1);
        digits = 0;
      } else if (/^[0-9]$/.test(character)) {
        const [start, end] = timecodeGroups[group];
        if (digits === -1 || digits >= end - start) digits = 0;
        replace(Math.min(maxima[group], (digits === 0 ? 0 : Number(value.slice(start, end))) * 10 + Number(character)));
        digits++;
        if (digits >= end - start && group < 3) { group++; digits = -1; }
      }
    }
  }
  return { value, group, digits };
}
