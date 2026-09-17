import { type Page } from '@playwright/test';

// #1056: タイムラインに、採用ノードの推定(blob 対象)が付いた画像投稿を 1 件だけ置く fixture。
// #1108 の詳細 dialog の撮影 spec と視覚回帰からも再利用する。

export const TIMELINE_ADVISORY_OBJECT_ID = 'timeline-advisory-post';
export const TIMELINE_ADVISORY_URL = '/#/timeline?topic=kukuri%3Atopic%3Ageneral';

export type TimelineAdvisorySeedOptions = {
  locale: 'ja' | 'en' | 'zh-CN';
  theme: 'dark' | 'light';
  lookup: 'advisory' | 'pending';
};

export async function seedTimelineAdvisory(
  page: Page,
  { locale, theme, lookup }: TimelineAdvisorySeedOptions
) {
  await page.addInitScript(
    ({ locale, theme, lookup, objectId }) => {
      localStorage.setItem('kukuri.desktop.locale', locale);
      localStorage.setItem('kukuri.desktop.theme', theme);
      const imageHash = 'b'.repeat(64);
      const issuerNodeId = 'd'.repeat(64);
      const png =
        'iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAEhQGAhKmMIQAAAABJRU5ErkJggg==';
      let desktopApi = (window as unknown as { __KUKURI_DESKTOP__?: unknown }).__KUKURI_DESKTOP__;
      Object.defineProperty(window, '__KUKURI_DESKTOP__', {
        configurable: true,
        get: () => desktopApi,
        set: (api: Record<string, unknown>) => {
          desktopApi = api;
          if (!api) return;
          const post = {
            object_id: objectId,
            envelope_id: `envelope-${objectId}`,
            author_pubkey: 'a'.repeat(64),
            author_name: 'kukuri builder',
            author_display_name: 'kukuri builder',
            author_picture_asset: null,
            following: false,
            followed_by: false,
            mutual: false,
            friend_of_friend: false,
            provenance: null,
            withdrawal: null,
            content: '水着の写真を投稿しました',
            content_status: 'Available',
            content_labels: [],
            attachments: [
              {
                hash: imageHash,
                mime: 'image/png',
                bytes: 467,
                role: 'image_original',
                status: 'Available',
              },
            ],
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
          api.listTimeline = async () => ({ items: [post], next_cursor: null });
          api.getBlobMediaPayload = async (hash: string, mime: string) =>
            hash === imageHash ? { bytes_base64: png, mime } : null;
          api.lookupCommunityNodeContentAdvisories = async () => {
            if (lookup === 'pending') return new Promise(() => {});
            const config = await (
              api.getCommunityNodeConfig as () => Promise<{ nodes: { base_url: string }[] }>
            )();
            return {
              nodes: config.nodes.map((node) => ({
                base_url: node.base_url,
                node_id: issuerNodeId,
                error: null,
                advisories: [
                  {
                    issuer_node_id: issuerNodeId,
                    subject_kind: 'blob_cid',
                    subject_id: imageHash,
                    category: 'nsfw',
                    label: 'adult',
                    confidence: 84,
                    signal_id: 'signal-timeline-1',
                    basis: 'classifier_score',
                  },
                ],
              })),
            };
          };
          const fetchManifest = api.fetchCommunityNodeManifest as (
            baseUrl: string
          ) => Promise<{ status: string; manifest?: Record<string, unknown> }>;
          api.fetchCommunityNodeManifest = async (baseUrl: string) => {
            const result = await fetchManifest(baseUrl);
            return result.manifest
              ? {
                  ...result,
                  manifest: { ...result.manifest, node_id: issuerNodeId, node_name: 'index.kukuri.example' },
                }
              : result;
          };
        },
      });
    },
    { locale, theme, lookup, objectId: TIMELINE_ADVISORY_OBJECT_ID }
  );
}
