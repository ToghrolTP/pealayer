// Isolated renderer fixture. No socket, media source or hardware connection.
import React, { useState } from 'react';
import { createRoot } from 'react-dom/client';
import { ConfigProvider, theme } from 'antd';
import { HardwareTab } from '../src/components/HardwareTab';
import type { PlayerState } from '../src/components/RemoteControlTab';
import '../src/styles.css';
function Fixture() {
  const mode = new URLSearchParams(location.search).get('state');
  const light = new URLSearchParams(location.search).has('light');
  const error = new URLSearchParams(location.search).has('error');
  document.documentElement.dataset.theme = light ? 'light' : 'dark';
  const [commands,setCommands] = useState<unknown[]>([]);
  const state = { controller_connected: mode === 'board' || mode === 'catalog', hardware_connected: mode === 'stale',
    hardware_endpoint: 'configured-by-user', hardware_connection_requested: mode === 'connecting',
    hardware: {supports_rf_transmit:true},
    hardware_error: error ? 'embedded PCController unavailable (no packaged PCController library found; set PEALAYER_PCCONTROLLER_LIBRARY); external coordinator unavailable (connection timed out)' : null,
    hardware_details: mode === 'catalog' ? undefined : {board_name:'Cached fixture board',controls:[],folders:[],folder_update:{pending:false,sequence:0,error:''},outputs_available:false},
  } as PlayerState;
  return <ConfigProvider theme={{ algorithm: light ? theme.defaultAlgorithm : theme.darkAlgorithm,token:{colorPrimary:'#8c83df'} }}>
    <main style={{padding:16,minWidth:0,width:'100%',boxSizing:'border-box'}}>
      <HardwareTab state={state} locale="en" onPreferences={() => setCommands([...commands,['preferences']])}
        sendCmd={async (...command) => {setCommands(current => [...current,command]);return true;}} />
      <output id="fixture-state" hidden>{JSON.stringify(commands)}</output>
    </main>
  </ConfigProvider>;
}
createRoot(document.getElementById('root')!).render(<Fixture />);
