import React, { useEffect, useRef, useState } from 'react';
import { Alert, Button, Input, Modal, Table, Typography } from 'antd';
import { ArrowUpOutlined, FileImageOutlined, FolderOpenOutlined, ReloadOutlined } from '@ant-design/icons';
import { tr, type UiLocale } from '../i18n';
import { preferenceFileAllowed, preferenceFileParent } from '../preferenceFiles';

interface FileEntry { name: string; path: string; is_dir: boolean; size_bytes: number }
interface Directory { current_path: string; parent_path: string | null; entries: FileEntry[] }
interface Props {
  apiBaseUrl: string; locale: UiLocale; extensions: string[];
  initialFile?: string; title: string; onSelect: (path: string) => void; onCancel: () => void;
}

export const ServerFilePicker: React.FC<Props> = ({ apiBaseUrl, locale, extensions, initialFile, title, onSelect, onCancel }) => {
  const [directory, setDirectory] = useState<Directory | null>(null);
  const [path, setPath] = useState('');
  const [filter, setFilter] = useState('');
  const [selected, setSelected] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const request = useRef<AbortController | null>(null);
  const browse = async (next = '') => {
    request.current?.abort();
    const controller = new AbortController();
    request.current = controller;
    setLoading(true); setError(null); setSelected(null); setDirectory(null);
    try {
      const response = await fetch(`${apiBaseUrl}/api/fs/browse${next ? `?path=${encodeURIComponent(next)}` : ''}`, { signal: controller.signal });
      const value = await response.json();
      if (!response.ok) throw new Error(value.error || `Browse failed (${response.status})`);
      if (controller.signal.aborted) return;
      setDirectory(value); setPath(value.current_path);
    } catch (reason) {
      if (!controller.signal.aborted) setError(reason instanceof Error ? reason.message : String(reason));
    } finally { if (!controller.signal.aborted) setLoading(false); }
  };
  useEffect(() => {
    void browse(preferenceFileParent(initialFile));
    return () => request.current?.abort();
  }, [apiBaseUrl]);
  const rows = (directory?.entries ?? []).filter(entry => (entry.is_dir
    || preferenceFileAllowed(entry.name, extensions))
    && entry.name.toLocaleLowerCase().includes(filter.toLocaleLowerCase()));
  return <Modal open title={title} width={680} style={{ maxWidth: 'calc(100vw - 32px)' }}
    onCancel={onCancel} onOk={() => selected && onSelect(selected)} okText={tr(locale, 'Select')}
    cancelText={tr(locale, 'Cancel')} okButtonProps={{ disabled: !selected || loading }}>
    <div className="server-file-picker__navigation">
      <Button icon={<ArrowUpOutlined />} disabled={!directory?.parent_path || loading}
        title={tr(locale, 'Parent folder')} aria-label={tr(locale, 'Parent folder')}
        onClick={() => void browse(directory?.parent_path ?? '')} />
      <Input value={path} aria-label={tr(locale, 'Folder path')} onChange={event => setPath(event.target.value)}
        onPressEnter={() => void browse(path)} />
      <Button icon={<ReloadOutlined />} loading={loading} title={tr(locale, 'Load folder')}
        aria-label={tr(locale, 'Load folder')} onClick={() => void browse(path)} />
    </div>
    <Input.Search allowClear placeholder={tr(locale, 'Filter files')} value={filter} onChange={event => setFilter(event.target.value)} />
    {error && <Alert type="error" showIcon message={error} />}
    <Table<FileEntry> rowKey="path" size="small" loading={loading} dataSource={rows} pagination={false}
      scroll={{ y: 300 }} showHeader={false} className="server-file-picker__files"
      rowSelection={{ type: 'radio', selectedRowKeys: selected ? [selected] : [],
        getCheckboxProps: entry => ({ disabled: entry.is_dir }), onChange: keys => setSelected(String(keys[0] ?? '') || null) }}
      onRow={entry => ({ onClick: () => entry.is_dir ? void browse(entry.path) : setSelected(entry.path),
        onDoubleClick: () => !entry.is_dir && onSelect(entry.path) })}
      columns={[{ key: 'name', ellipsis: true, render: (_, entry) => <span title={entry.path}>
        {entry.is_dir ? <FolderOpenOutlined /> : <FileImageOutlined />} <Typography.Text>{entry.name}</Typography.Text>
      </span> }]} />
  </Modal>;
};
