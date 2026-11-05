import React, { useRef, useState } from 'react';
import { Button, message, Progress } from 'antd';
import { ImportOutlined } from '@ant-design/icons';
import { externalFiles, importAssetFiles } from '../assetImports';
import type { AssetDestination } from '../assetImports';

export function AssetDropTarget({ children, destination, apiBaseUrl, sendCmd, destinationForDrop }: {
  children: React.ReactNode; destination: AssetDestination; apiBaseUrl: string;
  sendCmd: (command: string, payload?: Record<string, unknown>) => Promise<boolean>;
  destinationForDrop?: (event: React.DragEvent<HTMLDivElement>) => AssetDestination;
}) {
  const input = useRef<HTMLInputElement>(null);
  const [progress, setProgress] = useState<number | null>(null);
  const [hover, setHover] = useState(false);
  const controller = useRef<AbortController | null>(null);
  const upload = async (files: File[], target = destination) => {
    if (controller.current) return;
    const transfer = new AbortController(); controller.current = transfer; setProgress(0);
    try { await importAssetFiles(files, target, apiBaseUrl, sendCmd, setProgress, transfer.signal); }
    catch (error) { if (!transfer.signal.aborted) void message.error(String(error instanceof Error ? error.message : error)); }
    finally { controller.current = null; setProgress(null); }
  };
  return <div className={`asset-drop-target ${hover ? 'is-external-drop' : ''}`}
    onDragOver={event => { if (externalFiles(event.dataTransfer)) { event.preventDefault(); event.dataTransfer.dropEffect = 'copy'; setHover(true); } }}
    onDragLeave={event => { if (!event.currentTarget.contains(event.relatedTarget as Node)) setHover(false); }}
    onDrop={event => {
      if (!externalFiles(event.dataTransfer) || event.defaultPrevented) return;
      event.preventDefault(); setHover(false); void upload(Array.from(event.dataTransfer.files), destinationForDrop?.(event) ?? destination);
    }}>
    <div className="asset-import-actions">
      <input ref={input} type="file" multiple hidden onChange={event => { void upload(Array.from(event.target.files ?? [])); event.target.value = ''; }} />
      <Button icon={<ImportOutlined />} disabled={progress !== null} onClick={() => input.current?.click()}>Import assets</Button>
      {progress !== null && <><Progress percent={progress} size="small" status={progress === 100 ? 'active' : 'normal'} showInfo={progress !== 100} /><span>{progress === 100 ? 'Preparing assets…' : 'Transferring…'}</span><Button onClick={() => controller.current?.abort()}>Cancel</Button></>}
    </div>
    {children}
  </div>;
}
