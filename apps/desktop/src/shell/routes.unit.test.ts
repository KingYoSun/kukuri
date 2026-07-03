/**
 * WP-S5: routes.ts 関数単位の characterization テスト。
 *
 * 後続 WP-H6(shell の store スライス化・selector 化・prop drilling 除去)の
 * 安全網として「観測した現挙動」をそのまま固定する。このテストが落ちた場合に
 * 疑うべきは直近の変更でありテストではない。期待値は生リテラルで書き、
 * 実装側の定数・関数から期待値を生成しない。
 *
 * URL 復元シナリオ(App 統合)は既存 routes.test.tsx に任せ、ここでは
 * 関数入出力のみを固定する。
 */
import { beforeEach, describe, expect, test } from 'vitest';

import {
  buildShellUrl,
  isProfileConnectionsView,
  isSettingsSection,
  parseHashRouteLocation,
  parseLegacyRequestedChannel,
  parsePrimarySectionPath,
  type BuildShellUrlOptions,
} from './routes';

// 各テストを独立させる(setup.ts は hash を掃除しない)。
beforeEach(() => {
  window.history.replaceState(null, '', '/');
});

describe('isSettingsSection', () => {
  test.each([
    'about',
    'appearance',
    'connectivity',
    'discovery',
    'community-node',
    'reactions',
    'release',
  ])('returns true for "%s"', (value) => {
    expect(isSettingsSection(value)).toBe(true);
  });

  test.each([null, '', 'settings', 'About', 'community_node'])(
    'returns false for %j',
    (value) => {
      expect(isSettingsSection(value)).toBe(false);
    }
  );
});

describe('isProfileConnectionsView', () => {
  test.each(['following', 'followed', 'muted'])('returns true for "%s"', (value) => {
    expect(isProfileConnectionsView(value)).toBe(true);
  });

  test.each([null, '', 'follower', 'Following'])('returns false for %j', (value) => {
    expect(isProfileConnectionsView(value)).toBe(false);
  });
});

describe('parsePrimarySectionPath', () => {
  const cases: Array<{ name: string; pathname: string; expected: string | null }> = [
    { name: 'normalizes "/" to the timeline section', pathname: '/', expected: 'timeline' },
    { name: 'resolves /timeline', pathname: '/timeline', expected: 'timeline' },
    { name: 'resolves /live', pathname: '/live', expected: 'live' },
    { name: 'resolves /game', pathname: '/game', expected: 'game' },
    { name: 'resolves /messages', pathname: '/messages', expected: 'messages' },
    { name: 'resolves /profile', pathname: '/profile', expected: 'profile' },
    { name: 'resolves /notifications', pathname: '/notifications', expected: 'notifications' },
    { name: 'returns null for the excluded /channels path', pathname: '/channels', expected: null },
    { name: 'returns null for an unknown path', pathname: '/unknown', expected: null },
    { name: 'does not normalize a trailing slash', pathname: '/timeline/', expected: null },
    { name: 'returns null without a leading slash', pathname: 'timeline', expected: null },
  ];

  test.each(cases)('$name', ({ pathname, expected }) => {
    expect(parsePrimarySectionPath(pathname)).toBe(expected);
  });
});

describe('parseHashRouteLocation', () => {
  const cases: Array<{
    name: string;
    hash: string;
    expected: { pathname: string; search: string };
  }> = [
    {
      name: 'returns the root location for an empty hash',
      hash: '',
      expected: { pathname: '/', search: '' },
    },
    {
      name: 'returns the root location for a bare "#"',
      hash: '#',
      expected: { pathname: '/', search: '' },
    },
    {
      name: 'parses a hash with a pathname only',
      hash: '#/timeline',
      expected: { pathname: '/timeline', search: '' },
    },
    {
      name: 'splits pathname and query at the first "?"',
      hash: '#/timeline?topic=kukuri%3Atopic%3Ademo&settings=about',
      expected: {
        pathname: '/timeline',
        search: '?topic=kukuri%3Atopic%3Ademo&settings=about',
      },
    },
    {
      name: 'accepts a hash without the leading "#"',
      hash: '/messages?peerPubkey=peer-1',
      expected: { pathname: '/messages', search: '?peerPubkey=peer-1' },
    },
    {
      name: 'falls back to the root pathname when the hash starts with a query',
      hash: '#?settings=about',
      expected: { pathname: '/', search: '?settings=about' },
    },
    {
      name: 'preserves a trailing empty query marker',
      hash: '#/messages?',
      expected: { pathname: '/messages', search: '?' },
    },
  ];

  test.each(cases)('$name', ({ hash, expected }) => {
    expect(parseHashRouteLocation(hash)).toEqual(expected);
  });
});

