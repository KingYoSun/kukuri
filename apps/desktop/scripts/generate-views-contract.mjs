// IPC contract fixture から viewsContract.ts(型検査用 literal)を再生成する。
// 実行: node scripts/generate-views-contract.mjs
//
// 役割: src/lib/api/__fixtures__/views/*.json(Rust 側 views_wire_snapshot.rs が
// 生成・検証する共有 golden)を `satisfies <View型>` の object literal として
// 書き出す。tsc の excess/missing property check が types.ts との整合を強制し、
// viewsContract.test.ts が literal と JSON の同一性を検証する。
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const root = path.dirname(fileURLToPath(import.meta.url));
const fixtureDir = path.join(root, '../src/lib/api/__fixtures__/views');
const outPath = path.join(root, '../src/lib/api/__fixtures__/viewsContract.ts');

// fixture ファイル名 → [export 名, TS 型名]
const CONTRACTS = [
  ['post_view.full.json', 'postViewFull', 'PostView'],
  ['post_view.minimal.json', 'postViewMinimal', 'PostView'],
  ['timeline_view.json', 'timelineView', 'TimelineView'],
  ['bookmarked_post_view.json', 'bookmarkedPostView', 'BookmarkedPostView'],
  ['sync_status.full.json', 'syncStatusFull', 'SyncStatus'],
  ['sync_status.minimal.json', 'syncStatusMinimal', 'SyncStatus'],
  ['notification_view.json', 'notificationView', 'NotificationView'],
  ['joined_private_channel_view.json', 'joinedPrivateChannelView', 'JoinedPrivateChannelView'],
  ['dm_conversation_view.json', 'dmConversationView', 'DirectMessageConversationView'],
  ['dm_message_view.json', 'dmMessageView', 'DirectMessageMessageView'],
  ['dm_timeline_view.json', 'dmTimelineView', 'DirectMessageTimelineView'],
  ['game_room_view.score.json', 'gameRoomViewScore', 'GameRoomView'],
  ['game_room_view.metaverse.json', 'gameRoomViewMetaverse', 'GameRoomView'],
  ['author_social_view.json', 'authorSocialView', 'AuthorSocialView'],
  ['channel_access_token_export.json', 'channelAccessTokenExport', 'ChannelAccessTokenExport'],
  ['channel_access_token_preview.json', 'channelAccessTokenPreview', 'ChannelAccessTokenPreview'],
];

const types = [...new Set(CONTRACTS.map(([, , type]) => type))].sort();
const blocks = CONTRACTS.map(([file, name, type]) => {
  const json = fs.readFileSync(path.join(fixtureDir, file), 'utf8').trim();
  return `// ${file}\nexport const ${name} = ${json} satisfies ${type};`;
});

const header = [
  '// 自動生成ファイル — 手で編集しない。',
  '// 再生成: apps/desktop で `node scripts/generate-views-contract.mjs`',
  '// (Rust 側 fixture の再生成は `UPDATE_FIXTURES=1 cargo test -p kukuri-app-api views_wire`)。',
  '//',
  '// IPC 契約の TS 側検出装置(WP-S4 T2):',
  '// - literal の `satisfies` により、Rust が出力するフィールドが types.ts に無い場合',
  '//   (excess property)や、TS 必須フィールドが fixture に無い場合(missing)に tsc が fail する。',
  '// - viewsContract.test.ts が literal と JSON fixture の同一性を検証する',
  '//   (どちらか片方だけの編集を検出)。',
  '// - fail した場合は types.ts / views.rs の IPC 互換影響を評価すること。',
  'import type {',
  ...types.map((type) => `  ${type},`),
  "} from '../types';",
  '',
].join('\n');

fs.writeFileSync(outPath, `${header}\n${blocks.join('\n\n')}\n`);
console.log(`generated ${path.relative(process.cwd(), outPath)} (${CONTRACTS.length} literals)`);
