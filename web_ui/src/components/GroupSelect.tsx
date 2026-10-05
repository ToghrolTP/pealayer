import React, { useState } from 'react';
import { Button, Divider, Select } from 'antd';
import { PlusOutlined } from '@ant-design/icons';
import { tr, UiLocale } from '../i18n';

interface GroupSelectProps {
  value: string;
  groups: string[];
  onChange: (value: string) => void;
  locale: UiLocale;
  onCreate?: () => void;
}

/** A single group, not a tag list. Search is a draft until explicitly selected. */
export const GroupSelect: React.FC<GroupSelectProps> = ({ value, groups, onChange, locale, onCreate }) => {
  const [search, setSearch] = useState('');
  const names = [...new Set([...groups, ...(onCreate ? [] : [value])].map((name) => name.trim()).filter(Boolean))].sort();
  const query = search.trim();
  const options = [
    ...(onCreate ? [] : [{ value: '', label: tr(locale, 'Ungrouped') }]),
    ...names.filter((name) => name.toLocaleLowerCase().includes(query.toLocaleLowerCase()))
      .map((name) => ({ value: name, label: name })),
    ...(onCreate || !query || names.includes(query) ? [] : [{ value: query, label: `${tr(locale, 'Create group')}: ${query}` }]),
  ];
  return <Select
    aria-label={tr(locale, 'Group')}
    style={{ width: '100%', minWidth: 0 }}
    showSearch
    searchValue={search}
    onSearch={setSearch}
    filterOption={false}
    value={value.trim()}
    options={options}
    popupRender={(menu) => <>{menu}{onCreate && <><Divider style={{ margin: '6px 0' }} /><Button type="text" icon={<PlusOutlined />} onClick={onCreate}>{tr(locale, 'New...')}</Button></>}</>}
    onOpenChange={(open) => { if (!open) setSearch(''); }}
    onChange={(group) => { onChange(group); setSearch(''); }}
  />;
};
