import { expect, test } from 'vitest';

import {
  createInitialWorkspaceState,
  setTimelineColumnTopic,
} from '@/shell/slices/workspace';
import { workspaceForRoute } from './routeWorkspaceProjection';

test('route projection reuses an active Timeline whose scope was replaced in place', () => {
  const initial = createInitialWorkspaceState();
  const timelineId = initial.activeColumnId;
  const switched = setTimelineColumnTopic(initial, timelineId, 'kukuri:topic:dev');

  const projected = workspaceForRoute(switched, {
    gameColumnResolutionPending: false,
    isScoreGameRoom: false,
    nextTimelineView: 'feed',
    routeScope: { topicId: 'kukuri:topic:dev', channelId: null },
    routeSection: 'timeline',
    selectedAuthorPubkey: null,
    selectedDirectMessagePeerPubkey: null,
    selectedGameRoomId: null,
    selectedLiveSessionId: null,
    selectedThread: null,
  });

  expect(projected.columns).toHaveLength(initial.columns.length);
  expect(projected.activeColumnId).toBe(timelineId);
  expect(projected.columns.find((column) => column.id === timelineId)?.scope).toEqual({
    topicId: 'kukuri:topic:dev',
    channelId: null,
  });
});
