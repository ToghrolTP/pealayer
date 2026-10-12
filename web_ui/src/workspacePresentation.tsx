import React from 'react';
import { AppstoreOutlined, BorderOutlined, DesktopOutlined, EditOutlined, LayoutOutlined, RadarChartOutlined,
  VideoCameraOutlined } from '@ant-design/icons';

// Shared presentation of the authoritative profile icon, never its caption/ID.
export const workspaceGlyph = (icon?: string): React.ReactNode => {
  switch (icon?.trim().toLowerCase()) {
    case 'timeline': return <EditOutlined />;
    case 'hardware': return <RadarChartOutlined />;
    case 'effects': case 'tabs': return <AppstoreOutlined />;
    case 'window': return <LayoutOutlined />;
    case 'video': return <VideoCameraOutlined />;
    case 'layout': return <BorderOutlined />;
    default: return <DesktopOutlined />;
  }
};
