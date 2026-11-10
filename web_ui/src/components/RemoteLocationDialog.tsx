import { useEffect, useRef, useState } from 'react';
import { clipboardUrl } from '../clipboardUrls';
import { useExtraMouse } from '../mouseBindings';
import type { MouseBindings } from '../mouseBindings';
import { Alert, Button, Checkbox, Descriptions, Dropdown, Input, Modal, Pagination, Segmented, Space, Table, Tag, Tooltip, Typography, message } from 'antd';
import { AppstoreOutlined, ArrowDownOutlined, ArrowLeftOutlined, ArrowRightOutlined, ArrowUpOutlined, CopyOutlined, FileOutlined, FolderOpenOutlined, GlobalOutlined, InfoCircleOutlined, LoadingOutlined, PlayCircleOutlined, ReloadOutlined, SearchOutlined, SnippetsOutlined, UnorderedListOutlined } from '@ant-design/icons';

export interface RemoteEntry { name: string; url: string; is_dir: boolean; playable: boolean; size_bytes: number | null; modified: string | null }
export interface RemoteBrowser {
  previous_file?: string | null; next_file?: string | null;
  history_back_url: string | null; history_forward_url: string | null;
  request_id: number; revision: number; visible: boolean; loading: boolean; target: string; use_proxy: boolean;
  auto_next: boolean; thumbnails: boolean; sort: 'name' | 'date' | 'size'; descending: boolean;
  view_mode: 'list' | 'grid'; inspected: RemoteEntry | null;
  selected: string | null; error: string | null;
  route_errors: { use_proxy: boolean; message: string }[];
  listing: { url: string; host: string; server: string | null; content_type: string | null; parent_url: string | null; warning: string | null; file: RemoteEntry | null; entries: RemoteEntry[] } | null;
}
const bytes = (value: number | null) => {
  if (value === null) return '—';
  const index = value === 0 ? 0 : Math.min(4, Math.floor(Math.log(value) / Math.log(1024)));
  return `${(value / 1024 ** index).toLocaleString(undefined, { maximumFractionDigits: index ? 1 : 0 })} ${['B','KiB','MiB','GiB','TiB'][index]}`;
};
function Thumbnail({ entry, base, enabled, retry }: { entry: RemoteEntry; base: string; enabled: boolean; retry: () => Promise<boolean> }) {
  const [source, setSource] = useState<string>();
  const [error, setError] = useState<string>();
  const [pending, setPending] = useState(enabled && entry.playable);
  const [attempt, setAttempt] = useState(0);
  useEffect(() => {
    setSource(undefined); setError(undefined); setPending(enabled && entry.playable);
    if (!enabled || !entry.playable) return;
    const abort = new AbortController(); let timer: ReturnType<typeof setTimeout>; let objectUrl: string | undefined; let attempts = 0;
    const request = async () => {
      try {
        const response = await fetch(`${base}/api/remote/thumbnail?url=${encodeURIComponent(entry.url)}`, { signal: abort.signal, cache: 'no-store' });
        if (abort.signal.aborted) return;
        if (response.status === 202) {
          if (++attempts < 60) timer = setTimeout(request, 1500);
          else { setError('Thumbnail is still pending. Try again.'); setPending(false); }
        } else if (response.ok) {
          const blob = await response.blob();
          if (abort.signal.aborted) return;
          objectUrl = URL.createObjectURL(blob); setSource(objectUrl); setPending(false);
        } else {
          const result = await response.json();
          if (!abort.signal.aborted) { setError(result.error || 'Thumbnail unavailable'); setPending(false); }
        }
      } catch (error) {
        if (!abort.signal.aborted) { setError(String(error)); setPending(false); }
      }
    };
    void request();
    return () => { abort.abort(); clearTimeout(timer); if (objectUrl) URL.revokeObjectURL(objectUrl); };
  }, [entry.url, entry.playable, base, enabled, attempt]);
  return <Tooltip title={error || (pending ? 'Fetching thumbnail…' : undefined)}>
    <span className="remote-file-thumbnail" aria-busy={pending}>
      {source ? <img src={source} alt="" onError={() => { setSource(undefined); setError('This preview could not be decoded.'); }} />
        : pending ? <span role="status" aria-label="Fetching thumbnail"><LoadingOutlined spin /></span>
        : entry.is_dir ? <FolderOpenOutlined /> : <FileOutlined />}
      {error && <Button type="text" size="small" icon={<ReloadOutlined />} aria-label="Retry thumbnail" onClick={event => {
        event.stopPropagation(); void retry().then(ok => { if (ok) setAttempt(value => value + 1); });
      }} />}
    </span>
  </Tooltip>;
}

