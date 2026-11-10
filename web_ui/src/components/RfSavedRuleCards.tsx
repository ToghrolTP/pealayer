import { Button, Card, Checkbox, Typography } from 'antd';
import { EditOutlined } from '@ant-design/icons';
import { tr, type UiLocale } from '../i18n';
import '../rf-saved-rules.css';

export type RfSavedRule = { name: string; enabled: boolean; gestureLabel: string; summaries: readonly string[] };

/** Presentation only. The owning manager retains waveform, draft and RPC semantics. */
export function RfSavedRuleCards({ rules, disabled, locale, onEdit, onEnabled }: {
  rules: readonly RfSavedRule[]; disabled: boolean; locale: UiLocale;
  onEdit: (name: string) => void; onEnabled: (name: string, enabled: boolean) => void;
}) {
  return <div className="rf-saved-rule-list">
    {rules.map(rule => <Card key={rule.name} size="small" className="rf-saved-rule-card">
      <header className="rf-saved-rule-header">
        <Checkbox checked={rule.enabled} disabled={disabled}
          aria-label={`${tr(locale, 'Enabled')} ${rule.name}`}
          onChange={event => onEnabled(rule.name, event.target.checked)}>
          <strong className="rf-saved-rule-caption">{rule.gestureLabel}</strong>
        </Checkbox>
        <Button className="rf-saved-rule-edit" size="small" icon={<EditOutlined />}
          aria-label={`${tr(locale, 'Edit')} ${rule.name}`} onClick={() => onEdit(rule.name)}>{tr(locale, 'Edit')}</Button>
      </header>
      <div className="rf-saved-rule-details">
        {rule.summaries.map((summary, index) => <Typography.Text key={index} className="rf-saved-rule-summary" title={summary}>{summary}</Typography.Text>)}
        <Typography.Text type="secondary" className="rf-saved-rule-id" title={rule.name}>{rule.name}</Typography.Text>
      </div>
    </Card>)}
  </div>;
}
