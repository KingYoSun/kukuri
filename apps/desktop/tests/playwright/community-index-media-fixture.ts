import { type Page } from '@playwright/test';

import { columnIdentityId } from '../../src/shell/slices/workspace';
import { WORKSPACE_LAYOUT_STORAGE_KEY } from '../../src/shell/workspacePersistence';

// #1052: 「見つける」の解決済み投稿に添付画像を持たせるブラウザ用 fixture。
// 実装差分と無関係な seed / 操作をここへ集め、記録用の撮影 spec からも再利用する。

export const OBJECT_ID = 'explore-media-post';
const IMAGE_HASH = 'b'.repeat(64);
// #1055: 発行 node の manifest `node_id`(署名鍵の x-only 公開鍵 hex)。
const ADVISORY_ISSUER_NODE_ID = 'd'.repeat(64);
const AUTHOR_PUBKEY = 'a'.repeat(64);
const PNG_BASE64 = 'iVBORw0KGgoAAAANSUhEUgAAAGAAAABgCAYAAADimHc4AAABmklEQVR42u3E0YaQYQBF0f04SZIkSZIkSZIkSZIkSZIkI0lGkowkSZIkSUaSjCQZSTKSZCTJJ0l6jb/zFvvmXKzFipkxlYeVqTysSuVhdSoPa1J5WJvKw7pUHtan8rBhdkzlYWMqD5tSedicysOWVB62pvKwLZWH7ak87JgbU3nYmcrDrlQedqfysCeVh72pPOxL5WF/Kg8H7oypPBxM5eFQKg+HU3k4ksrD0VQejqXycDyVhxMPx1QeTqbycCqVh9OpPJxJ5eFsKg/nUnk4n8rDzPyYysOFVB4upvJwKZWHy6k8zKbycCWVh6upPFxbGFN5uJ7Kw1wqDzdSebiZysOtVB5up/JwJ5WHu4tjKg/3Unm4n8rDg1QeHqby8CiVh8epPDxJ5eHp0pjKw3wqD89SeXieysOLVB5epvKwkMrDq1QeXi+PqTy8SeXhbSoPi6k8vEvl4X0qDx9SefiYysPSGFN5+JTKw+dUHr6k8vA1lYflVB6+pfLwPZWHH//GVB5+pvIwUnn4lcrD71Qe/qTy8DeVh3+pPP8B+DNqc6Zh66kAAAAASUVORK5CYII=';
export const SHOT_PREFIX = process.env.KUKURI_1052_SHOT_PREFIX ?? 'after';
export const SHOT_DIR = '../../docs/ui-reviews/assets/1052';

export type SeedOptions = {
  locale: 'ja' | 'en';
  theme: 'dark' | 'light';
  adultLabeled?: boolean;
  /// #1055: 設定済み Community Node が発行した content advisory を index 応答へ載せる。
  advisoryLabeled?: boolean;
};