describe('parseLegacyRequestedChannel', () => {
  // 引数順は (timelineScope, composeTarget)。compose target 側が優先される。
  const cases: Array<{
    name: string;
    scope: string | null;
    compose: string | null;
    expected: string | null;
  }> = [
    {
      name: 'returns null when both values are null',
      scope: null,
      compose: null,
      expected: null,
    },
    {
      name: 'prefers the compose target channel over the timeline scope channel',
      scope: 'channel:scope-channel',
      compose: 'channel:compose-channel',
      expected: 'compose-channel',
    },
    {
      name: 'reads the channel from the timeline scope when the compose target is null',
      scope: 'channel:scope-channel',
      compose: null,
      expected: 'scope-channel',
    },
    {
      name: 'reads the channel from the compose target when the timeline scope is null',
      scope: null,
      compose: 'channel:compose-channel',
      expected: 'compose-channel',
    },
    {
      name: 'falls through a non-channel compose target to the scope channel',
      scope: 'channel:scope-channel',
      compose: 'following',
      expected: 'scope-channel',
    },
    {
      name: 'returns null when neither value has a channel prefix',
      scope: 'global',
      compose: 'home',
      expected: null,
    },
    {
      // 現挙動の固定: prefix のみの値は空文字列の channel id になる。
      name: 'returns an empty string for a bare channel prefix',
      scope: 'channel:',
      compose: null,
      expected: '',
    },
  ];

  test.each(cases)('$name', ({ scope, compose, expected }) => {
    expect(parseLegacyRequestedChannel(scope, compose)).toBe(expected);
  });
});

