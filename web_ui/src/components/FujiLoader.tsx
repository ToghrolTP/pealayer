import { tr, type UiLocale } from '../i18n';

/** A real pending state, not a fabricated progress meter or timed splash. */
export function FujiLoader({ locale = 'en' }: { locale?: UiLocale }) {
  return <div className="fuji-loader" role="status" aria-live="polite">
    <img src="/fuji-loader.svg" alt="" aria-hidden="true" width="232" height="131" />
    <span>{tr(locale, 'Loading…')}</span>
  </div>;
}
