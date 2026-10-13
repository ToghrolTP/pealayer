import React, { useEffect, useState } from 'react';
import { Button, Card, Select, Space, Typography } from 'antd';
import { tr, UiLocale } from '../i18n';
import { pwmPolicyPayload } from '../pwmPolicy';

type Policy = 'full' | 'restore';
type Config = { on_behavior?: Policy | null; remembered_raw?: number | null; remembered_percent?: number | null };
export function PwmPolicyEditor({ config, channel, locale, disabled, sendCmd }: {
  config?: Config | null; channel?: number | null; locale: UiLocale; disabled: boolean;
  sendCmd: (command: string, payload?: Record<string, unknown>) => Promise<boolean>;
}) {
  const current = config?.on_behavior;
  const [draft, setDraft] = useState<Policy>(current === 'restore' ? 'restore' : 'full');
  const [pending, setPending] = useState(false);
  useEffect(() => { if (current) setDraft(current); }, [current]);
  if (!current || channel == null) return <Typography.Text type="secondary">
    {tr(locale, 'Shared On policy is not advertised for this channel')}
  </Typography.Text>;
  return <Card size="small" title={tr(locale, 'Shared On behavior')}>
    <Typography.Paragraph type="secondary">{tr(locale, 'Applies to all connected PCController clients')}</Typography.Paragraph>
    <Space wrap>
      <Select aria-label={tr(locale, 'Shared On behavior')} value={draft} onChange={setDraft} disabled={disabled || pending}
        style={{ minWidth: 210 }} options={[
          { value: 'full', label: tr(locale, 'Full brightness (100%)') },
          { value: 'restore', label: tr(locale, 'Restore last brightness') },
        ]} />
      <Button type="primary" loading={pending} disabled={disabled || pending || draft === current} onClick={() => {
        setPending(true);
        void sendCmd('hardware.pwm.policy.set', pwmPolicyPayload(channel, draft)).finally(() => setPending(false));
      }}>{tr(locale, 'Apply policy')}</Button>
    </Space>
    <Typography.Paragraph type="secondary" style={{ marginTop: 10, marginBottom: 4 }}>
      {config?.remembered_percent != null
        ? `${tr(locale, 'Remembered brightness')}: ${config.remembered_percent.toFixed(1)}%`
        : tr(locale, 'Unknown brightness falls back to 100%')}
    </Typography.Paragraph>
    <Typography.Text type="secondary">{tr(locale, 'Brightness memory resets when PCController restarts or the board reconnects')}</Typography.Text>
  </Card>;
}
