import React from 'react';
import { Input, Modal, Space } from 'antd';
import { FolderAddOutlined, PlusOutlined } from '@ant-design/icons';
import { tr, UiLocale } from '../i18n';

export const EffectGroupDialog: React.FC<{
  name: string | null;
  setName: (name: string | null) => void;
  create: () => void;
  locale: UiLocale;
}> = ({ name, setName, create, locale }) => <Modal
  title={<Space><FolderAddOutlined />{tr(locale, 'New group')}</Space>}
  open={name !== null} width={360} destroyOnHidden zIndex={1200}
  okText={tr(locale, 'Create group')} cancelText={tr(locale, 'Cancel')}
  okButtonProps={{ disabled: !name?.trim(), icon: <PlusOutlined /> }}
  onCancel={() => setName(null)} onOk={create}
>
  <Input autoFocus aria-label={tr(locale, 'Group name')} placeholder={tr(locale, 'Group name')}
    value={name ?? ''} onChange={(event) => setName(event.target.value)} onPressEnter={create} />
</Modal>;
