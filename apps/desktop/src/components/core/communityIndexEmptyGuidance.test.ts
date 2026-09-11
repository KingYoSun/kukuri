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
