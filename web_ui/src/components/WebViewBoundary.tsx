import React from 'react';
import { Alert, Button } from 'antd';
import { ReloadOutlined } from '@ant-design/icons';
import { tr, type UiLocale } from '../i18n';

/** An interrupted/update-invalidated lazy import must not leave a spinner forever. */
export class WebViewBoundary extends React.Component<
  { children: React.ReactNode; locale: UiLocale }, { failed: boolean }
> {
  state = { failed: false };
  static getDerivedStateFromError() { return { failed: true }; }
  componentDidCatch(error: Error) { console.error('Web view could not load', error); }
  render() {
    if (this.state.failed) return <Alert type="error" showIcon
      message={tr(this.props.locale, 'This view could not load')}
      action={<Button icon={<ReloadOutlined />} onClick={() => window.location.reload()}>{tr(this.props.locale, 'Reload')}</Button>} />;
    return this.props.children;
  }
}
