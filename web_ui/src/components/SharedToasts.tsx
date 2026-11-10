import { useEffect, useRef, useState } from 'react';
import { Modal } from 'antd';
import { CheckCircleOutlined, CloseOutlined, CopyOutlined, InfoCircleOutlined, WarningOutlined } from '@ant-design/icons';
import { visibleToasts, type ToastSnapshot } from '../messaging';
import { tr, type UiLocale } from '../i18n';
type Toast = ToastSnapshot['toasts'][number];
const diagnostic = (t: Toast) => `${t.title}\n${t.message}\n\nSource: ${t.source}\nTime: ${t.created_at_ms}\n\n${t.details || t.message}`;
export function SharedToasts({ snapshot, connected, dismiss, invoke, locale = 'en' }: { snapshot?: ToastSnapshot; connected: boolean; dismiss: (id: string) => void; invoke: (id: string, created: number, action: string) => void; locale?: UiLocale }) {
  const [now, setNow] = useState(Date.now());
  const [details, setDetails] = useState<Toast | null>(null);
  const [copyError, setCopyError] = useState('');
  const activated = useRef(new Set<string>());
  useEffect(() => {
    activated.current.clear();
    setDetails(null);
  }, [snapshot?.instance_id]);
  useEffect(() => {
    const expiries = (snapshot?.toasts || []).flatMap(t => t.expires_at_ms === null ? [] : [t.expires_at_ms]).filter(t => t > Date.now());
    if (!expiries.length) return;
    const timer = window.setTimeout(() => setNow(Date.now()), Math.max(1, Math.min(...expiries) - Date.now()));
    return () => window.clearTimeout(timer);
  }, [snapshot, now]);
  useEffect(() => {
    const live = new Set((snapshot?.toasts || []).map(t => `${t.id}:${t.created_at_ms}`));
    for (const key of activated.current) if (!live.has(key)) activated.current.delete(key);
  }, [snapshot]);
  const activate = (t: Toast, action: string) => {
    const key = `${t.id}:${t.created_at_ms}`;
    if (!connected || activated.current.has(key)) return;
    activated.current.add(key);
    invoke(t.id, t.created_at_ms, action);
  };
  const copy = async (t: Toast) => {
    try { await navigator.clipboard.writeText(diagnostic(t)); setCopyError(''); }
    catch { setCopyError('Clipboard unavailable. Select and copy the diagnostic text.'); setDetails(t); }
  };
  const toasts = visibleToasts(snapshot, Math.max(now, Date.now()), connected);
  return <>
    <aside className="shared-toasts" aria-label="Notifications">
      {toasts.map(t => <section key={`${snapshot?.instance_id}:${t.id}:${t.created_at_ms}`} className={`shared-toast shared-toast-${t.severity}`} role={t.severity === 'error' || t.severity === 'warning' ? 'alert' : 'status'}>
        <span className="shared-toast-icon">{t.severity === 'success' ? <CheckCircleOutlined /> : t.severity === 'warning' || t.severity === 'error' ? <WarningOutlined /> : <InfoCircleOutlined />}</span>
        <div>
          {t.default_action || t.severity === 'error' || t.details ?
            <button className="toast-body" type="button" title={t.actions?.find(a => a.id === t.default_action)?.label || tr(locale, 'Details')} onClick={() => t.default_action ? activate(t, t.default_action) : setDetails(t)}>{t.title && <strong>{t.title}</strong>}<p>{t.message}</p></button> :
            <div>{t.title && <strong>{t.title}</strong>}<p>{t.message}</p></div>}
          {!!t.actions?.length && <div className="toast-bound-actions">{t.actions.map(a => <button type="button" key={a.id} onClick={() => activate(t, a.id)}>{a.label}</button>)}</div>}
          {(t.severity === 'error' || t.details) && <div className="toast-error-actions">
            <button type="button" aria-label="Copy error" title="Copy error" onClick={() => void copy(t)}><CopyOutlined /></button>
            <button type="button" aria-label={tr(locale, 'Details')} title={tr(locale, 'Details')} onClick={() => { setCopyError(''); setDetails(t); }}><InfoCircleOutlined /></button>
          </div>}
        </div>
        <button type="button" aria-label="Dismiss notification" title="Dismiss notification" onClick={() => dismiss(t.id)}><CloseOutlined /></button>
      </section>)}
    </aside>
    <Modal open={details !== null} title={tr(locale, 'Details')} onCancel={() => setDetails(null)} footer={details ? <button type="button" onClick={() => void copy(details)}><CopyOutlined /> Copy error</button> : null}>
      {copyError && <p role="alert">{copyError}</p>}
      <pre className="toast-diagnostic">{details && diagnostic(details)}</pre>
    </Modal>
  </>;
}