export function RemoteLocationDialog({ state, connected, base, sendCmd, autoInspect, mouseBindings }: {
  state?: RemoteBrowser; connected: boolean; base: string; sendCmd: (command: string, payload?: Record<string, unknown>) => Promise<boolean>;
  autoInspect: boolean;
  mouseBindings?: MouseBindings;
}) {
  const extraMouse = useExtraMouse(mouseBindings?.remote_browser, action => {
    if (!connected || !state?.visible || state.loading) return;
    if (action === 'history_back' && (state.history_back_url || state.listing?.parent_url)) sendCmd('remote_history',{direction:-1});
    else if (action === 'history_forward' && state.history_forward_url) sendCmd('remote_history',{direction:1});
    else if (action === 'parent_folder' && state.listing?.parent_url) sendCmd('remote_history',{direction:-1,parent:true});
  });
  const [target, setTarget] = useState(''); const [filter, setFilter] = useState(''); const [proxy, setProxy] = useState(true);
  const [page, setPage] = useState(1);
  const [showLoading, setShowLoading] = useState(false);
  useEffect(() => {
    setShowLoading(false);
    if (!state?.loading) return;
    const timer = setTimeout(() => setShowLoading(true), 250);
    return () => clearTimeout(timer);
  }, [state?.loading, state?.request_id]);
  useEffect(() => setPage(1), [filter, state?.request_id, state?.view_mode]);
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
  const files = listing?.entries.length ? listing.entries : listing?.file ? [listing.file] : [];
  const selected = files.find(entry => entry.url === state.selected);
  const browse = (url: string) => { lastSent.current = url; dirty.current = false; sendCmd('pealayer.remote.browse', { target: url, ...(override.current === undefined ? {} : { use_proxy: override.current }) }); };
  const choose = (entry: RemoteEntry, play = false) => entry.is_dir ? browse(entry.url) : sendCmd('pealayer.remote.select', { target: entry.url, play });
  const copy = (text: string) => {
    if (!navigator.clipboard) { void message.info('Use Properties to select and copy the location. Clipboard access is unavailable in this browser context.'); return; }
    void navigator.clipboard.writeText(text).catch(() => { void message.warning('Clipboard access was denied. Select and copy the location from Properties.'); });
  };
  const valid = !!clipboardUrl(target);
  const parent: RemoteEntry | undefined = listing?.parent_url ? { name: 'Parent folder', url: listing.parent_url, is_dir: true, playable: false, size_bytes: null, modified: null } : undefined;
  const rows = [...(parent ? [parent] : []), ...files.filter(entry => entry.name.toLowerCase().includes(filter.toLowerCase()))];
  const retry = (entry: RemoteEntry) => !connected || state.loading ? Promise.resolve(false) : sendCmd('pealayer.remote.thumbnail.retry', {target:entry.url});
  const menu = (entry: RemoteEntry) => ({ items: [
    { key: 'play', label: entry.is_dir ? 'Open folder' : 'Play', icon: entry.is_dir ? <FolderOpenOutlined /> : <PlayCircleOutlined />, disabled: !connected || state.loading || (!entry.is_dir && !entry.playable), onClick: () => choose(entry, true) },
    ...(entry.url !== parent?.url ? [{ key: 'properties', label: 'Properties', icon: <InfoCircleOutlined />, disabled: !connected, onClick: () => { void sendCmd('pealayer.remote.inspect',{target:entry.url}); } }] : []),
    { key: 'copy', label: 'Copy link', icon: <CopyOutlined />, onClick: () => copy(entry.url) },
    ...(entry.url !== parent?.url ? [{ key: 'copy-name', label: 'Copy name', icon: <CopyOutlined />, onClick: () => copy(entry.name) }] : []),
  ] });
  const sortOrder = (by: RemoteBrowser['sort']) => state.sort === by ? state.descending ? 'descend' as const : 'ascend' as const : null;
  return <><Modal modalRender={node => <div {...extraMouse}>{node}</div>} title={<Space><FolderOpenOutlined />Remote location</Space>} open centered onCancel={() => sendCmd('pealayer.remote.close')}
    width="min(1000px, calc(100vw - 32px))" className="remote-location-modal" styles={{ body: { maxHeight: 'calc(100dvh - 220px)', overflow: 'auto' } }}
    footer={<Space><Button onClick={() => sendCmd('pealayer.remote.close')}>Close</Button><Button type="primary" icon={<PlayCircleOutlined />} disabled={!connected || state.loading || !selected?.playable} onClick={() => selected && choose(selected, true)}>Play selected</Button></Space>}>
    <div className="remote-location-address"><Input aria-label="Remote location" value={target} onChange={e => { dirty.current = true; setTarget(e.target.value); }} onPressEnter={() => valid && browse(target)} placeholder="https://host/folder/" prefix={<GlobalOutlined />} /><Tooltip title="Paste a link; browser clipboard permission may be required"><Button icon={<SnippetsOutlined />} aria-label="Paste remote link" disabled={!navigator.clipboard?.readText} onClick={() => { void navigator.clipboard.readText().then(text => { dirty.current = true; setTarget(text); }).catch(() => { void message.info('Clipboard access was denied. Paste into the address field with Ctrl+V or your browser menu.'); }); }} /></Tooltip><Button icon={<SearchOutlined />} disabled={!connected || !valid} onClick={() => browse(target)}>Browse</Button></div>
    <Space wrap className="remote-location-options">
      <Button icon={<ArrowLeftOutlined />} aria-label="Back" disabled={!connected || state.loading || !(state.history_back_url || listing?.parent_url)} onClick={() => sendCmd('remote_history',{direction:-1})} />
      <Button icon={<ArrowRightOutlined />} aria-label="Forward" disabled={!connected || state.loading || !state.history_forward_url} onClick={() => sendCmd('remote_history',{direction:1})} />
      <Tooltip title="A manual choice overrides automatic route selection until this dialog closes"><Checkbox checked={proxy} onChange={e => { override.current = e.target.checked; setProxy(e.target.checked); if (valid) browse(target); }}>Use proxy</Checkbox></Tooltip>
      {listing && <><Tag icon={<GlobalOutlined />}>{listing.host}</Tag>{listing.server && <Typography.Text type="secondary">{listing.server}</Typography.Text>}
        {listing.parent_url && <Button icon={<ArrowUpOutlined />} disabled={!connected || state.loading} onClick={() => browse(listing.parent_url!)}>Parent folder</Button>}
        <Button icon={<ReloadOutlined />} disabled={!connected || state.loading} onClick={() => browse(target)} aria-label="Reload directory" /></>}
    </Space>
    {state.error && <Alert type="error" showIcon message="Cannot open this remote location" description={state.error} />}
    {state.route_errors.length > 0 && <div className="remote-route-errors">{state.route_errors.map(error => <Alert key={String(error.use_proxy)} type="error" showIcon message={error.use_proxy ? 'Proxy' : 'Direct'} description={error.message} />)}</div>}
    {listing?.warning && <Alert type="warning" showIcon message={listing.warning} />}
    <div className="remote-browser-tools"><Input aria-label="Filter remote files" allowClear prefix={<SearchOutlined />} value={filter} placeholder="Filter files..." onChange={e => setFilter(e.target.value)} className="remote-file-filter" />
      <Segmented disabled={!connected} aria-label="Remote folder layout" value={state.view_mode} options={[{value:'list',icon:<UnorderedListOutlined />,label:<span className="remote-view-label">List</span>},{value:'grid',icon:<AppstoreOutlined />,label:<span className="remote-view-label">Grid</span>}]}
        onChange={value => { void sendCmd('pealayer.config.update',{remote_folder_view:value}); }} />
    </div>
    {state.view_mode === 'grid' ? <>
      <Space className="remote-grid-sort"><Typography.Text type="secondary">Sort by</Typography.Text>
        {(['name','date','size'] as const).map(by => <Button disabled={!connected} key={by} type="text" className={state.sort === by ? 'remote-sort-active' : ''} onClick={() => sendCmd('pealayer.remote.sort',{by,descending:state.sort === by ? !state.descending:false})}>{by === 'date' ? 'Modified' : by === 'size' ? 'Size' : 'Name'}{state.sort === by && (state.descending ? <ArrowDownOutlined />:<ArrowUpOutlined />)}</Button>)}
      </Space>
      <div className="remote-file-grid" aria-busy={state.loading}>
        {rows.slice((page-1)*50,page*50).map(entry => <Dropdown key={entry.url} menu={menu(entry)} trigger={['contextMenu']}>
          <div role="button" tabIndex={connected && !state.loading ? 0 : -1} aria-disabled={!connected || state.loading} aria-pressed={entry.url === state.selected} className={`remote-grid-card ${entry.url === state.selected ? 'remote-grid-card--selected' : ''}`}
            onClick={event => { if (connected && !state.loading && (!entry.is_dir || event.detail < 2)) choose(entry); }} onDoubleClick={() => { if (connected && !state.loading && !entry.is_dir && entry.playable) choose(entry,true); }}
            onKeyDown={event => { if ((event.key === 'Enter' || event.key === ' ') && connected && !state.loading) { event.preventDefault(); choose(entry); } }}>
            {entry.url === parent?.url ? <span className="remote-file-thumbnail"><ArrowUpOutlined /></span> : <Thumbnail entry={entry} base={base} enabled={state.thumbnails} retry={() => retry(entry)} />}
            <div className="remote-grid-caption"><Typography.Text ellipsis={{tooltip:entry.name}}>{entry.name}</Typography.Text><Typography.Text type="secondary">{entry.url === parent?.url ? 'Go up one level' : entry.is_dir ? 'Folder' : entry.size_bytes === null ? 'Media file' : bytes(entry.size_bytes)}</Typography.Text></div>
            <Button type="text" size="small" icon={entry.is_dir ? <FolderOpenOutlined />:<PlayCircleOutlined />} aria-label={entry.is_dir ? 'Open folder' : 'Play'} disabled={!connected || state.loading || (!entry.is_dir && !entry.playable)} onClick={event => { event.stopPropagation(); choose(entry,true); }} />
          </div>
        </Dropdown>)}
      </div>
      {!rows.length && <Typography.Paragraph type="secondary">{files.length ? 'No matching files.' : 'This folder is empty.'}</Typography.Paragraph>}
      {rows.length > 50 && <Pagination size="small" current={page} pageSize={50} total={rows.length} showSizeChanger={false} onChange={setPage} />}
      <div className="remote-grid-loading" role="status" style={{visibility:showLoading?'visible':'hidden'}}><LoadingOutlined spin /> Reading remote location…</div>
    </> : <Table<RemoteEntry> size="small" rowKey="url" pagination={files.length > 100 ? { pageSize: 50, showSizeChanger: false } : false} loading={{ spinning: state.loading, delay: 250 }} scroll={{ y: 320, x: 560 }} dataSource={rows}
      rowClassName={e => e.url === state.selected ? 'remote-file-selected' : ''}
      onChange={(_page, _filters, sorter) => { const s = Array.isArray(sorter) ? sorter[0] : sorter; sendCmd('pealayer.remote.sort', { by: s.columnKey || 'name', descending: s.order === 'descend' }); }}
      onRow={entry => ({ onClick: event => connected && !state.loading && (!entry.is_dir || event.detail < 2) && choose(entry), onDoubleClick: () => connected && !state.loading && !entry.is_dir && entry.playable && choose(entry, true) })}
      columns={[
        { key: 'name', title: 'Name', sorter: true, sortOrder: sortOrder('name'), render: (_, entry) => <Dropdown menu={menu(entry)} trigger={['contextMenu']}><div className="remote-file-name">{entry.url === parent?.url ? <span className="remote-file-thumbnail"><ArrowUpOutlined /></span> : <Thumbnail entry={entry} base={base} enabled={state.thumbnails} retry={() => retry(entry)} />}<Typography.Text ellipsis={{ tooltip: entry.name }}>{entry.name}</Typography.Text></div></Dropdown> },
        { key: 'date', title: 'Modified', width: 180, sorter: true, sortOrder: sortOrder('date'), render: (_, e) => e.url === parent?.url ? null : <Typography.Text type="secondary">{e.modified || '—'}</Typography.Text> },
        { key: 'size', title: 'Size', width: 105, align: 'right', sorter: true, sortOrder: sortOrder('size'), render: (_, e) => e.url === parent?.url ? null : bytes(e.size_bytes) },
        { key: 'actions', width: 70, render: (_, entry) => <Button size="small" icon={entry.is_dir ? <FolderOpenOutlined /> : <PlayCircleOutlined />} disabled={!connected || state.loading || (!entry.playable && !entry.is_dir)} aria-label={entry.is_dir ? 'Open folder' : 'Play'} onClick={event => { event.stopPropagation(); choose(entry, true); }} /> },
      ]} />}
    <Space wrap className="remote-location-options"><Checkbox checked={state.auto_next} disabled={!connected} onChange={e => sendCmd('pealayer.config.update', { remote_folder_auto_next: e.target.checked })}>Automatically play next file</Checkbox><Checkbox checked={state.thumbnails} disabled={!connected} onChange={e => sendCmd('pealayer.config.update', { remote_folder_thumbnails: e.target.checked })}>Thumbnails</Checkbox><Typography.Text type="secondary">{listing?.entries.length || 0} items</Typography.Text></Space>
  </Modal>
    <Modal title={<Space><InfoCircleOutlined />Properties</Space>} open={!!state.inspected} onCancel={() => sendCmd('pealayer.remote.inspect',{target:null})} width="min(600px, calc(100vw - 32px))" className="remote-properties-modal" styles={{body:{maxHeight:'calc(100dvh - 240px)',overflow:'auto'}}}
      footer={<Space><Button icon={<CopyOutlined />} onClick={() => state.inspected && copy(state.inspected.url)}>Copy link</Button><Button onClick={() => sendCmd('pealayer.remote.inspect',{target:null})}>Close</Button></Space>}>
      {state.inspected && <Descriptions bordered size="small" column={1} items={[
        {key:'name',label:'Name',children:state.inspected.name},
        {key:'kind',label:'Type',children:state.inspected.is_dir ? 'Folder' : state.inspected.playable ? 'Playable media' : 'File'},
        {key:'host',label:'Host',children:listing?.host || 'Remote server'},
        {key:'size',label:'Size',children:state.inspected.is_dir ? 'Not applicable' : state.inspected.size_bytes === null ? 'Not provided by server' : bytes(state.inspected.size_bytes)},
        {key:'date',label:'Modified',children:state.inspected.modified || 'Not provided by server'},
        {key:'location',label:'Location',children:<Typography.Text copyable>{state.inspected.url}</Typography.Text>},
      ]} />}
    </Modal>
  </>;
}