export async function seedExploreMedia(
  page: Page,
  { locale, theme, adultLabeled = false, advisoryLabeled = false }: SeedOptions
) {
  const scope = { topicId: 'kukuri:topic:general', channelId: null };
  const columns = (['timeline', 'explore'] as const).map((kind) => ({
    id: columnIdentityId(kind, scope),
    kind,
    scope,
    pinned: true,
    preferredDesktopSpan: 1,
  }));

  await page.addInitScript(
    ({
      locale,
      theme,
      adultLabeled,
      advisoryLabeled,
      columns,
      layoutKey,
      objectId,
      imageHash,
      authorPubkey,
      issuerNodeId,
      png,
    }) => {
      localStorage.setItem('kukuri.desktop.locale', locale);
      localStorage.setItem('kukuri.desktop.theme', theme);
      localStorage.setItem(
        layoutKey,
        JSON.stringify({ version: 1, activeColumnId: columns[1].id, columns })
      );

      let desktopApi = (window as unknown as { __KUKURI_DESKTOP__?: unknown }).__KUKURI_DESKTOP__;
      Object.defineProperty(window, '__KUKURI_DESKTOP__', {
        configurable: true,
        get: () => desktopApi,
        set: (api: Record<string, unknown>) => {
          desktopApi = api;
          if (!api) return;
          const attachment = {
            hash: imageHash,
            mime: 'image/png',
            bytes: 467,
            role: 'image_original',
            status: 'Available',
          };
          const post = {
            object_id: objectId,
            envelope_id: `envelope-${objectId}`,
            author_pubkey: authorPubkey,
            author_name: 'kukuri builder',
            author_display_name: 'kukuri builder',
            author_picture_asset: null,
            following: false,
            followed_by: false,
            mutual: false,
            friend_of_friend: false,
            provenance: null,
            withdrawal: null,
            content: 'clock man と kawaii gazou の添付つき投稿',
            content_status: 'Available',
            content_labels: adultLabeled ? ['adult'] : [],
            attachments: [attachment],
            created_at: 1_700_000_000,
            reply_to: null,
            reply_preview: null,
            root_id: objectId,
            object_kind: 'post',
            published_topic_id: 'kukuri:topic:general',
            origin_topic_id: 'kukuri:topic:general',
            repost_of: null,
            repost_commentary: null,
            is_threadable: true,
            channel_id: null,
            audience_label: 'Public',
            reaction_summary: [],
            my_reactions: [],
          };
          const entry = {
            scope_kind: 'public_topic',
            scope_id: 'kukuri:topic:general',
            object_id: objectId,
            author_pubkey: authorPubkey,
            text: 'indexed text',
            created_at: 1_700_000_000,
            content_advisories: advisoryLabeled
              ? [
                  {
                    issuer_node_id: issuerNodeId,
                    subject_kind: 'blob_cid',
                    subject_id: imageHash,
                    category: 'nsfw',
                    label: 'adult',
                    confidence: 84,
                    signal_id: 'signal-1',
                    basis: 'classifier_score',
                  },
                ]
              : [],
          };
          const queryResponse = async () => ({ entries: [entry] });
          api.searchCommunityNodeIndex = queryResponse;
          api.discoverCommunityNodeIndex = queryResponse;
          api.recommendCommunityNodeIndex = queryResponse;
          api.resolveCommunityIndexPosts = async (inputs: { key: string }[]) => ({
            entries: inputs.map((input) => ({
              key: input.key,
              post,
              capabilities: {
                open_thread: true,
                reply: true,
                repost: true,
                quote_repost: true,
                react: true,
                copy_link: true,
                bookmark: true,
                withdraw: false,
              },
            })),
          });
          api.getBlobMediaPayload = async (hash: string, mime: string) =>
            hash === imageHash ? { bytes_base64: png, mime } : null;
          if (advisoryLabeled) {
            api.fetchCommunityNodeManifest = async () => ({
              status: 'ok',
              manifest: {
                node_id: issuerNodeId,
                node_name: 'index.kukuri.example',
                node_role: 'default-onboarding-node',
                server_name: 'index.kukuri.example',
                manifest_version: 'v1',
                capability_scope: {
                  available_enabled: ['community_index', 'community_local_trust'],
                  planned_enabled: [],
                },
                authority_scope: {
                  applies_to: [
                    'communities_indexed_by_this_node',
                    'trust_signals_issued_by_this_node',
                  ],
                  does_not_apply_to: ['kukuri_network_as_a_whole'],
                },
                p2p_boundary: {
                  identity_authority: false,
                  profile_canonical_store: false,
                  social_graph_canonical_store: false,
                  content_truth_source: false,
                  network_wide_authority: false,
                },
                abuse_contact: 'abuse@index.kukuri.example',
                report_endpoint: 'https://index.kukuri.example/v1/report',
                terms_url: 'https://index.kukuri.example/terms',
              },
            });
          }
        },
      });
    },
    {
      locale,
      theme,
      adultLabeled,
      advisoryLabeled,
      columns,
      layoutKey: WORKSPACE_LAYOUT_STORAGE_KEY,
      objectId: OBJECT_ID,
      imageHash: IMAGE_HASH,
      authorPubkey: AUTHOR_PUBKEY,
      issuerNodeId: ADVISORY_ISSUER_NODE_ID,
      png: PNG_BASE64,
    }
  );
}

export async function runExploreSearch(page: Page) {
  await page.goto('/#/explore?topic=kukuri%3Atopic%3Ageneral');
  const explore = page.getByTestId('community-index-explore');
  const form = explore.locator('form.shell-community-index-form');
  await form.locator('input').first().fill('media');
  await form.locator('button[type="submit"]').click();
  return explore;
}

