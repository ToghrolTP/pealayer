import { useEffect, useRef, useState } from 'react';
import { clipboardUrl } from '../clipboardUrls';
import { Alert, Button, Checkbox, Descriptions, Dropdown, Input, Modal, Space, Table, Tag, Tooltip, Typography, message } from 'antd';
import { ArrowLeftOutlined, ArrowUpOutlined, CopyOutlined, FileOutlined, FolderOpenOutlined, GlobalOutlined, PlayCircleOutlined, ReloadOutlined, SearchOutlined, SnippetsOutlined } from '@ant-design/icons';

export interface RemoteEntry { name: string; url: string; is_dir: boolean; playable: boolean; size_bytes: number | null; modified: string | null }
export interface RemoteBrowser {
  previous_file?: string | null; next_file?: string | null;
  request_id: number; revision: number; visible: boolean; loading: boolean; target: string; use_proxy: boolean;
  auto_next: boolean; thumbnails: boolean; sort: 'name' | 'date' | 'size'; descending: boolean;
  selected: string | null; error: string | null;
  route_errors: { use_proxy: boolean; message: string }[];
  listing: { url: string; host: string; server: string | null; content_type: string | null; parent_url: string | null; warning: string | null; file: RemoteEntry | null; entries: RemoteEntry[] } | null;
}
const bytes = (value: number | null) => {
  if (value === null) return '—';
  const index = value === 0 ? 0 : Math.min(4, Math.floor(Math.log(value) / Math.log(1024)));
  return `${(value / 1024 ** index).toLocaleString(undefined, { maximumFractionDigits: index ? 1 : 0 })} ${['B','KiB','MiB','GiB','TiB'][index]}`;
};
function Thumbnail({ entry, base, enabled }: { entry: RemoteEntry; base: string; enabled: boolean }) {
  const [source, setSource] = useState<string>();
  const [error, setError] = useState<string>();
  useEffect(() => {
    setSource(undefined); setError(undefined);
    if (!enabled || !entry.playable) return;
    const abort = new AbortController(); let timer: ReturnType<typeof setTimeout>; let objectUrl: string | undefined; let attempts = 0;
    const request = async () => {
      try {
        const response = await fetch(`${base}/api/remote/thumbnail?url=${encodeURIComponent(entry.url)}`, { signal: abort.signal, cache: 'no-store' });
        if (response.status === 202) {
          if (++attempts < 60) timer = setTimeout(request, 1500); else setError('Thumbnail is still pending. Reopen the folder to retry.');
        } else if (response.ok) { objectUrl = URL.createObjectURL(await response.blob()); setSource(objectUrl); }
        else { const result = await response.json(); setError(result.error || 'Thumbnail unavailable'); }
      } catch (e) { if (!abort.signal.aborted) setError(String(e)); }
    };
    void request(); return () => { abort.abort(); clearTimeout(timer); if (objectUrl) URL.revokeObjectURL(objectUrl); };
  }, [entry.url, entry.playable, base, enabled]);
  return <Tooltip title={error}><span className="remote-file-thumbnail">{source ? <img src={source} alt="" /> : entry.is_dir ? <FolderOpenOutlined /> : <FileOutlined />}</span></Tooltip>;
}

