import React, { useState } from 'react';
import { Alert, Button, Modal, Space, Typography } from 'antd';
import { LockOutlined, UnlockOutlined, SwapOutlined, WarningOutlined } from '@ant-design/icons';
import type { PlayerState } from './RemoteControlTab';

interface Props {
  state: PlayerState;
  sendCmd: (command: string, payload?: Record<string, unknown>) => Promise<boolean>;
  controls?: boolean;
}

/** PCController owns the reservation; neither surface invents local ownership. */
export const PublishingAuthority: React.FC<Props> = ({ state, sendCmd, controls = false }) => {
  const [dismissed, setDismissed] = useState<number | null>(null);
  const sync = state.hardware_sync;
  const authority = sync?.authority;
  const actor = sync?.authority_client_id;
  if (!authority || !actor) return null;
  const owner = authority.owner_id === actor;
  const pending = authority.pending.find(claim => claim.client_id === actor);
  const request = authority.pending[0];
  const conflict = Boolean(authority.owner_id && !owner);
  const paused = state.playing === false;
  const change = (operation: string, requester_id = '') => sendCmd('pealayer.hardware.authority', { operation, requester_id });
  if (controls) return <Alert type={conflict ? 'warning' : 'info'} showIcon icon={<SwapOutlined />}
    message={authority.owner_label || 'No hardware publisher'}
    description={<Space wrap>
      {owner ? <>
        <Button icon={authority.exclusive ? <UnlockOutlined /> : <LockOutlined />}
          onClick={() => change(authority.exclusive ? 'unlock' : 'lock')}>
          {authority.exclusive ? 'Unlock production' : 'Lock production'}</Button>
        <Button disabled={!paused} onClick={() => change('release')}>Release authority</Button>
      </> : authority.exclusive ? <Typography.Text type="secondary">Production locked · monitoring only</Typography.Text>
        : <Button icon={<SwapOutlined />} disabled={Boolean(pending)} onClick={() => change('request')}>
          {pending ? 'Handoff requested' : 'Request authority'}</Button>}
    </Space>} />;
  const visible = (conflict || (owner && Boolean(request))) && dismissed !== authority.revision;
  return <Modal open={visible} title={<Space><WarningOutlined />Publishing authority</Space>}
    width={480} onCancel={() => setDismissed(authority.revision)} footer={owner && request ? <Space wrap>
      <Button onClick={() => change('reject', request.client_id)}>Decline</Button>
      <Button type="primary" disabled={!paused || authority.exclusive}
        onClick={() => change('accept', request.client_id)}>Accept handoff</Button>
    </Space> : <Space wrap>
      <Button onClick={() => setDismissed(authority.revision)}>Monitor</Button>
      {!authority.exclusive && <Button type="primary" disabled={Boolean(pending)} onClick={() => change('request')}>
        {pending ? 'Handoff requested' : 'Request handoff'}</Button>}
    </Space>}>
    {owner && request ? <>
      <p>{request.label} requests control of the media clock and hardware timeline.</p>
      {!paused && <p>Pause playback before accepting the handoff.</p>}
    </> : <>
      <p>{authority.owner_label} owns hardware playback. This instance is not publishing a competing timeline.</p>
      {authority.exclusive && <p>Production is locked. Hardware controls are read-only; E-STOP remains available.</p>}
    </>}
  </Modal>;
};
