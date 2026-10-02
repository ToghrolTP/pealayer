import React, { useEffect, useState } from 'react';
import { Card, Table, Breadcrumb, Button, Input, Tag, Space, Avatar, message, Popconfirm, Spin, Typography } from 'antd';
import {
  FolderOutlined,
  VideoCameraOutlined,
  PlayCircleOutlined,
  ArrowUpOutlined,
  ReloadOutlined,
  DeleteOutlined,
  SearchOutlined,
} from '@ant-design/icons';
import { tr, UiLocale } from '../i18n';

const { Text } = Typography;

interface FileEntry {
  name: string;
  path: string;
  is_dir: boolean;
  is_media: boolean;
  size_bytes: number;
  has_thumbnail: boolean;
}

interface BrowseResponse {
  current_path: string;
  parent_path: string | null;
  entries: FileEntry[];
}

interface MediaLibraryTabProps {
  sendCmd: (command: string, payload?: Record<string, any>) => void;
  onMediaPlayStarted?: () => void;
  locale: UiLocale;
  apiBaseUrl: string;
}

export const MediaLibraryTab: React.FC<MediaLibraryTabProps> = ({ sendCmd, onMediaPlayStarted, locale, apiBaseUrl }) => {
  const [data, setData] = useState<BrowseResponse | null>(null);
  const [loading, setLoading] = useState<boolean>(false);
  const [searchQuery, setSearchQuery] = useState<string>('');
  const [currentPath, setCurrentPath] = useState<string | undefined>(undefined);

  const fetchDirectory = async (path?: string) => {
    setLoading(true);
    try {
      const query = path ? `?path=${encodeURIComponent(path)}` : '';
      const res = await fetch(`${apiBaseUrl}/api/fs/browse${query}`);
      if (res.ok) {
        const json: BrowseResponse = await res.json();
        setData(json);
        setCurrentPath(json.current_path);
      } else {
        message.error(tr(locale, 'Failed loading directory'));
      }
    } catch {
      message.error(tr(locale, 'Error connecting to file system API'));
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    fetchDirectory(currentPath);
  }, []);

  const handlePlayMedia = (filePath: string, fileName: string) => {
    sendCmd('open_video', { path: filePath });
    message.success(`${tr(locale, 'Playing:')} ${fileName}`);
    if (onMediaPlayStarted) {
      onMediaPlayStarted();
    }
  };

  const handleDeleteFile = async (filePath: string) => {
    try {
      const res = await fetch(`${apiBaseUrl}/api/fs/trash`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ target_path: filePath }),
      });
      if (res.ok) {
        message.success(tr(locale, 'File deleted'));
        fetchDirectory(currentPath);
      } else {
        message.error(tr(locale, 'Failed to delete file'));
      }
    } catch {
      message.error(tr(locale, 'Error deleting file'));
    }
  };

  const formatSize = (bytes: number) => {
    if (bytes === 0) return '-';
    const k = 1024;
    const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + ' ' + sizes[i];
  };

  const filteredEntries = data?.entries.filter((item) =>
    item.name.toLowerCase().includes(searchQuery.toLowerCase())
  ) || [];

  const columns = [
    {
      title: tr(locale, 'Name'),
      dataIndex: 'name',
      key: 'name',
      render: (_: any, record: FileEntry) => (
        <Space size="middle">
          {record.is_dir ? (
            <Avatar shape="square" icon={<FolderOutlined />} className="media-library__avatar media-library__avatar--folder" />
          ) : record.has_thumbnail ? (
            <Avatar
              shape="square"
              src={`${apiBaseUrl}/api/fs/thumbnail?path=${encodeURIComponent(record.path)}`}
              icon={<VideoCameraOutlined />}
              className="media-library__avatar"
            />
          ) : (
            <Avatar shape="square" icon={<VideoCameraOutlined />} className="media-library__avatar media-library__avatar--video" />
          )}

          {record.is_dir ? (
            <Button
              type="link"
              onClick={() => fetchDirectory(record.path)}
              className="media-library__name"
            >
              {record.name}
            </Button>
          ) : (
            <Text className={record.is_media ? 'media-library__name' : 'media-library__name media-library__name--muted'}>
              {record.name}
            </Text>
          )}
        </Space>
      ),
    },
    {
      title: tr(locale, 'Type'),
      key: 'type',
      width: 120,
      render: (_: any, record: FileEntry) =>
        record.is_dir ? (
          <Tag color="warning">{tr(locale, 'Folder')}</Tag>
        ) : record.is_media ? (
          <Tag color="processing">{tr(locale, 'Media Video')}</Tag>
        ) : (
          <Tag color="default">{tr(locale, 'File')}</Tag>
        ),
    },
    {
      title: tr(locale, 'Size'),
      dataIndex: 'size_bytes',
      key: 'size_bytes',
      width: 120,
      render: (size: number, record: FileEntry) =>
        record.is_dir ? '-' : <Text type="secondary" style={{ fontFamily: 'monospace' }}>{formatSize(size)}</Text>,
    },
    {
      title: tr(locale, 'Actions'),
      key: 'actions',
      width: 140,
      render: (_: any, record: FileEntry) => (
        <Space size="small">
          {record.is_media && (
            <Button
              type="primary"
              size="small"
              icon={<PlayCircleOutlined />}
              onClick={() => handlePlayMedia(record.path, record.name)}
            >
              {tr(locale, 'Play')}
            </Button>
          )}
          {!record.is_dir && (
            <Popconfirm
              title={tr(locale, 'Delete File')}
              description={tr(locale, 'Are you sure you want to delete this file?')}
              onConfirm={() => handleDeleteFile(record.path)}
              okText={tr(locale, 'Delete')}
              okButtonProps={{ danger: true }}
              cancelText={tr(locale, 'Cancel')}
            >
              <Button type="text" danger size="small" icon={<DeleteOutlined />} />
            </Popconfirm>
          )}
        </Space>
      ),
    },
  ];

  const pathParts = data?.current_path ? data.current_path.split('/').filter(Boolean) : [];

  const breadcrumbItems = [
    {
      title: (
        <a onClick={() => fetchDirectory('/')}>
          {tr(locale, 'Root')}
        </a>
      ),
    },
    ...pathParts.map((part, index) => {
      const subPath = '/' + pathParts.slice(0, index + 1).join('/');
      return {
        title: (
          <a onClick={() => fetchDirectory(subPath)}>
            {part}
          </a>
        ),
      };
    }),
  ];

  return (
    <Card bordered={false} className="surface-card media-library" bodyStyle={{ padding: 20 }}>
      {/* Header controls: Breadcrumb & Search bar */}
      <Space direction="vertical" style={{ width: '100%' }} size="middle">
        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', flexWrap: 'wrap', gap: 12 }}>
          <Space>
            {data?.parent_path && (
              <Button
                icon={<ArrowUpOutlined />}
                onClick={() => fetchDirectory(data.parent_path!)}
                title={tr(locale, 'Go Up Directory')}
              >
                {tr(locale, 'Up')}
              </Button>
            )}
            <Button icon={<ReloadOutlined />} onClick={() => fetchDirectory(currentPath)}>
              {tr(locale, 'Refresh')}
            </Button>
          </Space>

          <Input
            prefix={<SearchOutlined />}
            placeholder={tr(locale, 'Search media files...')}
            allowClear
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            className="media-library__search"
          />
        </div>

        {/* Current Path Breadcrumbs */}
        <Breadcrumb items={breadcrumbItems} className="media-library__breadcrumbs" />

        {/* File Table */}
        <Spin spinning={loading}>
          <Table
            dataSource={filteredEntries}
            columns={columns}
            rowKey="path"
            pagination={{ pageSize: 15, showSizeChanger: true }}
            style={{ marginTop: 8 }}
          />
        </Spin>
      </Space>
    </Card>
  );
};
