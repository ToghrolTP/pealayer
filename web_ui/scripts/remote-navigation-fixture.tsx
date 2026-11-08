// Real remote browser, memory-only command transport. Never opens remote media.
import React, {useState} from 'react';
import {createRoot} from 'react-dom/client';
import {ConfigProvider, theme} from 'antd';
import {RemoteLocationDialog, type RemoteBrowser} from '../src/components/RemoteLocationDialog';
import '../src/styles.css';
import '../src/remote-location.css';
const params = new URLSearchParams(location.search);
const light = params.has('light');
document.documentElement.dataset.theme = light ? 'light' : 'dark';
window.fetch = async () => { throw Error('Unexpected fixture network request'); };
const target = 'https://files.invalid/folder/';
const state: RemoteBrowser = {
  request_id: 1, revision: 1, visible: true, loading: false, target, use_proxy: false,
  auto_next: false, thumbnails: false, sort: 'name', descending: false, selected: null,
  error: null, route_errors: [],
  listing: {url: target, host: 'files.invalid', server: null, content_type: 'text/html',
    parent_url: params.has('parent') ? 'https://files.invalid/' : null,
    warning: null, file: null, entries: [
      {name: 'Child folder', url: `${target}child/`, is_dir: true, playable: false, size_bytes: null, modified: null},
      {name: 'Movie.mkv', url: `${target}Movie.mkv`, is_dir: false, playable: true, size_bytes: 1024, modified: '2026-01-02'},
    ]},
};
function Fixture() {
  const [commands, setCommands] = useState<unknown[]>([]);
  return <ConfigProvider theme={{algorithm: light ? theme.defaultAlgorithm : theme.darkAlgorithm, token: {colorPrimary: '#8c83df'}}}>
    <RemoteLocationDialog state={state} connected base="" autoInspect={false} sendCmd={(...cmd) => setCommands(current => [...current, cmd])}/>
    <output id="fixture-state" hidden>{JSON.stringify({commands})}</output>
  </ConfigProvider>;
}
createRoot(document.getElementById('root')!).render(<Fixture/>);
