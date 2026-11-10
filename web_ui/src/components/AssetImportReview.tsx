import { Alert, Button, Modal, Space, Typography } from 'antd';
import { DeleteOutlined, FileAddOutlined, SaveOutlined } from '@ant-design/icons';
import {tr,type UiLocale} from '../i18n';
import type { AssetImportState, ImportedEffect } from '../assetImports';
import { formatTimelineTime } from '../timelineTime';

export function PendingTimelineChange({open,sendCmd,locale}:{open:boolean;locale:UiLocale;sendCmd:(command:string,payload?:Record<string,unknown>)=>Promise<boolean>}) {
  const decide=(decision:'save'|'discard'|'cancel')=>void sendCmd('resolve_timeline_close',{decision});
  return <Modal open={open} title={tr(locale,'Unsaved timeline cues')} onCancel={()=>decide('cancel')}
    footer={<Space wrap><Button onClick={()=>decide('cancel')}>{tr(locale,'Cancel')}</Button>
      <Button danger icon={<DeleteOutlined/>} onClick={()=>decide('discard')}>{tr(locale,'Discard')}</Button>
      <Button type="primary" icon={<SaveOutlined/>} onClick={()=>decide('save')}>{tr(locale,'Save')}</Button></Space>}>
      {tr(locale,'Save cues to timeline sidecar before proceeding?')}
    </Modal>;
}

export function AssetImportReview({ state, sendCmd, openEffect }: {
  state?: AssetImportState;
  sendCmd: (command: string, payload?: Record<string, unknown>) => Promise<boolean>;
  openEffect: (effect: ImportedEffect) => void;
}) {
  const review = state?.reviews?.[0];
  const time = review?.destination.surface === 'timeline' ? review.destination.start_time_ms / 1000 : 0;
  return <Modal open={Boolean(review)} title={<Space><FileAddOutlined />Import asset</Space>}
    okText={review?.kind === 'effect' ? 'Open properties' : 'Add to timeline'}
    onCancel={() => { if (review) void sendCmd('review_asset', { id: review.id, accept: false }); }}
    onOk={() => {
      if (!review) return;
      if (review.kind === 'effect' && review.effect) {
        openEffect(review.effect);
        void sendCmd('review_asset', { id: review.id, accept: false });
      } else void sendCmd('review_asset', { id: review.id, accept: true });
    }}>
    <Typography.Paragraph strong>{review?.name}</Typography.Paragraph>
    <Typography.Paragraph>{review?.kind === 'effect'
      ? 'Review this definition in Effect properties, then Save or Publish. Drag the saved effect from the library to reuse it. Hardware effects remain owned by PCController.'
      : `Append at ${formatTimelineTime(time,true)} without replacing existing cues, moving the playhead or changing your workspace.`}</Typography.Paragraph>
    <div className="asset-review-items">{review?.summary.map((line,index)=><div key={index}>{line}</div>)}</div>
    <Alert type="info" showIcon message={review?.kind === 'scene' ? 'Sampled relay/PWM values become set-and-keep cues. Unknown channels are not invented. No live scene is run.' : 'Pause playback before arranging timeline assets. Existing channel locks are respected.'} />
  </Modal>;
}
