import { useEffect, useRef } from 'react';

export function clipboardUrl(text: string): string | undefined {
  const trimmed = text.trim();
  if (!trimmed || trimmed.length > 8192 || /[\r\n]/.test(trimmed)) return;
  try {
    const url = new URL(trimmed);
    if (!['http:', 'https:'].includes(url.protocol) || !url.hostname || url.username || url.password) return;
    url.hash = ''; return url.href;
  } catch { return; }
}

// Browsers never grant unrestricted OS clipboard access. Observe foreground
// changes only when readText is allowed; trusted paste remains a fallback.
export function useClipboardUrls(enabled: boolean, connected: boolean, browse: (url: string) => void) {
  const seen = useRef(new Set<string>());
  const last = useRef<string | undefined>(undefined);
  const baseline = useRef(false);
  useEffect(() => {
    if (!enabled || !connected) { baseline.current = false; return; }
    let active = true; let reading = false;
    const observe = (text: string, explicit = false) => {
      if (!active) return;
      const value = text.slice(0, 8193); const changed = last.current !== value;
      last.current = value;
      if (!baseline.current && !explicit) { baseline.current = true; return; }
      baseline.current = true;
      const url = clipboardUrl(text);
      if (changed && url && !seen.current.has(url)) {
        seen.current.add(url);
        if (seen.current.size > 128) seen.current.delete(seen.current.values().next().value!);
        browse(url);
      }
    };
    const read = async () => {
      if (!active || reading || document.visibilityState !== 'visible' || !document.hasFocus() || !navigator.clipboard?.readText) return;
      reading = true;
      try {
        if (navigator.permissions?.query) {
          try {
            const permission = await navigator.permissions.query({ name: 'clipboard-read' as PermissionName });
            if (permission.state !== 'granted') { baseline.current = true; return; }
          } catch { /* Firefox/Safari may not expose this permission name. */ }
        }
        observe(await navigator.clipboard.readText());
      } catch { baseline.current = true; /* Permission/secure-context boundary; no prompts or retry storms. */ }
      finally { reading = false; }
    };
    const pasted = (event: ClipboardEvent) => { if (event.isTrusted && event.clipboardData) observe(event.clipboardData.getData('text/plain'), true); };
    const focus = () => { void read(); };
    window.addEventListener('focus', focus); document.addEventListener('visibilitychange', focus); document.addEventListener('paste', pasted);
    const timer = window.setInterval(focus, 2000); void read();
    return () => { active = false; clearInterval(timer); window.removeEventListener('focus', focus); document.removeEventListener('visibilitychange', focus); document.removeEventListener('paste', pasted); };
  }, [enabled, connected, browse]);
}
