import React, { useEffect, useState } from 'react';
import { Alert, Button, Card, Checkbox, Collapse, Dropdown, Empty, Form, Input, InputNumber, Modal, Select, Space, Table, Tabs, Tag, Typography } from 'antd';
import { ApiOutlined, DeleteOutlined, EditOutlined, MoreOutlined, PlusOutlined, ReloadOutlined, SaveOutlined, SendOutlined } from '@ant-design/icons';

type Action = { type: string; app_kind?: string; app_target?: string; app_value?: string; action_id?: string; command?: string; macro?: string; virtual_key?: string; hold_ms?: number; executable?: string; script?: string; detached?: boolean; args?: string[]; event?: string; rf?: Waveform };
type Waveform = { code: number; bits: number; protocol: number; pulse_us: number; repeats?: number };
type Binding = { name: string; enabled: boolean; cooldown_ms: number; match: { kind: string; source: string; rf_code: number; rf_bits: number; rf_protocol: number; gesture: string }; actions: Action[] };
type Entry = Waveform & { id: number; name?: string; code_display: string; action_kind: number; action_value: number; behavior: number };
type Application = { id: string; surface: string; values?: { app_actions?: string } };
type MappingOption = { action: string; targets: string[]; behaviors: string[] };
type Catalog = { connected?: boolean; entries?: Entry[]; entries_sampled?: boolean; board_error?: string; bindings?: Binding[]; learning?: { active: boolean; remaining_ms: number; learned: number }; activity?: { id: number; text: string; time: string; rf_code: number; rf_bits: number; rf_protocol: number }[]; hostname?: string; action_types?: string[]; gestures?: string[]; applications?: Application[]; effects?: { name: string; reference: string }[]; peripherals?: { controls: { name: string; actions: { id: string; name?: string; verb?: string }[] }[] }; board_mapping_options?: MappingOption[] };
export type RfSnapshot = { catalog?: Catalog; pending?: boolean; error?: string; last_result?: unknown };
type Props = { rf?: RfSnapshot; sendCmd: (command: string, payload?: Record<string, unknown>) => void };
const blank = (): Binding => ({ name: '', enabled: true, cooldown_ms: 250, match: { kind: 'rf.gesture', source: 'rf', rf_code: 0, rf_bits: 24, rf_protocol: 1, gesture: 'down' }, actions: [{ type: 'app', app_target: 'pealayer', app_kind: 'pealayer.toggle', app_value: '' }] });
const labels: Record<string, string> = { app: 'Application', control: 'Board control', board: 'Controller command', effect: 'Effect', rf: 'RF transmit', 'virtual-key': 'Controller host keyboard', host: 'External program', script: 'Script', emit: 'Event' };
const human = (key: string) => key.replace(/[._-]/g, ' ').replace(/^pealayer /, '').replace(/^./, s => s.toUpperCase());

