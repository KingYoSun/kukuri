import { expect, test, type Page } from '@playwright/test';

// #1056: タイムラインで、採用ノードの推定が付いた投稿が「見つける」と同じ代替表示になること、
// 照会中はスケルトン(代替表示とは別)になること、設定で採用を切り替えられることを実ブラウザで確認する。
// 視覚回帰 baseline は増やさず、記録用の画像だけを環境変数指定時に書き出す。変更前の撮影では
// 新しい挙動の検証を行わない(KUKURI_1056_SHOT_PREFIX=before)。

const SHOT_DIR = '../../docs/ui-reviews/assets/1056';
const SHOT_PREFIX = process.env.KUKURI_1056_SHOT_PREFIX ?? 'after';
const VERIFY = SHOT_PREFIX === 'after';
const OBJECT_ID = 'timeline-advisory-post';

async function capture(page: Page, name: string) {
  if (!process.env.KUKURI_1056_SHOTS) return;
  await page.screenshot({ path: `${SHOT_DIR}/${SHOT_PREFIX}-${name}.png` });
}

type SeedOptions = {
  locale: 'ja' | 'en';
  theme: 'dark' | 'light';
  lookup: 'advisory' | 'pending';
};

async function seedTimelineAdvisory(page: Page, { locale, theme, lookup }: SeedOptions) {
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
    { locale, theme, lookup, objectId: OBJECT_ID }
  );
}

test('advisory-labeled timeline posts are gated and explain the issuing node', async ({ page }) => {
  await page.setViewportSize({ width: 1400, height: 980 });
  await seedTimelineAdvisory(page, { locale: 'ja', theme: 'dark', lookup: 'advisory' });
  await page.goto('/#/timeline?topic=kukuri%3Atopic%3Ageneral');
  const card = page.locator(`[data-post-object-id="${OBJECT_ID}"]`).first();
  await expect(card).toBeVisible();
  if (VERIFY) {
    await expect(card.getByTestId(`media-adult-gated-${OBJECT_ID}`)).toBeVisible();
    await expect(card.getByTestId(`media-preview-${OBJECT_ID}`)).toHaveCount(0);
    await expect(card.getByTestId(`post-advisory-gated-${OBJECT_ID}`)).toContainText(
      'コミュニティノードによる推定'
    );
    const appeal = card.getByTestId(`post-advisory-appeal-${OBJECT_ID}`);
    await appeal.focus();
    await expect(appeal).toBeFocused();
  } else {
    await page.waitForTimeout(1500);
  }
  await capture(page, 'ja-dark-1400-timeline-gated');
});

test('the placeholder stays narrow-safe in the light theme', async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await seedTimelineAdvisory(page, { locale: 'ja', theme: 'light', lookup: 'advisory' });
  await page.goto('/#/timeline?topic=kukuri%3Atopic%3Ageneral');
  const card = page.locator(`[data-post-object-id="${OBJECT_ID}"]`).first();
  await expect(card).toBeVisible();
  if (VERIFY) {
    await expect(card.getByTestId(`post-advisory-gated-${OBJECT_ID}`)).toBeVisible();
    const overflow = await page.evaluate(
      () => document.documentElement.scrollWidth - document.documentElement.clientWidth
    );
    expect(overflow).toBeLessThanOrEqual(0);
  } else {
    await page.waitForTimeout(1500);
  }
  await capture(page, 'ja-light-390-timeline-gated');
});

test('pending lookups show a skeleton that differs from the placeholder', async ({ page }) => {
  await page.setViewportSize({ width: 1400, height: 980 });
  await seedTimelineAdvisory(page, { locale: 'ja', theme: 'dark', lookup: 'pending' });
  await page.goto('/#/timeline?topic=kukuri%3Atopic%3Ageneral');
  const card = page.locator(`[data-post-object-id="${OBJECT_ID}"]`).first();
  await expect(card).toBeVisible();
  if (!VERIFY) return;
  const pending = card.getByTestId(`media-advisory-pending-${OBJECT_ID}`);
  await expect(pending).toBeVisible();
  await expect(pending).toHaveAttribute('aria-busy', 'true');
  await expect(card.getByTestId(`media-adult-gated-${OBJECT_ID}`)).toHaveCount(0);
  await expect(card.getByTestId(`media-preview-${OBJECT_ID}`)).toHaveCount(0);
  await capture(page, 'ja-dark-1400-timeline-pending');
});

test('the adoption toggle is shown per node in Community Node settings', async ({ page }) => {
  await page.setViewportSize({ width: 1400, height: 980 });
  await seedTimelineAdvisory(page, { locale: 'ja', theme: 'dark', lookup: 'advisory' });
  await page.goto('/#/timeline?topic=kukuri%3Atopic%3Ageneral&settings=community-node');
  const dialog = page.getByRole('dialog');
  await expect(dialog).toBeVisible();
  if (!VERIFY) return;
  const toggle = dialog.getByRole('checkbox', { name: 'このノードの成人向け表現の推定を使う' });
  await toggle.evaluate((element) => element.scrollIntoView({ block: 'center' }));
  await expect(toggle).toBeChecked();
  await capture(page, 'ja-dark-1400-settings-adoption');
  await toggle.click();
  await expect(toggle).not.toBeChecked();
  await expect(dialog).toContainText('このノードへは推定を確認せず、推定も使いません。');
});
