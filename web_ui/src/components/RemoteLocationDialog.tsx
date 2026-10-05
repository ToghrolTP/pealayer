import { useEffect, useState } from 'react';
import { Alert, Button, Checkbox, Descriptions, Dropdown, Input, Modal, Space, Table, Tag, Tooltip, Typography } from 'antd';
import { ArrowUpOutlined, CopyOutlined, FileOutlined, FolderOpenOutlined, GlobalOutlined, PlayCircleOutlined, ReloadOutlined, SearchOutlined } from '@ant-design/icons';

export interface RemoteEntry { name: string; url: string; is_dir: boolean; playable: boolean; size_bytes: number | null; modified: string | null }
export interface RemoteBrowser {
  previous_file?: string | null; next_file?: string | null;
  revision: number; visible: boolean; loading: boolean; target: string; use_proxy: boolean;
  auto_next: boolean; thumbnails: boolean; sort: 'name' | 'date' | 'size'; descending: boolean;
  selected: string | null; error: string | null;
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

export function RemoteLocationDialog({ state, connected, base, sendCmd }: {
  state?: RemoteBrowser; connected: boolean; base: string; sendCmd: (command: string, payload?: Record<string, unknown>) => void;
}) {
  const [target, setTarget] = useState(''); const [filter, setFilter] = useState(''); const [proxy, setProxy] = useState(true);
  useEffect(() => { if (state?.visible) { setTarget(state.target); setProxy(state.use_proxy); } }, [state?.target, state?.visible, state?.use_proxy]);
  if (!state?.visible) return null;
  const listing = state.listing;
  const selected = listing?.entries.find(entry => entry.url === state.selected);
  const browse = (url: string) => sendCmd('pealayer.remote.browse', { target: url, use_proxy: proxy });
  const choose = (entry: RemoteEntry, play = false) => entry.is_dir && play ? browse(entry.url) : sendCmd('pealayer.remote.select', { target: entry.url, play });
  const copy = (url: string) => { void navigator.clipboard?.writeText(url); };
  const valid = (() => { try { const url = new URL(target); return ['http:', 'https:'].includes(url.protocol) && !url.username && !url.password; } catch { return false; } })();
  const menu = (entry: RemoteEntry) => ({ items: [
    { key: 'play', label: entry.is_dir ? 'Open folder' : 'Play', icon: entry.is_dir ? <FolderOpenOutlined /> : <PlayCircleOutlined />, disabled: !connected || (!entry.is_dir && !entry.playable), onClick: () => choose(entry, true) },
    { key: 'select', label: 'File information', icon: <FileOutlined />, onClick: () => choose(entry) },
    { key: 'copy', label: 'Copy URL', icon: <CopyOutlined />, onClick: () => copy(entry.url) },
  ] });
  const sortOrder = (by: RemoteBrowser['sort']) => state.sort === by ? state.descending ? 'descend' as const : 'ascend' as const : null;
  return <Modal title={<Space><FolderOpenOutlined />Remote location</Space>} open centered onCancel={() => sendCmd('pealayer.remote.close')}
    width="min(1000px, calc(100vw - 32px))" className="remote-location-modal" styles={{ body: { maxHeight: 'calc(100dvh - 220px)', overflow: 'auto' } }}
    footer={<Space><Button onClick={() => sendCmd('pealayer.remote.close')}>Close</Button><Button type="primary" icon={<PlayCircleOutlined />} disabled={!connected || !selected?.playable} onClick={() => selected && choose(selected, true)}>Play selected</Button></Space>}>
    <div className="remote-location-address"><Input aria-label="Remote location" value={target} onChange={e => setTarget(e.target.value)} onPressEnter={() => valid && browse(target)} placeholder="https://host/folder/" prefix={<GlobalOutlined />} /><Button icon={<SearchOutlined />} disabled={!connected || !valid || state.loading} onClick={() => browse(target)}>Browse</Button></div>
    <Space wrap className="remote-location-options">
      <Checkbox checked={proxy} onChange={e => { setProxy(e.target.checked); if (valid && listing) sendCmd('pealayer.remote.browse', { target, use_proxy: e.target.checked }); }}>Use proxy</Checkbox>
      {listing && <><Tag icon={<GlobalOutlined />}>{listing.host}</Tag>{listing.server && <Typography.Text type="secondary">{listing.server}</Typography.Text>}
        <Button icon={<ArrowUpOutlined />} disabled={!connected || !listing.parent_url} onClick={() => listing.parent_url && browse(listing.parent_url)}>Parent folder</Button>
        <Button icon={<ReloadOutlined />} disabled={!connected || state.loading} onClick={() => browse(target)} aria-label="Reload directory" /></>}
    </Space>
    {state.error && <Alert type="error" showIcon message="Cannot open this remote location" description={state.error} />}
    {listing?.warning && <Alert type="warning" showIcon message={listing.warning} />}
    <Input allowClear prefix={<SearchOutlined />} value={filter} placeholder="Filter files..." onChange={e => setFilter(e.target.value)} className="remote-file-filter" />
    <Table<RemoteEntry> size="small" rowKey="url" pagination={listing && listing.entries.length > 100 ? { pageSize: 50, showSizeChanger: false } : false} loading={state.loading} scroll={{ y: 320, x: 560 }} dataSource={listing?.entries.filter(e => e.name.toLowerCase().includes(filter.toLowerCase())) || []}
      rowClassName={e => e.url === state.selected ? 'remote-file-selected' : ''}
      onChange={(_page, _filters, sorter) => { const s = Array.isArray(sorter) ? sorter[0] : sorter; sendCmd('pealayer.remote.sort', { by: s.columnKey || 'name', descending: s.order === 'descend' }); }}
      onRow={entry => ({ onClick: () => choose(entry), onDoubleClick: () => (entry.is_dir || entry.playable) && choose(entry, true) })}
      columns={[
        { key: 'name', title: 'Name', sorter: true, sortOrder: sortOrder('name'), render: (_, entry) => <Dropdown menu={menu(entry)} trigger={['contextMenu']}><div className="remote-file-name"><Thumbnail entry={entry} base={base} enabled={state.thumbnails} /><Typography.Text ellipsis={{ tooltip: entry.name }}>{entry.name}</Typography.Text></div></Dropdown> },
        { key: 'date', title: 'Modified', width: 180, sorter: true, sortOrder: sortOrder('date'), render: (_, e) => <Typography.Text type="secondary">{e.modified || '—'}</Typography.Text> },
        { key: 'size', title: 'Size', width: 105, align: 'right', sorter: true, sortOrder: sortOrder('size'), render: (_, e) => bytes(e.size_bytes) },
        { key: 'actions', width: 70, render: (_, entry) => <Button size="small" icon={entry.is_dir ? <FolderOpenOutlined /> : <PlayCircleOutlined />} disabled={!connected || (!entry.playable && !entry.is_dir)} aria-label={entry.is_dir ? 'Open folder' : 'Play'} onClick={event => { event.stopPropagation(); choose(entry, true); }} /> },
      ]} />
    {selected && <Descriptions bordered size="small" column={2} className="remote-file-info"><Descriptions.Item label="File" span={2}><Typography.Text ellipsis={{ tooltip: selected.name }}>{selected.name}</Typography.Text></Descriptions.Item><Descriptions.Item label="Size">{bytes(selected.size_bytes)}</Descriptions.Item><Descriptions.Item label="Modified">{selected.modified || 'Unavailable'}</Descriptions.Item><Descriptions.Item label="URL" span={2}><Typography.Text copyable ellipsis={{ tooltip: selected.url }}>{selected.url}</Typography.Text></Descriptions.Item></Descriptions>}
    <Space wrap className="remote-location-options"><Checkbox checked={state.auto_next} disabled={!connected} onChange={e => sendCmd('pealayer.config.update', { remote_folder_auto_next: e.target.checked })}>Automatically play next file</Checkbox><Checkbox checked={state.thumbnails} disabled={!connected} onChange={e => sendCmd('pealayer.config.update', { remote_folder_thumbnails: e.target.checked })}>Thumbnails</Checkbox><Typography.Text type="secondary">{listing?.entries.length || 0} items</Typography.Text></Space>
  </Modal>;
}
