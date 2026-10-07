import React from 'react';
import { Input, Modal, Space } from 'antd';
import { EditOutlined, FolderAddOutlined, PlusOutlined, SaveOutlined } from '@ant-design/icons';
import { EffectIconPicker, effectGlyph } from '../effectIcons';
import { tr, UiLocale } from '../i18n';

export type EffectGroupDraft = {
  original_name: string;
  name: string;
  icon: string;
};

export const EffectGroupDialog: React.FC<{
  draft: EffectGroupDraft | null;
  setDraft: (draft: EffectGroupDraft | null) => void;
  save: () => void;
  locale: UiLocale;
}> = ({ draft, setDraft, save, locale }) => {
  const editing = Boolean(draft?.original_name.trim());
  return <Modal
    title={<Space>{editing ? <EditOutlined /> : <FolderAddOutlined />}{tr(locale, editing ? 'Manage effect group' : 'New group')}</Space>}
    open={draft !== null} width={400} destroyOnHidden zIndex={1200}
    okText={tr(locale, editing ? 'Save to PCController' : 'Create group')} cancelText={tr(locale, 'Cancel')}
    okButtonProps={{ disabled: !draft?.name.trim(), icon: editing ? <SaveOutlined /> : <PlusOutlined /> }}
    onCancel={() => setDraft(null)} onOk={save}
  >
    {draft && <div className="effect-group-editor">
      <label>
        <span>{tr(locale, 'Name')}</span>
        <Input autoFocus aria-label={tr(locale, 'Group name')} placeholder={tr(locale, 'Group name')}
          value={draft.name} maxLength={64}
          onChange={(event) => setDraft({ ...draft, name: event.target.value })}
          onPressEnter={save} />
      </label>
      <label>
        <span>{tr(locale, 'Icon')}</span>
        <EffectIconPicker
          value={draft.icon || 'folder'}
          searchPlaceholder={tr(locale, 'Search icons...')}
          presetsLabel={tr(locale, 'Presets')}
          emptyLabel={tr(locale, 'No matching icons')}
          onChange={(icon) => setDraft({ ...draft, icon })}
        />
      </label>
      <div className="effect-group-editor__preview" aria-hidden="true">
        <span>{effectGlyph(draft.icon || 'folder')}</span>
        <strong>{draft.name.trim() || tr(locale, 'New group')}</strong>
      </div>
    </div>}
  </Modal>;
};
