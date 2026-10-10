import React, { useEffect, useState } from 'react';
import { Alert, Button, Dropdown, Empty, Input, Modal, Popconfirm, Space, Tag } from 'antd';
import { FolderAddOutlined, FolderOpenOutlined, MoreOutlined } from '@ant-design/icons';
import { EffectIconPicker, effectGlyph } from '../effectIcons';
import { channelFolderKind, isRawSeatRelay, ChannelFolder } from '../channelFolders';
import { tr, UiLocale } from '../i18n';
import type { PlayerState } from './RemoteControlTab';

type Details = NonNullable<PlayerState['hardware_details']>;
type Draft = { kind: string; original?: string; name: string; icon: string; saving: boolean; revision: string; sequence: number; error?: string };

export function useHardwareFolders(details: Details | null | undefined, locale: UiLocale,
  sendCmd: (command: string, payload?: Record<string, unknown>) => Promise<boolean>,
  dragKey: string | null, finishDrag: () => void, manageChannels: () => void) {
  const [manager, setManager] = useState<string | null>(null);
  const [draft, setDraft] = useState<Draft | null>(null);
  const [drop, setDrop] = useState<string | null>(null);
  const [closed, setClosed] = useState<Set<string>>(new Set());
  const folders = details ? details.folders : [];
  const enabled = Boolean(details?.profile?.attached && details.profile.configured) && !details?.folder_update?.pending;
  const revision = details?.profile?.revision ?? '';
  useEffect(() => {
    if (!draft?.saving || !details || details.folder_update.sequence <= draft.sequence || details.folder_update.pending) return;
    if (details?.folder_update?.error) setDraft({ ...draft, saving: false, error: details.folder_update.error });
    else if (revision !== draft.revision) setDraft(null);
  }, [details, revision, draft]);
  // Switching boards cannot carry an editor or collapse state onto another profile.
  useEffect(() => { setDraft(null); setManager(null); setClosed(new Set()); }, [details?.profile?.key, details?.port?.serial_number, details?.port?.name]);
  const update = (fields: Record<string, unknown>) => sendCmd('hardware.folder.update', fields);
  const edit = (kind: string, folder?: ChannelFolder) => setDraft({ kind, original: folder?.name, name: folder?.name ?? '', icon: folder?.icon ?? '', saving: false, revision, sequence: details?.folder_update.sequence ?? 0 });
  const move = (kind: string, name: string, keys: string[]) => update({ operation: 'move', kind, name, keys });
  const moveItems = (control: Details['controls'][number]) => isRawSeatRelay(control) ? [] : [{
    key: 'move-folder', label: tr(locale, 'Move to folder'), disabled: !enabled,
    children: [{ name: '', kind: channelFolderKind(control.kind), icon: '' }, ...folders.filter((folder) => folder.kind === channelFolderKind(control.kind))]
      .filter((folder) => folder.name !== control.group).map((folder) => ({ key: `folder:${folder.name}`, label: folder.name || tr(locale, 'Ungrouped') })),
  }];
  const sectionMenu = (kind: string, children: React.ReactNode) => <Dropdown trigger={['contextMenu']} menu={{ items: [
    { key: 'channels', label: tr(locale, 'Manage channels') },
    { key: 'folders', label: tr(locale, 'Manage folders') },
    { key: 'new', label: tr(locale, 'New folder'), disabled: !enabled },
    { key: 'expand', label: tr(locale, 'Expand folders') },
    { key: 'collapse', label: tr(locale, 'Collapse folders') },
  ], onClick: ({ key, domEvent }) => {
    domEvent.stopPropagation();
    if (key === 'channels') manageChannels();
    if (key === 'folders') setManager(kind);
    if (key === 'new') edit(kind);
    if (key === 'expand') setClosed((old) => new Set([...old].filter((id) => !id.startsWith(`${kind}:`))));
    if (key === 'collapse') setClosed((old) => new Set([...old, `${kind}:`, `${kind}:raw`, ...folders.filter((folder) => folder.kind === kind).map((folder) => `${kind}:${folder.name}`)]));
  } }}><span className="hardware-section-title">{children}</span></Dropdown>;
  const dropProps = (kind: string, name: string, raw = false) => {
    const source = details?.controls.find((control) => control.key === dragKey);
    const valid = enabled && !raw && source && !isRawSeatRelay(source) && channelFolderKind(source.kind) === kind && source.group !== name;
    const id = `${kind}:${name}`;
    return {
      className: drop === id ? 'is-folder-drop-target' : '',
      onDragOver: (event: React.DragEvent) => { if (valid) { event.preventDefault(); event.stopPropagation(); event.dataTransfer.dropEffect = 'move'; setDrop(id); } },
      onDragLeave: (event: React.DragEvent) => { if (!event.currentTarget.contains(event.relatedTarget as Node)) setDrop(null); },
      onDrop: (event: React.DragEvent) => {
        if (!valid || !source) return;
        event.preventDefault(); event.stopPropagation();
        void move(kind, name, [source.key]); setDrop(null); finishDrag();
        setClosed((old) => { const next = new Set(old); next.delete(id); return next; });
      },
    };
  };
  const folderHeader = (folder: ChannelFolder & { raw: boolean; controls: unknown[] }) => {
    const id = `${folder.kind}:${folder.raw ? 'raw' : folder.name}`;
    const props = dropProps(folder.kind, folder.name, folder.raw);
    return <Dropdown trigger={folder.raw ? [] : ['contextMenu']} menu={{ items: [
      { key: 'new', label: tr(locale, 'New folder'), disabled: !enabled },
      { key: 'manage', label: tr(locale, 'Manage folder'), disabled: !enabled || !folder.name },
    ], onClick: ({ key, domEvent }) => {
      domEvent.stopPropagation();
      if (key === 'new') edit(folder.kind); else edit(folder.kind, folder);
    } }}>
      <header {...props} className={`hardware-folder-header ${props.className} ${folder.controls.length ? '' : 'is-empty'}`}>
        <button type="button" aria-disabled={!folder.controls.length || undefined} aria-expanded={folder.controls.length ? !closed.has(id) : undefined} onClick={() => folder.controls.length && setClosed((old) => {
          const next = new Set(old); if (next.has(id)) next.delete(id); else next.add(id); return next;
        })}>{folder.controls.length > 0 && <span>{closed.has(id) ? '›' : '⌄'}</span>}{folder.icon ? effectGlyph(folder.icon) : <FolderOpenOutlined />}{folder.raw ? tr(locale, 'Raw relays') : folder.name || tr(locale, 'Ungrouped')}<Tag>{folder.controls.length}</Tag></button>
        {!folder.raw && folder.name && <Button size="small" type="text" icon={<MoreOutlined />} aria-label={tr(locale, 'Manage folder')} disabled={!enabled} onClick={() => edit(folder.kind, folder)} />}
      </header>
    </Dropdown>;
  };
  const save = async (deleting = false) => {
    if (!draft || draft.saving || !enabled || (!deleting && !draft.name.trim())) return;
    setDraft({ ...draft, saving: true, revision, sequence: details?.folder_update.sequence ?? 0, error: undefined });
    const accepted = await update(deleting ? { operation: 'delete', kind: draft.kind, name: draft.original }
      : draft.original ? { operation: 'update', kind: draft.kind, name: draft.original, next_name: draft.name.trim(), icon: draft.icon }
        : { operation: 'create', kind: draft.kind, name: draft.name.trim(), icon: draft.icon });
    if (!accepted) setDraft((current) => current && ({ ...current, saving: false, error: tr(locale, 'Folder change was not accepted') }));
  };
  const dialogs = <>
    <Modal title={tr(locale, 'Manage folders')} open={manager !== null} footer={null} onCancel={() => setManager(null)} destroyOnHidden>
      <Button icon={<FolderAddOutlined />} disabled={!enabled} onClick={() => manager && edit(manager)}>{tr(locale, 'New folder')}</Button>
      <div className="hardware-folder-manager">
        {folders.filter((folder) => folder.kind === manager).map((folder) => {
          const props = dropProps(folder.kind, folder.name);
          return <div key={folder.name} {...props}><Space>{folder.icon ? effectGlyph(folder.icon) : <FolderOpenOutlined />}{folder.name}<Tag>{details?.controls.filter((control) => channelFolderKind(control.kind) === folder.kind && control.group === folder.name).length}</Tag></Space><Button disabled={!enabled} onClick={() => edit(folder.kind, folder)}>{tr(locale, 'Manage')}</Button></div>;
        })}
        {manager && <div {...dropProps(manager, '')}><FolderOpenOutlined />{tr(locale, 'Ungrouped')}</div>}
        {!folders.some((folder) => folder.kind === manager) && <Empty description={tr(locale, 'No folders')} />}
      </div>
    </Modal>
    <Modal title={tr(locale, draft?.original ? 'Manage folder' : 'New folder')} open={draft !== null} zIndex={1200} destroyOnHidden
      onCancel={() => setDraft(null)} onOk={() => void save()} confirmLoading={draft?.saving}
      okText={tr(locale, 'Save')} cancelText={tr(locale, 'Cancel')} okButtonProps={{ disabled: !enabled || !draft?.name.trim() }}>
      {draft && <div className="effect-group-editor">
        <label><span>{tr(locale, 'Name')}</span><Input autoFocus maxLength={64} disabled={draft.saving} value={draft.name} onChange={(event) => setDraft({ ...draft, name: event.target.value })} onPressEnter={() => void save()} /></label>
        <label><span>{tr(locale, 'Icon')}</span><EffectIconPicker value={draft.icon || 'folder'} searchPlaceholder={tr(locale, 'Search icons...')} presetsLabel={tr(locale, 'Presets')} emptyLabel={tr(locale, 'No matching icons')} onChange={(icon) => !draft.saving && setDraft({ ...draft, icon })} /></label>
        {draft.error && <Alert type="error" message={draft.error} />}
        {draft.original && <Popconfirm title={tr(locale, 'Delete folder?')} description={tr(locale, 'Channels will be moved to Ungrouped.')} onConfirm={() => void save(true)}><Button danger disabled={!enabled || draft.saving}>{tr(locale, 'Delete folder')}</Button></Popconfirm>}
      </div>}
    </Modal>
  </>;
  return { folders, move, moveItems, sectionMenu, folderHeader, closed, dialogs };
}
