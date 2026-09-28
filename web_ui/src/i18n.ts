export type UiLocale = 'en' | 'fa';

const fa: Record<string, string> = {
  'Remote Control': 'کنترل از راه دور',
  'Media Library': 'کتابخانه رسانه',
  'System Info': 'اطلاعات سامانه',
  'Control Center': 'مرکز کنترل',
  'WebSocket Live': 'وب سوکت زنده',
  'HTTP Polling': 'پایش HTTP',
  'Offline': 'آفلاین',
  'Initializing…': 'در حال راه اندازی…',
};

export function tr(locale: UiLocale, text: string): string {
  return locale === 'fa' ? (fa[text] ?? text) : text;
}
