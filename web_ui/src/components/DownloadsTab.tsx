import { useEffect, useState } from 'react';
import { Alert, Button, Card, Checkbox, Collapse, Empty, Input, InputNumber, Progress, Segmented, Select, Space, Tag, Typography } from 'antd';
import { ArrowDownOutlined, ArrowUpOutlined, CheckCircleOutlined, ClockCircleOutlined, DashboardOutlined, DeleteOutlined, DownloadOutlined, PauseOutlined, PlayCircleOutlined, PlusOutlined, SearchOutlined, SettingOutlined, StopOutlined } from '@ant-design/icons';
import './downloads.css';

type State = 'queued' | 'connecting' | 'downloading' | 'paused' | 'complete' | 'failed' | 'cancelled';
type Engine = 'native' | 'aria2' | 'yt_dlp' | 'ffmpeg';
const engineNames: Record<Engine, string> = { native: 'Built-in', aria2: 'aria2', yt_dlp: 'yt-dlp', ffmpeg: 'FFmpeg' };
const stateNames: Record<State, string> = { queued: 'Queued', connecting: 'Connecting', downloading: 'Downloading', paused: 'Paused', complete: 'Complete', failed: 'Failed', cancelled: 'Cancelled' };
interface Job { id: string; filename: string; source: string; state: State; downloaded: number; total: number | null; speed: number; eta_seconds: number | null; error: string | null; output: string | null; samples: number[]; actions: string[]; engine: Engine; connections: number }
interface Snapshot { jobs: Job[]; active: number; total_speed: number; max_concurrent: number; bytes_per_second: number; revision: number; engines: Engine[]; aria2_endpoint: string | null }
const bytes = (n: number) => { const index = Math.min(3, Math.floor(Math.log2(Math.max(1, n)) / 10)); return `${(n / 1024 ** index).toFixed(1)} ${['B', 'KiB', 'MiB', 'GiB'][index]}`; };
const tones: Record<State, string> = { queued: 'default', connecting: 'processing', downloading: 'processing', paused: 'warning', complete: 'success', failed: 'error', cancelled: 'default' };

