// Real status bar/tooltip; synthetic snapshots and a memory-only preference sink.
import React, {useState} from 'react';
import {createRoot} from 'react-dom/client';
import {ConfigProvider, theme} from 'antd';
import {ApplicationStatusBar} from '../src/components/ApplicationStatusBar';
import type {PlayerState} from '../src/components/RemoteControlTab';
import {DEFAULT_VISIBILITY, type StatusBarVisibility} from '../src/statusWidgets';
import palettes from '../../assets/themes/palettes.json';
import '../src/styles.css';
const params = new URLSearchParams(location.search);
const light = params.has('light'), rtl = params.has('rtl');
const palette = palettes[params.has('neutral') ? 'native' : 'studio'][light ? 'light' : 'dark'];
document.documentElement.dataset.theme = light ? 'light' : 'dark';
document.documentElement.dir = rtl ? 'rtl' : 'ltr';
for (const [role, value] of Object.entries(palette)) document.documentElement.style.setProperty(`--${role}`, value);
document.documentElement.style.setProperty('--accent', palette.blue);
window.fetch = async () => { throw Error('Unexpected fixture network request'); };
function Fixture() {
  const [visibility, setVisibility] = useState<StatusBarVisibility>({...DEFAULT_VISIBILITY,
    fps_mode: 'media', media_rate: true, hardware: false, workspace: false, telemetry: false});
  const state = {playing: true, current_video: 'fixture.mp4', media_fps: params.has('unknown') ? null : 23.976,
    ui_fps: params.has('unknown') ? null : 30.62, hardware_connected: false} as unknown as PlayerState;
  return <ConfigProvider direction={rtl ? 'rtl' : 'ltr'} theme={{algorithm: light ? theme.defaultAlgorithm : theme.darkAlgorithm}}>
    <div style={{padding:16}}>Frame rate tooltip review</div>
    <ApplicationStatusBar state={state} connected={false} connectionMode="http" locale={rtl ? 'fa' : 'en'}
      sendCmd={()=>{throw Error('Unexpected command');}} visibility={visibility} onVisibilityChange={setVisibility} onNavigate={()=>{throw Error('Unexpected navigation');}}/>
    <output id="fixture-state" hidden>{JSON.stringify({mode:visibility.fps_mode})}</output>
  </ConfigProvider>;
}
createRoot(document.getElementById('root')!).render(<Fixture/>);
