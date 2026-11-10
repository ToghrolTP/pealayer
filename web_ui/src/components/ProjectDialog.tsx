import { useEffect, useRef, useState } from 'react';
import { Alert, Button, Checkbox, Collapse, Flex, Form, Input, InputNumber, Modal, Select, Space, Typography } from 'antd';
import { DownloadOutlined, FolderOpenOutlined, ImportOutlined, SaveOutlined } from '@ant-design/icons';
import { ServerFilePicker } from './ServerFilePicker';
import type { UiLocale } from '../i18n';
import './projects.css';

type Key = 'scene' | 'timeline' | 'media' | 'workspace';
type Selection = Record<Key, boolean>;
interface Document { kind: string; data: any }
const keys: Key[] = ['scene', 'timeline', 'media', 'workspace'];
const componentLabels: Record<Key, string> = { scene: 'Scene', timeline: 'Timeline', media: 'Media', workspace: 'Workspace' };
const all: Selection = { scene: true, timeline: true, media: true, workspace: true };

export function ProjectDialog({ base, locale, onClose }: { base: string; locale: UiLocale; onClose: () => void }) {
  const [document, setDocument] = useState<Document | null>(null);
  const [baseline, setBaseline] = useState('');
  const [path, setPath] = useState('');
  const [include, setInclude] = useState<Selection>(all);
  const [discard, setDiscard] = useState(false);
  const [confirmOutputs, setConfirmOutputs] = useState(false);
  const [selectedOutputs, setSelectedOutputs] = useState<string[]>([]);
  const [sceneReport, setSceneReport] = useState<any>(null);
  const [documentBase, setDocumentBase] = useState(base);
  const [documentOwner, setDocumentOwner] = useState('');
  const currentBase = useRef(base); currentBase.current = base;
  const applying = sceneReport?.status === 'running';
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState('');
  const [notice, setNotice] = useState('');
  const [picker, setPicker] = useState<{ title: string; extensions: string[]; select: (path: string) => void } | null>(null);
  const [closeConfirm, setCloseConfirm] = useState(false);
  const importInput = useRef<HTMLInputElement>(null);
  const selected = () => {
    const result = structuredClone(document);
    if (result?.kind === 'pealayer-project') keys.forEach(key => { if (!include[key]) result.data[key] = null; });
    return result;
  };
  async function request(value: object) {
    const operation = (value as { operation?: string }).operation;
    const writesAuthority = ['write', 'load', 'stage', 'apply_scene', 'cancel_scene'].includes(operation ?? '');
    if (writesAuthority && (documentBase !== base || !documentOwner)) {
      setError('Authority changed. Read or capture from the selected authority before changing its session or files.'); return null;
    }
    if (['read', 'snapshot', 'capture_scene', 'validate'].includes(operation ?? '') && document && JSON.stringify(selected()) !== baseline && !discard) {
      setError('Save or download the current document, or explicitly enable replacement before reading/capturing another.');
      return null;
    }
    setBusy(true); setError(''); setNotice('');
    try {
      const query = writesAuthority ? `?${new URLSearchParams({ owner_id: documentOwner })}` : '';
      const response = await fetch(`${base}/api/peer/documents${query}`, { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify(value) });
      const result = await response.json();
      if (currentBase.current !== base) return null;
      if (!response.ok || result.error) throw new Error(result.error || `Document request failed (${response.status})`);
      if (result.document) { setDocumentBase(base); setDocumentOwner(typeof result.owner_id === 'string' ? result.owner_id : ''); setConfirmOutputs(false); setSelectedOutputs([]); setDocument(result.document); setBaseline(JSON.stringify(result.document)); setInclude(all); }
      if ('scene_operation' in result) { setSceneReport(result.scene_operation); setConfirmOutputs(false); }
      if (typeof result.path === 'string') setPath(result.path);
      if (result.saved) { setBaseline(JSON.stringify(selected())); setNotice('Saved on the Pealayer authority.'); }
      if (result.loaded) setNotice('Loaded paused. Scene values are staged, not applied.');
      if (result.media_pending) setNotice('Loading media; timeline and workspace commit after decoder confirmation.');
      if (result.queued) setNotice('Commands queued; check Hardware Monitor for acknowledgements.');
      return result;
    } catch (reason) { setError(reason instanceof Error ? reason.message : String(reason)); return null; }
    finally { setBusy(false); }
  }
  useEffect(() => { setConfirmOutputs(false); setSceneReport(null); void request({ operation: 'snapshot' }); }, [base]);
  useEffect(() => {
    if (!applying || busy) return;
    const timer = window.setTimeout(() => void request({ operation: 'scene_status' }), 500);
    return () => window.clearTimeout(timer);
  }, [sceneReport, busy, base]);
  const edit = (apply: (document: Document) => void) => { setConfirmOutputs(false); setDocument(current => { if (!current) return current; const next = structuredClone(current); apply(next); return next; }); };
  const browse = (title: string, select: (path: string) => void, extensions = ['json']) => setPicker({ title, select, extensions });
  const close = () => {
    if (busy || applying) { setError('Wait for the current document operation to finish, or cancel after the current output.'); return; }
    if (JSON.stringify(selected()) !== baseline) setCloseConfirm(true); else onClose();
  };
  const project = document?.kind === 'pealayer-project' ? document.data : null;
  const part = (key: Key) => project?.[key]?.source === 'embedded' ? project[key].value : document?.kind === `pealayer-${key}` ? document.data : null;
  const scene = part('scene'), media = part('media');
  const editPart = (key: Key, apply: (value: any) => void) => edit(d => apply(d.kind === 'pealayer-project' ? d.data[key].value : d.data));
  const download = (value: Document, filename: string) => {
    const url = URL.createObjectURL(new Blob([JSON.stringify(value, null, 2) + '\n'], { type: 'application/json' }));
    const a = window.document.createElement('a'); a.href = url; a.download = filename; a.click(); URL.revokeObjectURL(url);
  };
  return <>
    <Modal open title={<Space><FolderOpenOutlined />Project metadata</Space>} width={820} className="project-dialog" onCancel={close}
      footer={<Flex gap={8} wrap justify="end">
        <Button onClick={close}>Close</Button>
        <Button disabled={!document || busy} icon={<DownloadOutlined />} onClick={() => download(selected()!, 'Project.pealayer.json')}>Download JSON</Button>
        <Button disabled={!document || busy || applying || documentBase !== base} onClick={() => void request({ operation: 'stage', document: selected(), base_path: path, discard_unsaved: discard })}>Load selected</Button>
        <Button type="primary" icon={<SaveOutlined />} loading={busy} disabled={!document || !path.trim() || applying || documentBase !== base} onClick={() => void request({ operation: 'write', path, document: selected() })}>Save to server</Button>
      </Flex>}>
      <Form layout="vertical" disabled={busy || applying}>
        <Form.Item label="Document path on the Pealayer authority"><Flex gap={8}>
          <Input value={path} onChange={e => setPath(e.target.value)} placeholder="Project.pealayer.json" />
          <Button icon={<FolderOpenOutlined />} onClick={() => browse('Open Pealayer document', value => { setPath(value); void request({ operation: 'read', path: value }); })}>Browse</Button>
        </Flex></Form.Item>
        <Flex gap={8} wrap className="project-dialog__actions">
          <Button disabled={!path} onClick={() => void request({ operation: 'read', path })}>Read file</Button>
          <Button onClick={() => void request({ operation: 'snapshot' })}>Capture session</Button>
          <Button onClick={() => void request({ operation: 'capture_scene' })}>Capture hardware scene</Button>
          <Button icon={<ImportOutlined />} onClick={() => importInput.current?.click()}>Import JSON</Button>
          <input ref={importInput} hidden type="file" accept=".json" disabled={busy} onChange={async e => {
            const file = e.target.files?.[0];
            e.target.value = '';
            if (!file) return;
            try { if (file.size > 32 * 1024 * 1024) throw new Error('Document exceeds 32 MiB'); const value = JSON.parse(await file.text());
              if (!['pealayer-project', 'pealayer-scene', 'pealayer-timeline', 'pealayer-media', 'pealayer-workspace'].includes(value.kind)) throw new Error('Not a Pealayer document');
              // Parse and validate the complete typed contract on the authority
              // before any untrusted nested object reaches the editor render.
              const validated = await request({ operation: 'validate', document: value });
              if (validated?.document) { setBaseline(''); setNotice('Imported for review.'); }
            } catch (reason) { setError(String(reason)); }
          }} />
        </Flex>
        {project && <>
          <Form.Item label="Title"><Input value={project.title} onChange={e => edit(d => { d.data.title = e.target.value; })} /></Form.Item>
          <Form.Item label="Notes"><Input.TextArea value={project.notes} rows={2} onChange={e => edit(d => { d.data.notes = e.target.value; })} /></Form.Item>
          <Typography.Title level={5}>Save / load components</Typography.Title>
          {keys.map(key => <div className="project-dialog__component" key={key}>
            <Flex gap={12} wrap align="center">
              <Checkbox checked={include[key]} onChange={e => setInclude({ ...include, [key]: e.target.checked })}>{componentLabels[key]}</Checkbox>
              <Checkbox checked={project.load[key]} disabled={!include[key]} onChange={e => edit(d => { d.data.load[key] = e.target.checked; })}>Load</Checkbox>
              <Select aria-label={`${key} source`} value={project[key]?.source ?? 'absent'} options={[{ value: 'embedded', label: 'Embedded' }, { value: 'file', label: 'Separate file' }, { value: 'absent', label: 'Not captured', disabled: true }]}
                onChange={source => { if (source === 'file') edit(d => { d.data[key] = { source: 'file', path: '' }; }); else setError('Capture the current session to embed this component.'); }} />
              {project[key]?.source === 'embedded' && <Button size="small" icon={<DownloadOutlined />} onClick={() => download({ kind: `pealayer-${key}`, data: project[key].value }, `${key}.json`)}>Export</Button>}
            </Flex>
            {project[key]?.source === 'file' && <Flex gap={8}><Input value={project[key].path} placeholder="Relative to project file" onChange={e => edit(d => { d.data[key].path = e.target.value; })} />
              <Button icon={<FolderOpenOutlined />} onClick={() => browse(`Select ${key} file`, value => edit(d => { d.data[key].path = value; }))} aria-label={`Browse ${key}`} />
            </Flex>}
          </div>)}
        </>}
        {media && <Collapse defaultActiveKey={['media']} items={[{ key: 'media', label: 'Media metadata', children: <>
          <Form.Item label="Media path or URL"><Flex gap={8}><Input value={media.target} onChange={e => editPart('media', m => { m.target = e.target.value; })} />
            <Button icon={<FolderOpenOutlined />} onClick={() => browse('Select media', value => editPart('media', m => { m.target = value; }), ['mkv', 'mp4', 'webm', 'mp3', 'wav', 'flac'])}>Browse</Button></Flex></Form.Item>
          <div className="project-dialog__media-grid">{(['position_ms', 'duration_ms', 'speed'] as const).map(key => <Form.Item key={key} label={{ position_ms: 'Position (ms)', duration_ms: 'Duration (ms)', speed: 'Speed' }[key]}>
            <InputNumber min={key === 'speed' ? 0.01 : 0} max={key === 'speed' ? 100 : 3155760000000} precision={key === 'speed' ? 2 : 0} value={media[key]} onChange={value => editPart('media', m => { m[key] = value ?? 0; })} /></Form.Item>)}</div>
          <Typography.Text type="secondary">{Object.keys(media.information?.metadata ?? {}).length} tags · {media.information?.chapters?.length ?? 0} chapters</Typography.Text>
        </> }]} />}
        {scene && <Collapse items={[{ key: 'scene', label: `Scene · ${Object.keys(scene.channels).length} channels`, children: <>
          <Form.Item label="Scene name"><Input value={scene.name} onChange={e => editPart('scene', s => { s.name = e.target.value; })} /></Form.Item>
          {Object.entries(scene.channels).map(([key, value]: [string, any]) => <Flex key={key} align="center" justify="space-between" gap={12} className="project-dialog__scene-row">
            <Checkbox checked={selectedOutputs.includes(key)} disabled={value.type === 'boolean' ? !/^relay\.[5-8]$/.test(key) : value.type === 'pwm' ? !/^pwm\.(?:[0-9]|10)$/.test(key) : true}
              onChange={e => { setConfirmOutputs(false); setSelectedOutputs(current => e.target.checked ? [...current, key] : current.filter(item => item !== key)); }}>{key}</Checkbox>
            {value.type === 'boolean' ? <Checkbox checked={value.on} onChange={e => editPart('scene', s => { s.channels[key].on = e.target.checked; })}>On</Checkbox> : value.type === 'pwm' ?
              <InputNumber min={0} max={100} precision={3} suffix="%" value={value.percent_milli / 1000} onChange={v => editPart('scene', s => { s.channels[key].percent_milli = Math.round((v ?? 0) * 1000); })} /> :
              <Select value={value.direction} options={['stop', 'up', 'down'].map(value => ({ value, label: value }))} onChange={direction => editPart('scene', s => { s.channels[key].direction = direction; })} />}
          </Flex>)}
          <Typography.Paragraph type="secondary">Unsampled channels are omitted, not assumed Off. Motion values are staged only.</Typography.Paragraph>
          <Checkbox checked={confirmOutputs} onChange={e => setConfirmOutputs(e.target.checked)}>Confirm hardware outputs</Checkbox>
          <Button danger disabled={!confirmOutputs || !selectedOutputs.length || documentBase !== base} onClick={() => void request({ operation: 'apply_scene', scene, selected: selectedOutputs, confirm: true })}>Apply selected outputs</Button>
        </> }]} />}
        {document?.kind === 'pealayer-timeline' && <Typography.Paragraph>{document.data.timeline.instances.length} cues · {document.data.timeline.analog_tracks.length} curves. Load to edit in Timeline.</Typography.Paragraph>}
        <Checkbox checked={discard} onChange={e => setDiscard(e.target.checked)}>Allow replacing unsaved document / session edits</Checkbox>
        <Typography.Paragraph type="secondary">Loading restores media paused and stages scene values without actuating outputs. Separate file paths belong to the connected Pealayer authority.</Typography.Paragraph>
      </Form>
      {sceneReport && <div role="status">
        <Typography.Title level={5}>Scene: {sceneReport.status}</Typography.Title>
        {sceneReport.outputs.map((item: any) => <div key={item.output.target}>{item.output.target} · {item.status}</div>)}
        {applying && <Button disabled={busy} onClick={() => void request({ operation: 'cancel_scene', operation_id: sceneReport.id })}>Cancel after current output</Button>}
      </div>}
      {error && <Alert type="error" showIcon title={error} />}{notice && <Alert type="success" showIcon title={notice} />}
    </Modal>
    <Modal open={closeConfirm} title="Unsaved document changes" onCancel={() => setCloseConfirm(false)} onOk={onClose} okText="Discard and close" okButtonProps={{ danger: true }}><Typography.Paragraph>Save or download to keep your changes.</Typography.Paragraph></Modal>
    {picker && <ServerFilePicker apiBaseUrl={base} locale={locale} title={picker.title} extensions={picker.extensions} initialFile={path} onCancel={() => setPicker(null)} onSelect={value => { picker.select(value); setPicker(null); }} />}
  </>;
}