export function DownloadsTab({ apiBaseUrl, sendCmd }: { apiBaseUrl: string; sendCmd: (method: string, params?: Record<string, unknown>) => Promise<boolean> }) {
  const [snapshot, setSnapshot] = useState<Snapshot | null>(null);
  const [url, setUrl] = useState('');
  const [proxy, setProxy] = useState(false);
  const [pending, setPending] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [queueError, setQueueError] = useState<string | null>(null);
  const [filter, setFilter] = useState<'All' | 'Pending' | 'Finished'>('All');
  const [search, setSearch] = useState('');
  const [engine, setEngine] = useState<Engine>('native');
  const [connections, setConnections] = useState(4);
  const [endpoint, setEndpoint] = useState('http://127.0.0.1:6800/jsonrpc');
  const [secret, setSecret] = useState('');
  const refresh = async (signal?: AbortSignal) => {
    const response = await fetch(`${apiBaseUrl}/api/rpc`, { method: 'POST', signal, headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ jsonrpc: '2.0', id: 'download-center', method: 'pealayer.downloads.list', params: {} }) });
    const data = await response.json();
    if (!response.ok || data.error) throw new Error(data.error?.message || 'Cannot read download queue');
    if (!Array.isArray(data.result?.jobs)) throw new Error('Invalid download queue response');
    setSnapshot(data.result); setQueueError(null);
  };
  useEffect(() => {
    const controller = new AbortController();
    let timer: ReturnType<typeof setTimeout>;
    const poll = async () => {
      try { await refresh(controller.signal); } catch (cause) { if (!controller.signal.aborted) setQueueError(String(cause)); }
      if (!controller.signal.aborted) timer = setTimeout(poll, document.hidden ? 5000 : 1000);
    };
    void poll();
    return () => { controller.abort(); clearTimeout(timer); };
  }, [apiBaseUrl]);
  const command = async (method: string, params: Record<string, unknown>) => {
    if (pending) return false;
    setPending(true);
    try { const ok = await sendCmd(method, params); if (ok) { setError(null); await refresh(); } return ok; }
    catch (cause) { setError(String(cause)); return false; }
    finally { setPending(false); }
  };
  const query = search.trim().toLocaleLowerCase();
  const jobs = snapshot?.jobs.filter(job => {
    const finished = ['complete', 'failed', 'cancelled'].includes(job.state);
    return (filter === 'All' || (filter === 'Finished' ? finished : !finished))
      && (!query || job.filename.toLocaleLowerCase().includes(query) || job.source.toLocaleLowerCase().includes(query));
  });
  return <section className="download-center">
    <div className="download-header">
      <Typography.Title level={3}><DownloadOutlined /> Download Center</Typography.Title>
      {snapshot && <div className="download-summary">
        <span><DashboardOutlined /> {bytes(snapshot.total_speed)}/s</span>
        <span><ClockCircleOutlined /> {snapshot.active} active</span>
        <span><CheckCircleOutlined /> {snapshot.jobs.filter(job => job.state === 'complete').length} complete</span>
      </div>}
    </div>
    {(error || queueError) && <Alert type="error" showIcon message={error || queueError} />}
    <Card className="download-compose">
      <Typography.Text strong>New download</Typography.Text>
      <div className="download-add"><Input aria-label="Download URL" placeholder="Paste a file or media link" value={url} onChange={e => setUrl(e.target.value)} />
        <Button type="primary" icon={<PlusOutlined />} loading={pending} disabled={!url.trim()} onClick={() => void command('pealayer.downloads.add', { url: url.trim(), use_proxy: proxy, engine, connections: engine === 'aria2' ? connections : 1 }).then(ok => { if (ok) setUrl(''); })}>Add download</Button></div>
      <Space wrap>
        <Select aria-label="Download engine" value={engine} onChange={setEngine} options={snapshot?.engines.filter(value => value !== 'ffmpeg').map(value => ({ value, label: engineNames[value] }))} style={{ minWidth: 140 }} />
        {engine === 'aria2' && <label>Connections <InputNumber aria-label="Connections per file" min={1} max={16} value={connections} onChange={value => { if (value != null) setConnections(value); }} /></label>}
        <Checkbox checked={proxy} onChange={e => setProxy(e.target.checked)}>Use proxy</Checkbox>
        {snapshot && <><label>Parallel files <InputNumber aria-label="Parallel files" min={1} max={8} value={snapshot.max_concurrent} disabled={pending}
          onChange={value => { if (value != null) void command('pealayer.downloads.configure', { max_concurrent: value, bytes_per_second: snapshot.bytes_per_second }); }} /></label>
          <label>Limit <InputNumber aria-label="Bandwidth limit in KiB per second; zero is unlimited" min={0} max={1048576} value={snapshot.bytes_per_second / 1024} disabled={pending}
            onChange={value => { if (value != null) void command('pealayer.downloads.configure', { max_concurrent: snapshot.max_concurrent, bytes_per_second: value * 1024 }); }} /> KiB/s</label></>}
      </Space>
      <Collapse ghost items={[{ key: 'engines', label: <><SettingOutlined /> Engine settings</>, children: <div className="download-engine-settings">
        <label>Local aria2 RPC <Input aria-label="aria2 RPC endpoint" value={endpoint} onChange={e => setEndpoint(e.target.value)} /></label>
        <Input.Password aria-label="aria2 RPC secret" placeholder="RPC secret (optional)" value={secret} onChange={e => setSecret(e.target.value)} />
        <Button disabled={pending} onClick={() => void command('pealayer.downloads.engines.configure', { aria2_endpoint: endpoint.trim(), aria2_secret: secret || null }).then(ok => { if (ok) setSecret(''); })}>Connect aria2</Button>
      </div> }]} />
    </Card>
    <div className="download-queue-toolbar">
      <Typography.Text strong>Queue{snapshot && ` · ${snapshot.jobs.length}`}</Typography.Text>
      <Segmented<'All' | 'Pending' | 'Finished'> options={['All', 'Pending', 'Finished']} value={filter} onChange={setFilter} />
      <Input aria-label="Filter downloads" prefix={<SearchOutlined />} placeholder="Filter by name or source" value={search} onChange={e => setSearch(e.target.value)} allowClear />
    </div>
    {jobs?.length === 0 && <Card><Empty image={Empty.PRESENTED_IMAGE_SIMPLE}
      description={snapshot?.jobs.length ? 'No matching downloads' : 'No downloads yet. Paste a link above to get started.'} /></Card>}
    {jobs?.map(job => {
      const act = (action: string) => void command('pealayer.downloads.action', { id: job.id, action });
      const peak = Math.max(1, ...job.samples);
      const points = job.samples.map((speed, i) => `${i / Math.max(1, job.samples.length - 1) * 600},${36 - speed / peak * 34}`).join(' ');
      return <Card key={job.id} className={`download-job download-job--${job.state}`}>
        <div className="download-heading"><div className="download-name"><DownloadOutlined /><div><Typography.Text strong ellipsis={{ tooltip: job.filename }}>{job.filename}</Typography.Text><Typography.Text type="secondary">{job.source}</Typography.Text></div></div>
          <Space wrap><Tag>{engineNames[job.engine]}</Tag><Tag color={tones[job.state]}>{stateNames[job.state]}</Tag></Space></div>
        {job.total ? <Progress percent={Math.min(100, job.downloaded / job.total * 100)} format={value => `${value?.toFixed(1)}%`} status={job.state === 'failed' ? 'exception' : job.state === 'complete' ? 'success' : 'normal'} /> : null}
        <div className="download-stats"><span>{bytes(job.downloaded)}{job.total ? ` / ${bytes(job.total)}` : ''}</span><span>{bytes(job.speed)}/s</span>
          {job.eta_seconds != null && <span>{Math.floor(job.eta_seconds / 60)}m {job.eta_seconds % 60}s left</span>}</div>
        <svg className={`download-chart${job.samples.length > 1 ? '' : ' download-chart--empty'}`} viewBox="0 0 600 38" preserveAspectRatio="none" role="img" aria-label="Measured download speed"><polyline points={points} fill="none" stroke="currentColor" strokeWidth="2" vectorEffect="non-scaling-stroke" /></svg>
        {job.error && <Alert type="error" showIcon message={job.error} />}
        <div className="download-actions">
          {job.actions.includes('pause') && <Button disabled={pending} icon={<PauseOutlined />} onClick={() => act('pause')}>Pause</Button>}
          {job.actions.includes('resume') && <Button disabled={pending} icon={<PlayCircleOutlined />} onClick={() => act('resume')}>Resume</Button>}
          {job.actions.includes('cancel') && <Button disabled={pending} icon={<StopOutlined />} onClick={() => act('cancel')}>Cancel</Button>}
          {job.state === 'complete' && job.output && <Button type="primary" icon={<PlayCircleOutlined />} disabled={pending} onClick={() => void sendCmd('pealayer.open', { target: job.output })}>Play in Pealayer</Button>}
          {job.actions.includes('remove') && <Button disabled={pending} icon={<DeleteOutlined />} onClick={() => act('remove')}>Remove from queue</Button>}
          {job.actions.includes('remux_mp4') && <Button disabled={pending} onClick={() => act('remux_mp4')}>Remux MP4</Button>}
          {job.actions.includes('extract_audio') && <Button disabled={pending} onClick={() => act('extract_audio')}>Extract audio</Button>}
          <span className="download-reorder"><Button aria-label="Move earlier in queue" title="Move earlier in queue" disabled={pending || !job.actions.includes('move_up')} icon={<ArrowUpOutlined />} onClick={() => act('move_up')} />
            <Button aria-label="Move later in queue" title="Move later in queue" disabled={pending || !job.actions.includes('move_down')} icon={<ArrowDownOutlined />} onClick={() => act('move_down')} /></span>
        </div>
      </Card>;
    })}
  </section>;
}
