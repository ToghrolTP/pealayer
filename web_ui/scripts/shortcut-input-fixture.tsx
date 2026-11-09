import React, { useState } from 'react';
import { createRoot } from 'react-dom/client';
import { ConfigProvider, theme } from 'antd';
import { ShortcutInput } from '../src/components/ShortcutInput';
import palettes from '../../assets/themes/palettes.json';
import '../src/styles.css';
const query = new URLSearchParams(location.search);
const light = query.has('light');
const colors = palettes[query.has('neutral') ? 'native' : 'studio'][light ? 'light' : 'dark'];
for (const [name, value] of Object.entries(colors)) document.documentElement.style.setProperty(`--${name}`, value);
document.documentElement.dir = query.has('rtl') ? 'rtl' : 'ltr';
function Fixture() {
  const [value, setValue] = useState('Ctrl+Shift+F10');
  const [commits, setCommits] = useState(0);
  return <ConfigProvider theme={{ algorithm: light ? theme.defaultAlgorithm : theme.darkAlgorithm }}>
    <main style={{ padding: 16, background: 'var(--surface-1)', color: 'var(--text)' }}>
      <label>Open media containing folder<ShortcutInput value={value} label="Shortcut" placeholder="Shortcut"
        onCommit={next => { setValue(next); setCommits(count => count + 1); }} /></label>
      <button id="outside">Outside field</button><output id="fixture-state">{JSON.stringify({ value, commits })}</output>
    </main>
  </ConfigProvider>;
}
createRoot(document.getElementById('root')!).render(<Fixture />);
