// Disconnected browser regression fixture; no network, media or hardware client.
import React, {useState} from 'react';
import {createRoot} from 'react-dom/client';
import {ConfigProvider, theme} from 'antd';
import {SequenceTimeline} from '../src/components/SequenceTimeline';
import type {SequenceStep} from '../src/cueAuthoring';
import '../src/styles.css';
function Fixture() {
  const light = new URLSearchParams(location.search).has('light');
  document.documentElement.dataset.theme = light ? 'light' : 'dark';
  const [steps,setSteps]=useState<SequenceStep[]>([
    {kind:'relay',target:0,value:1,at_us:0,duration_ms:500,repeat_count:2,repeat_interval_ms:750},
    {kind:'pwm',target:1,value:2048,at_us:500000,duration_ms:1000},
  ]);
  const [repeat,setRepeat]=useState({count:3,interval:2000});
  return <ConfigProvider theme={{algorithm:light ? theme.defaultAlgorithm : theme.darkAlgorithm,token:{colorPrimary:'#8c83df'}}}>
    <main style={{padding:12,minWidth:0,width:'100%',boxSizing:'border-box'}}>
      <h2>Effects Designer · Repeat projections</h2>
      <SequenceTimeline steps={steps} onChange={setSteps} locale="en" repeatCount={repeat.count} repeatIntervalMs={repeat.interval}
        onRepeatChange={(count,interval)=>setRepeat({count,interval})}/>
      <output id="fixture-state" hidden>{JSON.stringify({steps,repeat})}</output>
    </main>
  </ConfigProvider>;
}
createRoot(document.getElementById('root')!).render(<Fixture/>);
