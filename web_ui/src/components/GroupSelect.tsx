import React, { useState } from 'react';
import { Select } from 'antd';
import { tr, UiLocale } from '../i18n';

interface GroupSelectProps {
  value: string;
  groups: string[];
  onChange: (value: string) => void;
  locale: UiLocale;
}

/** A single group, not a tag list. Search is a draft until explicitly selected. */
export const GroupSelect: React.FC<GroupSelectProps> = ({ value, groups, onChange, locale }) => {
  const [search, setSearch] = useState('');
  const names = [...new Set([...groups, value].map((name) => name.trim()).filter(Boolean))].sort();
  const query = search.trim();
  const options = [
    { value: '', label: tr(locale, 'Ungrouped') },
    ...names.filter((name) => name.toLocaleLowerCase().includes(query.toLocaleLowerCase()))
      .map((name) => ({ value: name, label: name })),
    ...(!query || names.includes(query) ? [] : [{ value: query, label: `${tr(locale, 'Create group')}: ${query}` }]),
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
    onOpenChange={(open) => { if (!open) setSearch(''); }}
    onChange={(group) => { onChange(group); setSearch(''); }}
  />;
};
