import type { P2PMessage } from '@/stores/p2pStore';
import type { NostrEventPayload } from '@/types/nostr';

export const TIMELINE_REALTIME_DELTA_EVENT = 'timeline:realtime-delta';

interface BaseRealtimeDelta {
  receivedAt: number;
}

export interface NostrTimelineRealtimeDelta extends BaseRealtimeDelta {
  source: 'nostr';
  payload: NostrEventPayload;
}

export interface P2PTimelineRealtimeDelta extends BaseRealtimeDelta {
  source: 'p2p';
  topicId: string;
  message: P2PMessage;
}

export type TimelineRealtimeDelta = NostrTimelineRealtimeDelta | P2PTimelineRealtimeDelta;

export const dispatchTimelineRealtimeDelta = (
  delta:
    | Omit<NostrTimelineRealtimeDelta, 'receivedAt'>
    | Omit<P2PTimelineRealtimeDelta, 'receivedAt'>,
): void => {
  if (typeof window === 'undefined') {
    return;
  }

  const payload: TimelineRealtimeDelta = {
    ...delta,
    receivedAt: Date.now(),
  };

  window.dispatchEvent(
    new CustomEvent<TimelineRealtimeDelta>(TIMELINE_REALTIME_DELTA_EVENT, {
      detail: payload,
    }),
  );
};
