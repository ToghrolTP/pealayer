import { useEffect, useState } from 'react';
import { CheckCircleOutlined, CloseOutlined, InfoCircleOutlined, WarningOutlined } from '@ant-design/icons';

import { visibleToasts, type ToastSnapshot } from '../messaging';
import { tr, type UiLocale } from '../i18n';
export function SharedToasts({ snapshot, connected, dismiss, locale = 'en' }: { snapshot?: ToastSnapshot; connected: boolean; dismiss: (id: string) => void; locale?: UiLocale }) {
  const [now, setNow] = useState(Date.now());
  useEffect(() => {
    const expiries = (snapshot?.toasts || []).flatMap(t => t.expires_at_ms === null ? [] : [t.expires_at_ms]).filter(t => t > Date.now());
    if (!expiries.length) return;
    const timer = window.setTimeout(() => setNow(Date.now()), Math.max(1, Math.min(...expiries) - Date.now()));
    return () => window.clearTimeout(timer);
  }, [snapshot, now]);
  if (!connected) return null;
  const toasts = visibleToasts(snapshot, Math.max(now, Date.now()), connected);
  return <aside className="shared-toasts" aria-label="Notifications">
    {toasts.map(t => <section key={`${snapshot?.instance_id}:${t.id}`} className={`shared-toast shared-toast-${t.severity}`} role={t.severity === 'error' || t.severity === 'warning' ? 'alert' : 'status'}>
      <span className="shared-toast-icon">{t.severity === 'success' ? <CheckCircleOutlined /> : t.severity === 'warning' || t.severity === 'error' ? <WarningOutlined /> : <InfoCircleOutlined />}</span>
      <div>{t.title && <strong>{t.title}</strong>}<p>{t.message}</p>{t.details && <details><summary>{tr(locale, 'Details')}</summary><p style={{whiteSpace:'pre-wrap',overflowWrap:'anywhere',maxHeight:180,overflowY:'auto'}}>{t.details}</p></details>}</div>
      <button type="button" aria-label="Dismiss notification" title="Dismiss notification" onClick={() => dismiss(t.id)}><CloseOutlined /></button>
    </section>)}
  </aside>;
}
