// IPC contract fixture の同期検証(WP-S4 T2)。
//
// viewsContract.ts の literal(tsc が types.ts と突き合わせる)と、Rust 側
// views_wire_snapshot.rs が検証する JSON fixture が同一値であることを固定する。
// fail した場合はどちらか片方だけが更新されている —
// `node scripts/generate-views-contract.mjs` で literal を再生成すること。
import fs from 'node:fs';
import path from 'node:path';

import { describe, expect, test } from 'vitest';

import * as contract from './viewsContract';

const FIXTURES: Array<[string, keyof typeof contract]> = [
  ['post_view.full.json', 'postViewFull'],
  ['post_view.minimal.json', 'postViewMinimal'],
  ['timeline_view.json', 'timelineView'],
  ['bookmarked_post_view.json', 'bookmarkedPostView'],
  ['sync_status.full.json', 'syncStatusFull'],
  ['sync_status.minimal.json', 'syncStatusMinimal'],
  ['notification_view.json', 'notificationView'],
  ['joined_private_channel_view.json', 'joinedPrivateChannelView'],
  ['dm_conversation_view.json', 'dmConversationView'],
  ['dm_message_view.json', 'dmMessageView'],
  ['dm_timeline_view.json', 'dmTimelineView'],
  ['game_room_view.score.json', 'gameRoomViewScore'],
  ['game_room_view.metaverse.json', 'gameRoomViewMetaverse'],
  ['author_social_view.json', 'authorSocialView'],
  ['channel_access_token_export.json', 'channelAccessTokenExport'],
  ['channel_access_token_preview.json', 'channelAccessTokenPreview'],
];

const fixtureDir = path.join(import.meta.dirname, 'views');

test('every committed fixture has a typed literal (and vice versa)', () => {
  const files = fs.readdirSync(fixtureDir).filter((name) => name.endsWith('.json'));
  expect(files.sort()).toEqual(FIXTURES.map(([file]) => file).sort());
});

describe.each(FIXTURES)('%s', (file, exportName) => {
  test('matches the typed literal in viewsContract.ts', () => {
    const json = JSON.parse(fs.readFileSync(path.join(fixtureDir, file), 'utf8'));
    expect(contract[exportName]).toEqual(json);
  });
});
