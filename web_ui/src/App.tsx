import React from 'react';
import { ConfigProvider, theme } from 'antd';
import { RemoteControl } from './components/RemoteControl';

const App: React.FC = () => {
  return (
    <ConfigProvider
      theme={{
        algorithm: theme.darkAlgorithm,
        token: {
          colorPrimary: '#e11d48',
          colorBgContainer: '#151922',
          colorBgBase: '#0b0c10',
          borderRadius: 12,
          fontFamily: `-apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, 'Helvetica Neue', Arial, sans-serif`,
        },
      }}
    >
      <RemoteControl />
    </ConfigProvider>
  );
};

export default App;