export function RemoteLocationDialog({ state, connected, base, sendCmd, autoInspect }: {
  state?: RemoteBrowser; connected: boolean; base: string; sendCmd: (command: string, payload?: Record<string, unknown>) => void;
  autoInspect: boolean;
}) {
  const [target, setTarget] = useState(''); const [filter, setFilter] = useState(''); const [proxy, setProxy] = useState(true);
  const override = useRef<boolean | undefined>(undefined);
  const dirty = useRef(false); const lastSent = useRef('');
  useEffect(() => { if (!state?.visible) { override.current = undefined; dirty.current = false; } }, [state?.visible]);
  useEffect(() => {
    if (state?.visible && !(dirty.current && state.target === lastSent.current)) { setTarget(state.target); dirty.current = false; }
  }, [state?.request_id, state?.visible]);
  useEffect(() => { if (state?.visible) setProxy(state.use_proxy); }, [state?.use_proxy, state?.visible]);
  useEffect(() => {
    if (!state?.visible || !connected || !autoInspect || !dirty.current || !clipboardUrl(target)) return;
    const timer = setTimeout(() => {
      lastSent.current = target; dirty.current = false;
      sendCmd('pealayer.remote.browse', { target, ...(override.current === undefined ? {} : { use_proxy: override.current }) });
    }, 500);
    return () => clearTimeout(timer);
  }, [target, state?.visible, connected, autoInspect, sendCmd]);
  if (!state?.visible) return null;
  const listing = state.listing;
  const selected = listing?.entries.find(entry => entry.url === state.selected);
  const browse = (url: string) => { lastSent.current = url; dirty.current = false; sendCmd('pealayer.remote.browse', { target: url, ...(override.current === undefined ? {} : { use_proxy: override.current }) }); };
  const choose = (entry: RemoteEntry, play = false) => entry.is_dir && play ? browse(entry.url) : sendCmd('pealayer.remote.select', { target: entry.url, play });
  const copy = (url: string) => { void navigator.clipboard?.writeText(url); };
  const valid = !!clipboardUrl(target);
  const parent: RemoteEntry | undefined = listing?.parent_url ? { name: 'Back · Parent folder', url: listing.parent_url, is_dir: true, playable: false, size_bytes: null, modified: null } : undefined;
  const rows = [...(parent ? [parent] : []), ...(listing?.entries.filter(e => e.name.toLowerCase().includes(filter.toLowerCase())) || [])];
  const menu = (entry: RemoteEntry) => ({ items: [
    { key: 'play', label: entry.is_dir ? 'Open folder' : 'Play', icon: entry.is_dir ? <FolderOpenOutlined /> : <PlayCircleOutlined />, disabled: !connected || state.loading || (!entry.is_dir && !entry.playable), onClick: () => choose(entry, true) },
    { key: 'select', label: 'File information', icon: <FileOutlined />, disabled: entry.url === parent?.url || state.loading, onClick: () => choose(entry) },
    { key: 'copy', label: 'Copy URL', icon: <CopyOutlined />, onClick: () => copy(entry.url) },
  ] });
  const sortOrder = (by: RemoteBrowser['sort']) => state.sort === by ? state.descending ? 'descend' as const : 'ascend' as const : null;
  return <Modal title={<Space><FolderOpenOutlined />Remote location</Space>} open centered onCancel={() => sendCmd('pealayer.remote.close')}
    width="min(1000px, calc(100vw - 32px))" className="remote-location-modal" styles={{ body: { maxHeight: 'calc(100dvh - 220px)', overflow: 'auto' } }}
    footer={<Space><Button onClick={() => sendCmd('pealayer.remote.close')}>Close</Button><Button type="primary" icon={<PlayCircleOutlined />} disabled={!connected || !selected?.playable} onClick={() => selected && choose(selected, true)}>Play selected</Button></Space>}>
    <div className="remote-location-address"><Input aria-label="Remote location" value={target} onChange={e => { dirty.current = true; setTarget(e.target.value); }} onPressEnter={() => valid && browse(target)} placeholder="https://host/folder/" prefix={<GlobalOutlined />} /><Tooltip title="Paste a link; browser clipboard permission may be required"><Button icon={<SnippetsOutlined />} aria-label="Paste remote link" disabled={!navigator.clipboard?.readText} onClick={() => { void navigator.clipboard.readText().then(text => { dirty.current = true; setTarget(text); }).catch(() => { void message.info('Clipboard access was denied. Paste into the address field with Ctrl+V or your browser menu.'); }); }} /></Tooltip><Button icon={<SearchOutlined />} disabled={!connected || !valid} onClick={() => browse(target)}>Browse</Button></div>
    <Space wrap className="remote-location-options">
      <Tooltip title="A manual choice overrides automatic route selection until this dialog closes"><Checkbox checked={proxy} onChange={e => { override.current = e.target.checked; setProxy(e.target.checked); if (valid) browse(target); }}>Use proxy</Checkbox></Tooltip>
      {listing && <><Tag icon={<GlobalOutlined />}>{listing.host}</Tag>{listing.server && <Typography.Text type="secondary">{listing.server}</Typography.Text>}
        <Button icon={<ArrowUpOutlined />} disabled={!connected || !listing.parent_url} onClick={() => listing.parent_url && browse(listing.parent_url)}>Parent folder</Button>
        <Button icon={<ReloadOutlined />} disabled={!connected || state.loading} onClick={() => browse(target)} aria-label="Reload directory" /></>}
    </Space>
    {state.error && <Alert type="error" showIcon message="Cannot open this remote location" description={state.error} />}
    {state.route_errors.length > 0 && <div className="remote-route-errors">{state.route_errors.map(error => <Alert key={String(error.use_proxy)} type="error" showIcon message={error.use_proxy ? 'Proxy' : 'Direct'} description={error.message} />)}</div>}
    {listing?.warning && <Alert type="warning" showIcon message={listing.warning} />}
    <Input allowClear prefix={<SearchOutlined />} value={filter} placeholder="Filter files..." onChange={e => setFilter(e.target.value)} className="remote-file-filter" />
    <Table<RemoteEntry> size="small" rowKey="url" pagination={listing && listing.entries.length > 100 ? { pageSize: 50, showSizeChanger: false } : false} loading={{ spinning: state.loading, delay: 250 }} scroll={{ y: 320, x: 560 }} dataSource={rows}
      rowClassName={e => e.url === state.selected ? 'remote-file-selected' : ''}
      onChange={(_page, _filters, sorter) => { const s = Array.isArray(sorter) ? sorter[0] : sorter; sendCmd('pealayer.remote.sort', { by: s.columnKey || 'name', descending: s.order === 'descend' }); }}
      onRow={entry => ({ onClick: () => !state.loading && (entry.url === parent?.url ? browse(entry.url) : choose(entry)), onDoubleClick: () => !state.loading && (entry.is_dir || entry.playable) && choose(entry, true) })}
      columns={[
        { key: 'name', title: 'Name', sorter: true, sortOrder: sortOrder('name'), render: (_, entry) => <Dropdown menu={menu(entry)} trigger={['contextMenu']}><div className="remote-file-name">{entry.url === parent?.url ? <span className="remote-file-thumbnail"><ArrowLeftOutlined /></span> : <Thumbnail entry={entry} base={base} enabled={state.thumbnails} />}<Typography.Text ellipsis={{ tooltip: entry.name }}>{entry.name}</Typography.Text></div></Dropdown> },
        { key: 'date', title: 'Modified', width: 180, sorter: true, sortOrder: sortOrder('date'), render: (_, e) => <Typography.Text type="secondary">{e.modified || '—'}</Typography.Text> },
        { key: 'size', title: 'Size', width: 105, align: 'right', sorter: true, sortOrder: sortOrder('size'), render: (_, e) => bytes(e.size_bytes) },
        { key: 'actions', width: 70, render: (_, entry) => <Button size="small" icon={entry.is_dir ? <FolderOpenOutlined /> : <PlayCircleOutlined />} disabled={!connected || state.loading || (!entry.playable && !entry.is_dir)} aria-label={entry.is_dir ? 'Open folder' : 'Play'} onClick={event => { event.stopPropagation(); choose(entry, true); }} /> },
      ]} />
    {selected && <Descriptions bordered size="small" column={{ xs: 1, sm: 2 }} className="remote-file-info"><Descriptions.Item label="File" span="filled"><Typography.Text ellipsis={{ tooltip: selected.name }}>{selected.name}</Typography.Text></Descriptions.Item><Descriptions.Item label="Size">{bytes(selected.size_bytes)}</Descriptions.Item><Descriptions.Item label="Modified">{selected.modified || 'Unavailable'}</Descriptions.Item><Descriptions.Item label="URL" span="filled"><Typography.Text copyable ellipsis={{ tooltip: selected.url }}>{selected.url}</Typography.Text></Descriptions.Item></Descriptions>}
    <Space wrap className="remote-location-options"><Checkbox checked={state.auto_next} disabled={!connected} onChange={e => sendCmd('pealayer.config.update', { remote_folder_auto_next: e.target.checked })}>Automatically play next file</Checkbox><Checkbox checked={state.thumbnails} disabled={!connected} onChange={e => sendCmd('pealayer.config.update', { remote_folder_thumbnails: e.target.checked })}>Thumbnails</Checkbox><Typography.Text type="secondary">{listing?.entries.length || 0} items</Typography.Text></Space>
  </Modal>;
}
