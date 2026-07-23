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
}

export const MediaLibraryTab: React.FC<MediaLibraryTabProps> = ({ sendCmd, onMediaPlayStarted }) => {
  const [data, setData] = useState<BrowseResponse | null>(null);
  const [loading, setLoading] = useState<boolean>(false);
  const [searchQuery, setSearchQuery] = useState<string>('');
  const [currentPath, setCurrentPath] = useState<string | undefined>(undefined);

  const fetchDirectory = async (path?: string) => {
    setLoading(true);
    try {
      const query = path ? `?path=${encodeURIComponent(path)}` : '';
      const res = await fetch(`/api/fs/browse${query}`);
      if (res.ok) {
        const json: BrowseResponse = await res.json();
        setData(json);
        setCurrentPath(json.current_path);
      } else {
        message.error('Failed loading directory');
      }
    } catch {
      message.error('Error connecting to file system API');
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    fetchDirectory(currentPath);
  }, []);

  const handlePlayMedia = (filePath: string, fileName: string) => {
    sendCmd('open_video', { path: filePath });
    message.success(`Playing: ${fileName}`);
    if (onMediaPlayStarted) {
      onMediaPlayStarted();
    }
  };

  const handleDeleteFile = async (filePath: string) => {
    try {
      const res = await fetch('/api/fs/trash', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ target_path: filePath }),
      });
      if (res.ok) {
        message.success('File deleted');
        fetchDirectory(currentPath);
      } else {
        message.error('Failed to delete file');
      }
    } catch {
      message.error('Error deleting file');
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
      title: 'Name',
      dataIndex: 'name',
      key: 'name',
      render: (_: any, record: FileEntry) => (
        <Space size="middle">
          {record.is_dir ? (
            <Avatar shape="square" icon={<FolderOutlined />} style={{ backgroundColor: '#0a2239', color: '#53a2be' }} />
          ) : record.has_thumbnail ? (
            <Avatar
              shape="square"
              src={`/api/fs/thumbnail?path=${encodeURIComponent(record.path)}`}
              icon={<VideoCameraOutlined />}
              style={{ backgroundColor: '#0a2239' }}
            />
          ) : (
            <Avatar shape="square" icon={<VideoCameraOutlined />} style={{ backgroundColor: '#0a2239', color: '#1d84b5' }} />
          )}

          {record.is_dir ? (
            <Button
              type="link"
              onClick={() => fetchDirectory(record.path)}
              style={{ padding: 0, fontWeight: 600, color: '#f8fafc' }}
            >
              {record.name}
            </Button>
          ) : (
            <Text style={{ color: record.is_media ? '#f8fafc' : '#94a3b8', fontWeight: record.is_media ? 500 : 400 }}>
              {record.name}
            </Text>
          )}
        </Space>
      ),
    },
    {
      title: 'Type',
      key: 'type',
      width: 120,
      render: (_: any, record: FileEntry) =>
        record.is_dir ? (
          <Tag color="warning">Folder</Tag>
        ) : record.is_media ? (
          <Tag color="processing">Media Video</Tag>
        ) : (
          <Tag color="default">File</Tag>
        ),
    },
    {
      title: 'Size',
      dataIndex: 'size_bytes',
      key: 'size_bytes',
      width: 120,
      render: (size: number, record: FileEntry) =>
        record.is_dir ? '-' : <Text type="secondary" style={{ fontFamily: 'monospace' }}>{formatSize(size)}</Text>,
    },
    {
      title: 'Actions',
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
              style={{ borderRadius: 6, backgroundColor: '#1d84b5' }}
            >
              Play
            </Button>
          )}
          {!record.is_dir && (
            <Popconfirm
              title="Delete File"
              description="Are you sure you want to delete this file?"
              onConfirm={() => handleDeleteFile(record.path)}
              okText="Delete"
              okButtonProps={{ danger: true }}
              cancelText="Cancel"
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
        <a onClick={() => fetchDirectory('/')} style={{ color: '#38bdf8' }}>
          Root
        </a>
      ),
    },
    ...pathParts.map((part, index) => {
      const subPath = '/' + pathParts.slice(0, index + 1).join('/');
      return {
        title: (
          <a onClick={() => fetchDirectory(subPath)} style={{ color: '#cbd5e1' }}>
            {part}
          </a>
        ),
      };
    }),
  ];

  return (
    <Card
      bordered={false}
      style={{
        background: '#132e32',
        borderRadius: 16,
        border: '1px solid rgba(23, 96, 135, 0.3)',
        boxShadow: '0 20px 40px rgba(0, 0, 0, 0.4)',
      }}
      bodyStyle={{ padding: 20 }}
    >
      {/* Header controls: Breadcrumb & Search bar */}
      <Space direction="vertical" style={{ width: '100%' }} size="middle">
        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', flexWrap: 'wrap', gap: 12 }}>
          <Space>
            {data?.parent_path && (
              <Button
                icon={<ArrowUpOutlined />}
                onClick={() => fetchDirectory(data.parent_path!)}
                title="Go Up Directory"
              >
                Up
              </Button>
            )}
            <Button icon={<ReloadOutlined />} onClick={() => fetchDirectory(currentPath)}>
              Refresh
            </Button>
          </Space>

          <Input
            prefix={<SearchOutlined style={{ color: '#53a2be' }} />}
            placeholder="Search media files..."
            allowClear
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            style={{ width: 260, borderRadius: 8, background: '#0a2239', borderColor: '#176087' }}
          />
        </div>

        {/* Current Path Breadcrumbs */}
        <Breadcrumb items={breadcrumbItems} style={{ background: '#0a2239', padding: '8px 14px', borderRadius: 8, fontSize: 13, border: '1px solid rgba(23, 96, 135, 0.3)' }} />

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
