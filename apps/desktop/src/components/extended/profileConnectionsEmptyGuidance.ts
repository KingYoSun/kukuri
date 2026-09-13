// #994: フォロー中 / フォロワー / ミュート中 / ブロック中の 0 件を、始め方の手順・補足・次の行動へ変換する table。
// 表示専用であり、外部送信・認証・同意・永続化を伴う action は持たない(INVAR-1)。
import type { ProfileConnectionsView } from '@/components/shell/types';

/** 手順。`follow` / `mute` / `block` は実ボタンを chip で示し、`mute` は通報 icon 経由の導線も添える。 */
export type ConnectionsEmptyStep = 'openProfile' | 'follow' | 'mute' | 'block' | 'shareOwnId';
/** 補足。ミュート / ブロックの保存範囲は設定画面と同じ文言(同じ i18n key)を使う。 */
export type ConnectionsEmptyNote = 'followedInfo' | 'muteDeviceOnly' | 'blockSigned';
export type ConnectionsEmptyAction = 'openTimeline' | 'openExplore' | 'copyOwnId';

export type ConnectionsEmptyGuidance = {
  steps: ConnectionsEmptyStep[];
  notes: ConnectionsEmptyNote[];
  actions: ConnectionsEmptyAction[];
};

const GUIDANCE: Record<ProfileConnectionsView, ConnectionsEmptyGuidance> = {
  following: {
    steps: ['openProfile', 'follow'],
    notes: [],
    actions: ['openTimeline', 'openExplore'],
  },
  // フォロワーは相手の操作で増える(ADR 0013: incoming edge は届いた follow 情報から端末内で導出する)。
  followed: {
    steps: ['shareOwnId'],
    notes: ['followedInfo'],
    actions: ['copyOwnId', 'openTimeline'],
  },
  muted: {
    steps: ['openProfile', 'mute'],
    notes: ['muteDeviceOnly'],
    actions: ['openTimeline'],
  },
  blocking: {
    steps: ['openProfile', 'block'],
    notes: ['blockSigned'],
    actions: ['openTimeline'],
  },
};

export function profileConnectionsEmptyGuidance(view: ProfileConnectionsView): ConnectionsEmptyGuidance {
  return GUIDANCE[view];
}