export const RfManager: React.FC<Props> = ({ rf, sendCmd }) => {
  const [open, setOpen] = useState(false);
  const [tab, setTab] = useState('assignments');
  const [draft, setDraft] = useState<Binding>(blank);
  const [previousName, setPreviousName] = useState('');
  const [allowKeyboard, setAllowKeyboard] = useState(false);
  const [wave, setWave] = useState<Waveform>({ code: 0, bits: 24, protocol: 1, pulse_us: 0, repeats: 1 });
  const [map, setMap] = useState<{ id: number; action: string; target?: string; behavior?: string }>({ id: 0, action: 'none' });
  const catalog = rf?.catalog ?? {};
  const command = (operation: string, params: Record<string, unknown> = {}) => sendCmd('pealayer.rf', { operation, params });
  useEffect(() => {
    if (!open) return;
    const timer = window.setInterval(() => { if (!rf?.pending) command('catalog'); }, 1200);
    return () => window.clearInterval(timer);
  }, [open, rf?.pending, sendCmd]);
  const assign = (entry: Entry) => {
    const binding = blank(); binding.name = entry.name || `RF ${entry.id}`;
    binding.match = { ...binding.match, rf_code: entry.code, rf_bits: entry.bits, rf_protocol: entry.protocol };
    const existing = catalog.bindings?.find(item => item.match.rf_code === entry.code && item.match.rf_bits === entry.bits && item.match.rf_protocol === entry.protocol && item.match.gesture === 'down');
    setDraft(existing ?? binding); setPreviousName(existing?.name ?? ''); setTab('assignments');
  };
  const updateAction = (index: number, patch: Partial<Action>) => setDraft(current => ({ ...current, actions: current.actions.map((action, i) => i === index ? { ...action, ...patch } : action) }));
  const match = (patch: Partial<Binding['match']>) => setDraft(current => ({ ...current, match: { ...current.match, ...patch } }));
  const mapping = catalog.board_mapping_options?.find(option => option.action === map.action);
  const targetOptions = [...new Set(catalog.applications?.map(app => app.surface))].map(surface => ({ value: surface, label: `${surface} (all)` })).concat((catalog.applications ?? []).map(app => ({ value: app.id, label: app.id })));
  const activeBoardAction = catalog.entries?.find(entry => entry.code === draft.match.rf_code && entry.bits === draft.match.rf_bits && entry.protocol === draft.match.rf_protocol && entry.action_kind !== 0);
  return <>
    <Card className="surface-card" title={<Space><ApiOutlined />RF controls</Space>} extra={<Button icon={<EditOutlined />} onClick={() => { setOpen(true); command('catalog', { read_board: true }); }}>Manage</Button>}>
      <Space wrap><Tag>{catalog.bindings?.length ?? 0} assignments</Tag>{catalog.learning?.active && <Tag color="processing">Learning buttons</Tag>}</Space>
    </Card>
    <Modal className="rf-manager-modal" open={open} onCancel={() => setOpen(false)} footer={null} width={900} title={<Space><ApiOutlined />RF controls</Space>}>
      <Space style={{ display: 'flex', justifyContent: 'flex-end', marginBottom: 12 }}>
        <Button loading={rf?.pending} icon={<ReloadOutlined />} onClick={() => command('catalog', { read_board: true })}>Refresh</Button>
      </Space>
      {rf?.error && <Alert type="error" showIcon message={rf.error} style={{ marginBottom: 12 }} />}
      {catalog.board_error && <Alert type="warning" showIcon message={catalog.board_error} style={{ marginBottom: 12 }} />}
      <Tabs activeKey={tab} onChange={setTab} items={[
        { key: 'assignments', label: 'Assignments', children: <>
          <Space wrap style={{ marginBottom: 12 }}>
            <Button icon={<PlusOutlined />} onClick={() => { setDraft(blank()); setPreviousName(''); }}>New assignment</Button>
            {catalog.activity?.find(event => event.rf_code > 0) && <Button onClick={() => { const event = catalog.activity!.find(event => event.rf_code > 0)!; match({ rf_code: event.rf_code, rf_bits: event.rf_bits, rf_protocol: event.rf_protocol }); }}>Use last received</Button>}
          </Space>
          <Table<Binding> size="small" pagination={false} rowKey="name" dataSource={catalog.bindings ?? []} columns={[
            { title: 'Enabled', width: 80, render: (_, binding) => <Checkbox checked={binding.enabled} onChange={e => command('binding.put', { binding: { ...binding, enabled: e.target.checked } })} /> },
            { title: 'Assignment', dataIndex: 'name' },
            { title: 'Button', render: (_, binding) => <Typography.Text code>{`0x${binding.match.rf_code.toString(16).toUpperCase()} · ${human(binding.match.gesture)}`}</Typography.Text> },
            { title: '', width: 80, render: (_, binding) => <Button size="small" icon={<EditOutlined />} onClick={() => { setDraft(structuredClone(binding)); setPreviousName(binding.name); }}>Edit</Button> },
          ]} />
          <Form layout="vertical" style={{ marginTop: 18 }}>
            <div className="rf-form-grid">
              <Form.Item label="Name"><Input value={draft.name} maxLength={64} onChange={e => setDraft({ ...draft, name: e.target.value })} /></Form.Item>
              <Form.Item label="Gesture"><Select value={draft.match.gesture} options={(catalog.gestures ?? []).map(value => ({ value, label: human(value) }))} onChange={gesture => match({ gesture })} /></Form.Item>
              <Form.Item label="RF code"><InputNumber min={1} max={4294967295} value={draft.match.rf_code} onChange={code => match({ rf_code: code ?? 0 })} style={{ width: '100%' }} /></Form.Item>
              <Form.Item label="Bits"><InputNumber min={1} max={32} value={draft.match.rf_bits} onChange={bits => match({ rf_bits: bits ?? 24 })} /></Form.Item>
              <Form.Item label="Protocol"><InputNumber min={1} max={12} value={draft.match.rf_protocol} onChange={protocol => match({ rf_protocol: protocol ?? 1 })} /></Form.Item>
              <Form.Item label="Cooldown (ms)"><InputNumber min={1} max={3600000} value={draft.cooldown_ms} onChange={cooldown_ms => setDraft({ ...draft, cooldown_ms: cooldown_ms ?? 250 })} /></Form.Item>
            </div>
            {draft.actions.map((action, index) => {
              const apps = catalog.applications?.filter(app => app.id === action.app_target || app.surface === action.app_target) ?? [];
              const commands = [...new Set(apps.flatMap(app => (app.values?.app_actions ?? '').split(',').filter(Boolean)))];
              return <Card key={index} size="small" title={`Action ${index + 1}`} extra={<Button size="small" icon={<DeleteOutlined />} aria-label="Remove action" onClick={() => setDraft({ ...draft, actions: draft.actions.filter((_, i) => i !== index) })} />} style={{ marginBottom: 10 }}>
                <Form.Item label="Action"><Select value={action.type} options={(catalog.action_types ?? []).map(value => ({ value, label: labels[value] ?? human(value) }))} onChange={type => updateAction(index, { type })} /></Form.Item>
                {action.type === 'app' && <div className="rf-form-grid">
                  <Form.Item label="Application"><Select showSearch value={action.app_target} options={targetOptions} onChange={app_target => updateAction(index, { app_target, app_kind: '' })} /></Form.Item>
                  <Form.Item label="Command"><Select showSearch optionFilterProp="label" value={action.app_kind} options={commands.map(value => ({ value, label: human(value) }))} onChange={app_kind => updateAction(index, { app_kind })} /></Form.Item>
                  <Form.Item label="Value"><Input value={action.app_value} onChange={e => updateAction(index, { app_value: e.target.value })} placeholder={action.app_kind === 'pealayer.command' ? '{"command":"play"}' : undefined} /></Form.Item>
                </div>}
                {action.type === 'control' && <Form.Item label="Board control"><Select showSearch optionFilterProp="label" value={action.action_id} options={(catalog.peripherals?.controls ?? []).flatMap(control => control.actions.map(a => ({ value: a.id, label: `${control.name} · ${a.name || human(a.verb || a.id)}` })))} onChange={action_id => updateAction(index, { action_id })} /></Form.Item>}
                {(action.type === 'effect' || action.type === 'macro') && <Form.Item label="Effect"><Select showSearch value={action.macro} options={(catalog.effects ?? []).map(effect => ({ value: effect.reference, label: effect.name }))} onChange={macro => updateAction(index, { macro })} /></Form.Item>}
                {action.type === 'board' && <Form.Item label="Controller command"><Input value={action.command} onChange={e => updateAction(index, { command: e.target.value })} /></Form.Item>}
                {action.type === 'virtual-key' && <div className="rf-form-grid"><Form.Item label={`Keyboard on ${catalog.hostname || 'controller host'}`}><Input value={action.virtual_key} placeholder="SPACE, MEDIA_PLAY_PAUSE, F5…" onChange={e => updateAction(index, { virtual_key: e.target.value })} /></Form.Item><Form.Item label="Hold (ms)"><InputNumber min={10} max={1000} value={action.hold_ms ?? 50} onChange={hold_ms => updateAction(index, { hold_ms: hold_ms ?? 50 })} /></Form.Item></div>}
                {(action.type === 'host' || action.type === 'script') && <><Form.Item label="Program"><Input value={action.type === 'host' ? action.executable : action.script} onChange={e => updateAction(index, action.type === 'host' ? { executable: e.target.value } : { script: e.target.value })} /></Form.Item><Form.Item label="Arguments (one per line)"><Input.TextArea rows={2} value={action.args?.join('\n')} onChange={e => updateAction(index, { args: e.target.value.split('\n').filter(Boolean) })} /></Form.Item><Checkbox checked={action.detached} onChange={e => updateAction(index, { detached: e.target.checked })}>Launch independently</Checkbox></>}
                {action.type === 'emit' && <Form.Item label="Event"><Input value={action.event} onChange={e => updateAction(index, { event: e.target.value })} /></Form.Item>}
                {action.type === 'rf' && <WaveformFields value={action.rf ?? { code: 0, bits: 24, protocol: 1, pulse_us: 0, repeats: 1 }} onChange={rf => updateAction(index, { rf })} />}
              </Card>;
            })}
            {draft.actions.length < 8 && <Button icon={<PlusOutlined />} onClick={() => setDraft({ ...draft, actions: [...draft.actions, { type: 'app', app_target: 'pealayer', app_kind: 'pealayer.toggle' }] })}>Add action</Button>}
            {draft.actions.some(action => action.type === 'virtual-key') && <Form.Item style={{ marginTop: 12 }}><Checkbox checked={allowKeyboard} onChange={e => setAllowKeyboard(e.target.checked)}>Allow assigned keyboard keys on {catalog.hostname || 'controller host'}</Checkbox></Form.Item>}
            {activeBoardAction && <Alert type="warning" showIcon message="This button also has a board action" description={<Button size="small" onClick={() => command('map', { id: activeBoardAction.id, action: 'none' })}>Unassign board action</Button>} style={{ marginTop: 12 }} />}
            <Space style={{ marginTop: 16 }}><Button type="primary" icon={<SaveOutlined />} loading={rf?.pending} disabled={!draft.name.trim() || !draft.match.rf_code || !draft.actions.length} onClick={() => command('binding.put', { binding: draft, previous_name: previousName, allow_keyboard: allowKeyboard })}>Save assignment</Button>{previousName && <Button danger icon={<DeleteOutlined />} disabled={rf?.pending} onClick={() => command('binding.remove', { name: previousName })}>Remove</Button>}</Space>
          </Form>
        </> },
        { key: 'remotes', label: 'Remotes', children: <>
          <Space style={{ marginBottom: 12 }}>{catalog.learning?.active ? <><Tag color="processing">Learning · {Math.ceil(catalog.learning.remaining_ms / 1000)} s</Tag><Button onClick={() => command('learn.cancel')}>Stop learning</Button></> : <Button icon={<PlusOutlined />} disabled={!catalog.connected || rf?.pending} onClick={() => command('learn.start', { mode: 'timer', timeout_ms: 30000 })}>Learn buttons</Button>}</Space>
          <Table<Entry> size="small" pagination={false} rowKey="id" dataSource={catalog.entries ?? []} columns={[
            { title: 'Button', render: (_, entry) => <Space direction="vertical" size={0}><strong>{entry.name || `RF ${entry.id}`}</strong><Typography.Text code>{entry.code_display}</Typography.Text></Space> },
            { title: 'Signal', render: (_, entry) => `${entry.bits} bits · Protocol ${entry.protocol} · ${entry.pulse_us} µs` },
            { title: 'Board action', render: (_, entry) => entry.action_kind ? <Tag color="warning">Assigned</Tag> : <Tag>Unassigned</Tag> },
            { title: '', render: (_, entry) => <Space><Button size="small" onClick={() => assign(entry)}>Assign</Button><Button size="small" icon={<SendOutlined />} disabled={rf?.pending} onClick={() => command('transmit', { code: entry.code, bits: entry.bits, protocol: entry.protocol, pulse_us: entry.pulse_us, repeats: 1 })}>Transmit</Button><Dropdown menu={{ items: [{ key: 'board', label: 'Edit board action' }, { key: 'unassign', label: 'Unassign board action' }, { key: 'remove', label: 'Remove learned button', danger: true }], onClick: ({ key }) => { if (key === 'board') setMap({ id: entry.id, action: 'none' }); if (key === 'unassign') command('map', { id: entry.id, action: 'none' }); if (key === 'remove') Modal.confirm({ title: 'Remove learned RF button?', onOk: () => command('remove', { id: entry.id }) }); } }}><Button size="small" icon={<MoreOutlined />} aria-label="RF button actions" /></Dropdown></Space> },
          ]} />
          <Collapse style={{ marginTop: 16 }} items={[{ key: 'map', label: 'Board assignment', children: <Form layout="vertical"><div className="rf-form-grid"><Form.Item label="Button ID"><InputNumber min={0} max={19} value={map.id} onChange={id => setMap({ ...map, id: id ?? 0 })} /></Form.Item><Form.Item label="Action"><Select value={map.action} options={catalog.board_mapping_options?.map(option => ({ value: option.action, label: human(option.action) }))} onChange={action => setMap({ id: map.id, action })} /></Form.Item>{Boolean(mapping?.targets.length) && <Form.Item label="Target"><Select value={map.target} options={mapping?.targets.map(value => ({ value, label: human(value) }))} onChange={target => setMap({ ...map, target })} /></Form.Item>}{Boolean(mapping?.behaviors.length) && <Form.Item label="Behavior"><Select value={map.behavior} options={mapping?.behaviors.map(value => ({ value, label: human(value) }))} onChange={behavior => setMap({ ...map, behavior })} /></Form.Item>}</div><Button disabled={!catalog.connected || rf?.pending} icon={<SaveOutlined />} onClick={() => command('map', map)}>Save board assignment</Button></Form> }]} />
        </> },
        { key: 'transmit', label: 'Transmit', children: <Form layout="vertical"><WaveformFields value={wave} onChange={setWave} /><Button type="primary" icon={<SendOutlined />} disabled={!catalog.connected || !wave.code || rf?.pending} onClick={() => command('transmit', wave)}>Transmit</Button></Form> },
        { key: 'activity', label: 'Receive activity', children: catalog.activity?.length ? <Table size="small" pagination={false} rowKey="id" dataSource={catalog.activity} columns={[{ title: 'Received', dataIndex: 'text' }, { title: 'Time', render: (_, event) => new Date(event.time).toLocaleTimeString() }]} /> : <Empty description="No received RF activity" /> },
      ]} />
    </Modal>
  </>;
};

const WaveformFields: React.FC<{ value: Waveform; onChange: (wave: Waveform) => void }> = ({ value, onChange }) => <div className="rf-form-grid">{([['Code', 'code', 1, 4294967295], ['Bits', 'bits', 1, 32], ['Protocol', 'protocol', 1, 12], ['Pulse (µs, 0 = default)', 'pulse_us', 0, 65535], ['Repeats', 'repeats', 1, 20]] as const).map(([label, field, min, max]) => <Form.Item key={field} label={label}><InputNumber min={min} max={max} value={value[field]} onChange={n => onChange({ ...value, [field]: n ?? min })} style={{ width: '100%' }} /></Form.Item>)}</div>;
