import React, { useState, useEffect, useRef } from 'react';
import { Button, Input, InputNumber, Select, Space, Typography, message } from 'antd';
import { externalFiles, uploadAssetFiles } from '../assetImports';
import { FolderOpenOutlined, PlayCircleOutlined, StopOutlined } from '@ant-design/icons';
import { ServerFilePicker } from './ServerFilePicker';
import type { PlayerState } from './RemoteControlTab';
import { tr, type UiLocale } from '../i18n';

export interface AudioProgram { source?: string; volume?: number; output_device?: string; }
export const SoundEffectFields: React.FC<{
  program: AudioProgram;
  onChange: (patch: Partial<AudioProgram>) => void;
  state: PlayerState;
  reference: string;
  apiBaseUrl: string;
  locale: UiLocale;
  sendCmd: (command: string, payload?: Record<string, any>) => Promise<boolean>;
}> = ({ program, onChange, state, reference, apiBaseUrl, locale, sendCmd }) => {
  const [browse, setBrowse] = useState(false);
  const [transferring,setTransferring]=useState(false);
  const transfer=useRef<AbortController|null>(null);
  useEffect(()=>()=>{transfer.current?.abort();},[]);
  const devices = state.audio_devices ?? [];
  const output = program.output_device ?? '';
  return <div className="effect-editor-grid effect-editor-grid__wide">
    <label className="effect-editor-grid__wide"
      onDragOver={event=>{if(externalFiles(event.dataTransfer)){event.preventDefault();event.stopPropagation();event.dataTransfer.dropEffect='copy';}}}
      onDrop={event=>{
        if(!externalFiles(event.dataTransfer))return;
        event.preventDefault();event.stopPropagation();
        const files=Array.from(event.dataTransfer.files);
        if(files.length!==1 || !/\.(wav|mp3|flac|ogg|opus|m4a|aac|aiff?|wma)$/i.test(files[0].name)){void message.error('Choose one audio file for this sound effect');return;}
        if(transfer.current)return;
        const controller=new AbortController();transfer.current=controller;setTransferring(true);
        void uploadAssetFiles(files,apiBaseUrl,undefined,controller.signal).then(async receipt=>{if(controller.signal.aborted){await receipt.cancel();return;}try{await receipt.retain();if(!controller.signal.aborted)onChange({source:receipt.paths[0]});}catch(error){await receipt.cancel();throw error;}}).catch(error=>{if(!controller.signal.aborted)void message.error(String(error));}).finally(()=>{transfer.current=null;if(!controller.signal.aborted)setTransferring(false);});
      }}><span>{tr(locale, 'Audio file / URL')}{transferring?' · Transferring…':''}</span><Space.Compact style={{ width: '100%' }}><Input value={program.source} disabled={transferring} onChange={(e) => onChange({ source: e.target.value })} /><Button icon={<FolderOpenOutlined />} disabled={transferring} onClick={() => setBrowse(true)}>{tr(locale, 'Browse')}</Button></Space.Compact></label>
    <label><span>{tr(locale, 'Volume')}</span><InputNumber min={0} max={100} addonAfter="%" value={program.volume ?? 100} onChange={(volume) => onChange({ volume: volume ?? 100 })} /></label>
    <label><span>{tr(locale, 'Output device / backend')}</span><Select value={output} onOpenChange={(open) => { if (open) void sendCmd('audio.outputs.refresh'); }} onChange={(output_device) => onChange({ output_device })} options={[{ value: '', label: tr(locale, 'Preferences SFX output') }, ...devices.map((d) => ({ value: d.name, label: d.description })), ...(output && !devices.some((d) => d.name === output) ? [{ value: output, label: `${tr(locale, 'Unavailable device')}: ${output}` }] : [])]} /></label>
    <Space className="effect-editor-grid__wide" wrap><Button disabled={!reference || state.estop_active} icon={<PlayCircleOutlined />} onClick={() => sendCmd('controller_effect.play', { reference })}>{tr(locale, 'Preview')}</Button><Button icon={<StopOutlined />} onClick={() => sendCmd('audio_effect.stop')}>{tr(locale, 'Stop')}</Button>{state.audio_import_pending && <Typography.Text>{tr(locale, 'Reading audio…')}</Typography.Text>}</Space>
    {browse && <ServerFilePicker apiBaseUrl={apiBaseUrl} locale={locale} extensions={['wav', 'mp3', 'ogg', 'flac', 'm4a', 'aac', 'opus']} initialFile={program.source} title={tr(locale, 'Choose sound effect')} onSelect={(source) => { onChange({ source }); setBrowse(false); }} onCancel={() => setBrowse(false)} />}
  </div>;
};