describe('buildShellUrl', () => {
  // 全フィールド必須のため fixture でデフォルトを埋め、期待 URL は生リテラルで固定する。
  function buildOptions(overrides: Partial<BuildShellUrlOptions> = {}): BuildShellUrlOptions {
    return {
      activeTopic: 'kukuri:topic:demo',
      primarySection: 'timeline',
      timelineView: 'feed',
      profileMode: 'overview',
      profileConnectionsView: 'following',
      selectedThread: null,
      focusedObjectId: null,
      selectedAuthorPubkey: null,
      selectedDirectMessagePeerPubkey: null,
      settingsOpen: false,
      settingsSection: 'connectivity',
      selectedChannelId: null,
      selectedLiveSessionId: null,
      selectedGameRoomId: null,
      ...overrides,
    };
  }

  const cases: Array<{
    name: string;
    overrides: Partial<BuildShellUrlOptions>;
    expected: string;
  }> = [
    {
      name: 'builds the timeline url with only the encoded topic by default',
      overrides: {},
      expected: '/timeline?topic=kukuri%3Atopic%3Ademo',
    },
    {
      name: 'includes the channel on the timeline feed view',
      overrides: { selectedChannelId: 'channel-1' },
      expected: '/timeline?topic=kukuri%3Atopic%3Ademo&channel=channel-1',
    },
    {
      name: 'drops the channel and adds timelineView on the bookmarks view',
      overrides: { timelineView: 'bookmarks', selectedChannelId: 'channel-1' },
      expected: '/timeline?topic=kukuri%3Atopic%3Ademo&timelineView=bookmarks',
    },
    {
      name: 'includes the channel on non-timeline sections such as game',
      overrides: { primarySection: 'game', selectedChannelId: 'channel-1' },
      expected: '/game?topic=kukuri%3Atopic%3Ademo&channel=channel-1',
    },
    {
      name: 'drops the channel and emits peer and author pubkeys on messages',
      overrides: {
        primarySection: 'messages',
        selectedChannelId: 'channel-1',
        selectedDirectMessagePeerPubkey: 'peer-1',
        selectedAuthorPubkey: 'author-1',
      },
      expected: '/messages?topic=kukuri%3Atopic%3Ademo&peerPubkey=peer-1&authorPubkey=author-1',
    },
    {
      name: 'suppresses the thread context on messages',
      overrides: { primarySection: 'messages', selectedThread: 'thread-1' },
      expected: '/messages?topic=kukuri%3Atopic%3Ademo',
    },
    {
      name: 'drops channel, thread, author, and peer on notifications',
      overrides: {
        primarySection: 'notifications',
        selectedChannelId: 'channel-1',
        selectedThread: 'thread-1',
        selectedAuthorPubkey: 'author-1',
        selectedDirectMessagePeerPubkey: 'peer-1',
      },
      expected: '/notifications?topic=kukuri%3Atopic%3Ademo',
    },
    {
      name: 'emits the thread context with focus and author on the timeline',
      overrides: {
        selectedThread: 'thread-1',
        focusedObjectId: 'focus-1',
        selectedAuthorPubkey: 'author-1',
      },
      expected:
        '/timeline?topic=kukuri%3Atopic%3Ademo&context=thread&threadId=thread-1&focusObjectId=focus-1&authorPubkey=author-1',
    },
    {
      name: 'emits the author context when only an author is selected',
      overrides: { selectedAuthorPubkey: 'author-1' },
      expected: '/timeline?topic=kukuri%3Atopic%3Ademo&context=author&authorPubkey=author-1',
    },
    {
      name: 'combines channel and thread context on the timeline feed view',
      overrides: { selectedChannelId: 'channel-1', selectedThread: 'thread-1' },
      expected:
        '/timeline?topic=kukuri%3Atopic%3Ademo&channel=channel-1&context=thread&threadId=thread-1',
    },
    {
      name: 'emits profileMode=edit on the profile section',
      overrides: { primarySection: 'profile', profileMode: 'edit' },
      expected: '/profile?topic=kukuri%3Atopic%3Ademo&profileMode=edit',
    },
    {
      name: 'emits profileMode=connections with the connections view on the profile section',
      overrides: {
        primarySection: 'profile',
        profileMode: 'connections',
        profileConnectionsView: 'muted',
      },
      expected: '/profile?topic=kukuri%3Atopic%3Ademo&profileMode=connections&connectionsView=muted',
    },
    {
      name: 'omits profileMode for the profile overview mode',
      overrides: { primarySection: 'profile' },
      expected: '/profile?topic=kukuri%3Atopic%3Ademo',
    },
    {
      name: 'omits profileMode outside the profile section',
      overrides: { primarySection: 'game', profileMode: 'edit' },
      expected: '/game?topic=kukuri%3Atopic%3Ademo',
    },
    {
      name: 'emits the settings section while the settings drawer is open',
      overrides: { settingsOpen: true, settingsSection: 'appearance' },
      expected: '/timeline?topic=kukuri%3Atopic%3Ademo&settings=appearance',
    },
    {
      name: 'omits the settings section while the settings drawer is closed',
      overrides: { settingsOpen: false, settingsSection: 'appearance' },
      expected: '/timeline?topic=kukuri%3Atopic%3Ademo',
    },
    {
      name: 'emits sessionId only on the live section',
      overrides: { primarySection: 'live', selectedLiveSessionId: 'session-1' },
      expected: '/live?topic=kukuri%3Atopic%3Ademo&sessionId=session-1',
    },
    {
      name: 'omits roomId on the live section',
      overrides: { primarySection: 'live', selectedGameRoomId: 'room-1' },
      expected: '/live?topic=kukuri%3Atopic%3Ademo',
    },
    {
      name: 'emits roomId only on the game section',
      overrides: { primarySection: 'game', selectedGameRoomId: 'room-1' },
      expected: '/game?topic=kukuri%3Atopic%3Ademo&roomId=room-1',
    },
    {
      name: 'omits sessionId on the game section',
      overrides: { primarySection: 'game', selectedLiveSessionId: 'session-1' },
      expected: '/game?topic=kukuri%3Atopic%3Ademo',
    },
  ];

  test.each(cases)('$name', ({ overrides, expected }) => {
    expect(buildShellUrl(buildOptions(overrides))).toBe(expected);
  });
});
