// Isolated real control; no API, configuration writes or hardware connection.
import React, { useState } from 'react';
import { createRoot } from 'react-dom/client';
import { ConfigProvider, theme } from 'antd';
import { ChoiceSlider } from '../src/components/PreferencesTab';
import '../src/styles.css';
function Fixture() {
  const light = new URLSearchParams(location.search).has('light');
  document.documentElement.dataset.theme = light ? 'light' : 'dark';
  const options = ['Extra compact', 'Compact', 'Regular', 'Comfortable', 'Spacious']
    .map((label, index) => ({label, value:['extra_compact', 'compact', 'regular', 'comfortable', 'spacious'][index]}));
  const [value, setValue] = useState('compact');
  const [busy, setBusy] = useState(false);
  const [reject, setReject] = useState(false);
  const [commits, setCommits] = useState<unknown[]>([]);
  return <ConfigProvider theme={{algorithm: light ? theme.defaultAlgorithm : theme.darkAlgorithm}}>
    <main style={{padding:16, maxWidth:560, boxSizing:'border-box'}}>
      <h3>Control density</h3>
      <ChoiceSlider options={options} value={value} label="Control density" locale="en" disabled={busy}
        onCommit={next => {setCommits(items => [...items,next]);setBusy(true);setTimeout(() => {
          if (!reject) setValue(String(next));setBusy(false);
        },150);}} />
      <button id="reject" onClick={() => setReject(!reject)}>Reject next saves</button>
      <output id="fixture-state" hidden>{JSON.stringify({value,busy,commits})}</output>
    </main>
  </ConfigProvider>;
}
createRoot(document.getElementById('root')!).render(<Fixture />);
