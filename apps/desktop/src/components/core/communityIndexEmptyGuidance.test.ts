import { describe, expect, test } from 'vitest';

import { communityIndexEmptyGuidance, communityIndexQueryAsPubkey } from './communityIndexEmptyGuidance';

const PUBKEY = 'ab'.repeat(32);

describe('communityIndexQueryAsPubkey', () => {
  test.each([
    [PUBKEY, PUBKEY],
    [`  ${PUBKEY.toUpperCase()}  `, PUBKEY],
    ['CliPeerA', null],
    [PUBKEY.slice(0, 63), null],
    [`${PUBKEY}0`, null],
    ['zz'.repeat(32), null],
    ['', null],
  ])('%s -> %s', (query, expected) => {
    expect(communityIndexQueryAsPubkey(query)).toBe(expected);
  });
});

describe('communityIndexEmptyGuidance', () => {
  const base = {
    operation: 'search' as const,
    query: 'CliPeerA',
    activeTopic: 'rust',
    activeTimelineScope: { kind: 'public' as const },
    canRequestIndexing: true,
  };

  test('topic search explains text matching and offers a public topic indexing request', () => {
    expect(communityIndexEmptyGuidance({ ...base, mode: 'topic' })).toEqual({
      scope: 'topic',
      operation: 'search',
      explainsPostTextMatching: true,
      reasons: ['notIndexedYet', 'outsideNodeScope'],
      actions: ['retry', 'openTimeline', 'requestIndexing', 'openCommunityNodeSettings', 'openConnectivity'],
      authorPubkey: null,
      indexingTarget: { kind: 'public_topic', topicId: 'rust' },
      explainsTopicListRequest: false,
      indexStatus: null,
    });
  });

  // #975: 索引状況の応答で理由と申請 CTA が確定する。
  describe('index status', () => {
    const target = (supported: boolean) => ({ scope_kind: 'public_topic' as const, scope_id: 'rust', supported });
    const request = (status: 'pending' | 'approved' | 'rejected') => ({
      request_id: `r-${status}`,
      scope_kind: 'public_topic' as const,
      target_id: 'rust',
      status,
      created_at: 1,
      decided_at: status === 'pending' ? null : 2,
    });

    test.each([
      ['loading', { kind: 'loading' as const }, { kind: 'loading' }, ['notIndexedYet', 'outsideNodeScope'], true],
      ['unknown', { kind: 'unknown' as const }, { kind: 'unknown' }, ['notIndexedYet', 'outsideNodeScope'], true],
      ['supported', { kind: 'known' as const, response: { requests: [], target: target(true) } }, { kind: 'target', state: 'supported' }, ['notIndexedYet'], false],
      ['not supported', { kind: 'known' as const, response: { requests: [], target: target(false) } }, { kind: 'target', state: 'notSupported' }, ['notSupportedTarget'], true],
      ['pending', { kind: 'known' as const, response: { requests: [request('pending')], target: target(false) } }, { kind: 'target', state: 'pending' }, ['requestPending'], false],
      ['approved', { kind: 'known' as const, response: { requests: [request('approved')], target: target(true) } }, { kind: 'target', state: 'approved' }, ['notIndexedYet'], false],
      ['rejected', { kind: 'known' as const, response: { requests: [request('rejected')], target: target(false) } }, { kind: 'target', state: 'rejected' }, ['requestRejected'], false],
      ['target missing', { kind: 'known' as const, response: { requests: [], target: null } }, { kind: 'unknown' }, ['notIndexedYet', 'outsideNodeScope'], true],
    ])('topic %s', (_name, indexStatus, expected, reasons, offersRequest) => {
      const guidance = communityIndexEmptyGuidance({ ...base, mode: 'topic', indexStatus });
      expect(guidance.indexStatus).toEqual(expected);
      expect(guidance.reasons).toEqual(reasons);
      expect(guidance.actions.includes('requestIndexing')).toBe(offersRequest);
      expect(guidance.indexingTarget !== null).toBe(offersRequest);
    });

    test('a pubkey query keeps the user hint in front of the definite reason', () => {
      const guidance = communityIndexEmptyGuidance({
        ...base,
        mode: 'topic',
        query: PUBKEY,
        indexStatus: { kind: 'known', response: { requests: [], target: target(false) } },
      });
      expect(guidance.reasons).toEqual(['pubkeyQuery', 'notSupportedTarget']);
    });

    test('a private channel without an own request asks to verify from the request dialog', () => {
      const channel = { ...base, mode: 'topic' as const, activeTimelineScope: { kind: 'channel' as const, channel_id: 'channel-1' } };
      expect(
        communityIndexEmptyGuidance({ ...channel, indexStatus: { kind: 'known', response: { requests: [], target: null } } }).indexStatus
      ).toEqual({ kind: 'privateUnverified' });
      const pending = communityIndexEmptyGuidance({
        ...channel,
        indexStatus: {
          kind: 'known',
          response: {
            requests: [{ ...request('pending'), scope_kind: 'private_channel', target_id: 'channel-1' }],
            target: null,
          },
        },
      });
      expect(pending.indexStatus).toEqual({ kind: 'target', state: 'pending' });
      expect(pending.actions.includes('requestIndexing')).toBe(false);
    });

    test('explore groups own requests by status and never asserts a per-topic state', () => {
      const guidance = communityIndexEmptyGuidance({
        ...base,
        mode: 'explore',
        indexStatus: {
          kind: 'known',
          response: {
            requests: [
              { ...request('approved'), target_id: 'kukuri:topic:rust' },
              { ...request('pending'), target_id: 'kukuri:topic:golang' },
              { ...request('rejected'), scope_kind: 'private_channel', target_id: 'channel-9' },
            ],
            target: null,
          },
        },
      });
      expect(guidance.indexStatus).toEqual({
        kind: 'ownRequests',
        approved: ['kukuri:topic:rust'],
        pending: ['kukuri:topic:golang'],
        rejected: ['channel-9'],
      });
      expect(guidance.reasons).toEqual(['notIndexedYet', 'outsideNodeScope']);
      expect(guidance.indexingTarget).toBeNull();
    });
  });

  test('private channel search targets the channel with its label', () => {
    const guidance = communityIndexEmptyGuidance({
      ...base,
      mode: 'topic',
      activeTimelineScope: { kind: 'channel', channel_id: 'channel-1' },
      activeChannelLabel: 'Design room',
    });
    expect(guidance.scope).toBe('channel');
    expect(guidance.indexingTarget).toEqual({
      kind: 'private_channel',
      topicId: 'rust',
      channelId: 'channel-1',
      channelLabel: 'Design room',
    });
  });

  test('a missing channel label falls back to the channel id instead of an empty label', () => {
    const guidance = communityIndexEmptyGuidance({
      ...base,
      mode: 'topic',
      activeTimelineScope: { kind: 'channel', channel_id: 'channel-1' },
      activeChannelLabel: '  ',
    });
    expect(guidance.indexingTarget).toMatchObject({ channelLabel: 'channel-1' });
  });

  test('explore search never targets one topic and points to the topic list instead', () => {
    const guidance = communityIndexEmptyGuidance({ ...base, mode: 'explore' });
    expect(guidance.scope).toBe('explore');
    expect(guidance.indexingTarget).toBeNull();
    expect(guidance.actions).not.toContain('requestIndexing');
    expect(guidance.explainsTopicListRequest).toBe(true);
  });

  test('without an indexing entry point neither the CTA nor the topic list hint is offered', () => {
    const topic = communityIndexEmptyGuidance({ ...base, mode: 'topic', canRequestIndexing: false });
    const explore = communityIndexEmptyGuidance({ ...base, mode: 'explore', canRequestIndexing: false });
    expect(topic.indexingTarget).toBeNull();
    expect(topic.actions).not.toContain('requestIndexing');
    expect(explore.explainsTopicListRequest).toBe(false);
  });

  test('a user ID query leads with opening the user and drops the text matching rule', () => {
    const guidance = communityIndexEmptyGuidance({ ...base, mode: 'explore', query: ` ${PUBKEY} ` });
    expect(guidance.authorPubkey).toBe(PUBKEY);
    expect(guidance.explainsPostTextMatching).toBe(false);
    expect(guidance.reasons[0]).toBe('pubkeyQuery');
    expect(guidance.actions[0]).toBe('openAuthor');
  });

  test.each(['discovery', 'recommendations'] as const)(
    '%s keeps the scope reasons without text matching or user lookup',
    (operation) => {
      const guidance = communityIndexEmptyGuidance({
        ...base,
        mode: 'explore',
        operation,
        query: PUBKEY,
      });
      expect(guidance.operation).toBe(operation);
      expect(guidance.explainsPostTextMatching).toBe(false);
      expect(guidance.authorPubkey).toBeNull();
      expect(guidance.reasons).toEqual(['notIndexedYet', 'outsideNodeScope']);
      expect(guidance.actions).toEqual([
        'retry',
        'openTimeline',
        'openCommunityNodeSettings',
        'openConnectivity',
      ]);
    }
  );

  test('a blank active topic never produces an indexing target', () => {
    const guidance = communityIndexEmptyGuidance({ ...base, mode: 'topic', activeTopic: '  ' });
    expect(guidance.indexingTarget).toBeNull();
  });
});
