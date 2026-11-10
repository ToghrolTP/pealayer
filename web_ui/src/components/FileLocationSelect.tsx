import React, { useEffect, useRef, useState } from 'react';
import { Button, Select, Tooltip } from 'antd';
import { CloudOutlined, FolderOutlined, HddOutlined, ReloadOutlined, WarningOutlined } from '@ant-design/icons';
import { parseFileLocations, type FileLocation } from '../fileLocations';
import { tr, type UiLocale } from '../i18n';

export const FileLocationSelect: React.FC<{
  apiBaseUrl: string; locale: UiLocale; disabled?: boolean; onSelect: (path: string) => void;
}> = ({ apiBaseUrl, locale, disabled, onSelect }) => {
  const [locations, setLocations] = useState<FileLocation[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const request = useRef<AbortController | null>(null);
  const refresh = async () => {
    request.current?.abort();
    const controller = new AbortController(); request.current = controller;
    setLoading(true); setError(null);
    try {
      const response = await fetch(`${apiBaseUrl}/api/fs/locations`, { signal: controller.signal });
      if (!response.ok) throw new Error(`${tr(locale, 'Failed loading locations')} (${response.status})`);
      const value = parseFileLocations(await response.json());
      if (!controller.signal.aborted) setLocations(value);
    } catch (reason) {
      if (!controller.signal.aborted) setError(reason instanceof Error ? reason.message : String(reason));
    } finally { if (!controller.signal.aborted) setLoading(false); }
  };
  useEffect(() => { setLocations([]); void refresh(); return () => request.current?.abort(); }, [apiBaseUrl]);
  const groups: [string, FileLocation['kind'][]][] = [
    ['Drives', ['drive', 'removable']], ['Network locations', ['network']],
    ['Folders', ['folder', 'shortcut']], ['Recent folders', ['recent']],
  ];
  return <div className="file-location-select">
    <Select showSearch value={null} placeholder={tr(locale, 'Locations')} aria-label={tr(locale, 'Locations')}
      disabled={disabled} loading={loading} className="file-location-select__input" popupMatchSelectWidth={false}
      optionFilterProp="searchText" onChange={path => onSelect(String(path))}
      options={groups.flatMap(([name, kinds]) => {
        const entries = locations.filter(item => kinds.includes(item.kind));
        return entries.length ? [{ label: tr(locale, name), options: entries.map(item => ({
          value: item.path, searchText: `${item.name} ${item.path}`,
          label: <span className="file-location-select__label" title={item.path}>
            {item.kind === 'network' ? <CloudOutlined /> : ['drive', 'removable'].includes(item.kind) ? <HddOutlined /> : <FolderOutlined />}
            <span>{tr(locale, item.name)}</span>
          </span>,
        })) }] : [];
      })} />
    <Tooltip title={error ?? tr(locale, 'Refresh locations')}>
      <Button type="text" danger={!!error} loading={loading} disabled={disabled}
        aria-label={tr(locale, 'Refresh locations')} icon={error ? <WarningOutlined /> : <ReloadOutlined />}
        onClick={() => void refresh()} />
    </Tooltip>
  </div>;
};
