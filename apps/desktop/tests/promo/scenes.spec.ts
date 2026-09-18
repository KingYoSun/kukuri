import { expect, test, type Page } from '@playwright/test';

import { captureSceneStep } from './fixtures/captureScene';
import {
  DEMO_CHANNEL_LABEL,
  DEMO_TOPIC,
  MINATO,
  seedDemoStory,
  type PromoLocale,
} from './fixtures/demoStory';
import type { CaptureTarget } from './promoArtifacts';

/**
 * 台本 (docs/progress/2026-09-15-promo-lp-brief.md) の 3 場面を撮る (#1039)。
 *
 * S0 Hero 候補 / S1 話題を選ぶ / S2 公開で会話する / S3 私的チャンネルへ移る。
 * いずれも開発者モードは無効のまま撮る。実験機能 (Dome・Live・Game・Stream) の
 * 場面は本 spec で作らない。実機でしか撮れない同期の場面と Dome 予告は #1040 が撮る。
 */

const VIEWPORT = { width: 1600, height: 1000 };
const THEME = 'dark' as const;

// URL で topic を指定すると既存のワークスペースの右端に別 Column が増え、目的の
// Column が画面の端に寄る。既定の配置のまま、先頭の Timeline Column で話題を切り替える。
const APP_URL = '/';

const COPY = {
  ja: {
    privateChannelEntry: 'チャンネル作成・参加',
    channelDialog: 'プライベートチャンネル作成 / 参加',
    channelName: 'チャンネル名',
    createChannel: 'チャンネルを作成',
    closeDialog: 'ダイアログを閉じる',
    searchPlaceholder: 'インデックスを検索',
    searchSubmit: '結果を表示',
    searchWord: 'カラム',
    settingsEntryFor: (channel: string) => `${channel} のチャンネル設定と共有を開く`,
    closeColumnFor: (title: string) => `${title} を閉じる`,
    columnTitles: { notifications: '通知', messages: 'メッセージ' },
    writePost: '投稿を書く',
    writeReply: '返信を書く',
    reply: '返信',
    publicPost: '見つけるのカラムを右端に置いたら、検索しながら会話を追えるようになりました。',
    replyText: 'その並び、わたしも真似してみます。',
    firstChannelMessage: 'ここならカラム配置の細かい好みも、気軽に相談できますね。',
    captions: {
      s1c1: '話題を選ぶ',
      s1c2: '同意したノードで検索する',
      s2c1: '公開で投稿する',
      s2c2: 'スレッドで続ける',
      s2c3: '反応を返す',
      s3c1: '同じ話題の中に、小さな輪',
      s3c2: '公開範囲を選ぶ',
      s3c3: '話題を離れずに話す',
    },
  },
  en: {
    privateChannelEntry: 'Create or join a private channel',
    channelDialog: 'Create / Join Private Channel',
    channelName: 'Channel name',
    createChannel: 'Create Channel',
    closeDialog: 'Close dialog',
    searchPlaceholder: 'Search the index',
    searchSubmit: 'Show results',
    searchWord: 'column',
    settingsEntryFor: (channel: string) => `Open ${channel} channel settings and sharing`,
    closeColumnFor: (title: string) => `Close ${title}`,
    columnTitles: { notifications: 'Notifications', messages: 'Messages' },
    writePost: 'Write a post',
    writeReply: 'Write a reply',
    reply: 'Reply',
    publicPost: 'Moving Explore to the right edge lets me search while following the conversation.',
    replyText: 'I will try that arrangement too.',
    firstChannelMessage: 'Here we can talk through the finer layout preferences without worrying.',
    captions: {
      s1c1: 'Pick a topic',
      s1c2: 'Search through a node you consented to',
      s2c1: 'Post in the open',
      s2c2: 'Keep the thread going',
      s2c3: 'React',
      s3c1: 'A smaller circle, same topic',
      s3c2: 'Choose the audience',
      s3c3: 'Stay in the topic',
    },
  },
} as const;

function target(sceneId: string, cutId: string, locale: PromoLocale): CaptureTarget {
  return { sceneId, cutId, locale, theme: THEME, developerMode: false, viewport: VIEWPORT };
}

function firstColumn(page: Page) {
  return page.locator('[data-column-id]').first();
}

function topicSelect(page: Page) {
  return firstColumn(page).locator('select').first();
}

/** 先頭の Timeline Column をデモの話題に合わせる。撮影の構図を毎回同じにする。 */
async function selectDemoTopic(page: Page, locale: PromoLocale) {
  await expect(topicSelect(page)).toBeVisible();
  await topicSelect(page).selectOption({ value: DEMO_TOPIC });
  await expect(page.getByText(MINATO.name[locale]).first()).toBeVisible();
}

/** 撮影の前提。実験機能の面が写り込む状態で撮らない (INVAR-3)。 */
async function assertExperimentalSurfacesHidden(page: Page) {
  await expect(page.locator('[data-column-id][aria-label^="Metaverse"]')).toHaveCount(0);
  await expect(page.locator('[data-column-id][aria-label^="Stream"]')).toHaveCount(0);
}

function channelSettingsEntry(page: Page, locale: PromoLocale) {
  const copy = COPY[locale];
  return page.getByRole('button', { name: copy.settingsEntryFor(DEMO_CHANNEL_LABEL[locale]) });
}

/** 作成したチャンネルを表示している Column。 */
function channelColumn(page: Page, locale: PromoLocale) {
  return page.locator('[data-column-id]', { has: channelSettingsEntry(page, locale) });
}

/**
 * チャンネルが作られ、その中身が画面に出るまで待つ。
 *
 * 作成したチャンネルは既存の Column の右隣に開くので、ワークスペースを横に送って
 * 画面内へ入れる。URL は active な Column に追随するため完了条件に使わない
 * (Dialog を閉じると先頭の Column が active に戻り、URL も公開 topic へ戻る)。
 */
async function waitForChannelOpened(page: Page, locale: PromoLocale) {
  const entry = channelSettingsEntry(page, locale).first();
  await expect(entry).toBeAttached();
  await entry.scrollIntoViewIfNeeded();
  await expect(entry).toBeVisible();
}

/**
 * 通知と Messages の Column を閉じる。
 *
 * どちらもデモでは空の Column なので、残すとチャンネルの Column が画面の外へ押し出される。
 * 利用者も自分で行う Column の整理であり、画面の中身を偽るものではない。
 */
async function closeEmptyColumns(page: Page, locale: PromoLocale) {
  const copy = COPY[locale];
  for (const title of [copy.columnTitles.notifications, copy.columnTitles.messages]) {
    const close = page.getByRole('button', { name: copy.closeColumnFor(title), exact: true });
    await close.click();
    await expect(close).toHaveCount(0);
  }
}

/** 私的チャンネルの作成・参加 Dialog を開く。 */
async function openChannelDialog(page: Page, locale: PromoLocale) {
  const copy = COPY[locale];
  await firstColumn(page).getByRole('button', { name: copy.privateChannelEntry }).click();
  await expect(page.getByRole('dialog', { name: copy.channelDialog })).toBeVisible();
}

/** Dialog から私的チャンネルを作り、Dialog を閉じてチャンネルの Column を画面に出す。 */
async function createDemoChannel(page: Page, locale: PromoLocale) {
  const copy = COPY[locale];
  await openChannelDialog(page, locale);
  const dialog = page.getByRole('dialog', { name: copy.channelDialog });
  await dialog.getByPlaceholder(copy.channelName).fill(DEMO_CHANNEL_LABEL[locale]);
  await dialog.getByRole('button', { name: copy.createChannel }).click();
  await waitForChannelOpened(page, locale);
  await dialog.getByRole('button', { name: copy.closeDialog }).click();
  await expect(dialog).toBeHidden();
  await waitForChannelOpened(page, locale);
}

test.use({ viewport: VIEWPORT });

for (const locale of ['ja', 'en'] as const) {
  const copy = COPY[locale];

  test.describe(`promo scenes (${locale})`, () => {
    test.beforeEach(async ({ page }) => {
      await seedDemoStory(page, { locale, theme: THEME });
    });

    test('S0: Hero 候補の全景', async ({ page, browser }) => {
      await captureSceneStep(page, browser, {
        target: target('s0-hero', 'c1', locale),
        url: APP_URL,
        anchor: (p) => firstColumn(p),
        prepare: async (p) => {
          await assertExperimentalSurfacesHidden(p);
          await selectDemoTopic(p, locale);
        },
        caption: null,
      });
    });

    test('S1: 話題を選ぶ', async ({ page, browser }) => {
      await captureSceneStep(page, browser, {
        target: target('s1-topic', 'c1', locale),
        url: APP_URL,
        anchor: (p) => topicSelect(p),
        prepare: assertExperimentalSurfacesHidden,
        act: async (p) => {
          // 初期トピックの間を移動し、話題ごとに会話が分かれていることを見せる。
          await p.waitForTimeout(1200);
          await selectDemoTopic(p, locale);
          await p.waitForTimeout(2600);
        },
        caption: copy.captions.s1c1,
      });
    });

    test('S1: Community Index で検索する', async ({ page, browser }) => {
      await captureSceneStep(page, browser, {
        target: target('s1-topic', 'c2', locale),
        url: APP_URL,
        anchor: (p) => p.getByPlaceholder(copy.searchPlaceholder),
        prepare: assertExperimentalSurfacesHidden,
        act: async (p) => {
          const input = p.getByPlaceholder(copy.searchPlaceholder);
          await input.click();
          await p.waitForTimeout(600);
          await input.pressSequentially(copy.searchWord, { delay: 90 });
          await p.waitForTimeout(700);
          await p.getByRole('button', { name: copy.searchSubmit }).first().click();
          await p.waitForTimeout(2200);
        },
        caption: copy.captions.s1c2,
      });
    });

    test('S2: 公開で投稿する', async ({ page, browser }) => {
      await captureSceneStep(page, browser, {
        target: target('s2-conversation', 'c1', locale),
        url: APP_URL,
        anchor: (p) => topicSelect(p),
        prepare: async (p) => {
          await assertExperimentalSurfacesHidden(p);
          await selectDemoTopic(p, locale);
        },
        act: async (p) => {
          await p.waitForTimeout(1000);
          await firstColumn(p).locator('.shell-column-primary-action').click();
          const composer = p.getByPlaceholder(copy.writePost);
          await expect(composer).toBeVisible();
          await composer.pressSequentially(copy.publicPost, { delay: 40 });
          await p.waitForTimeout(500);
          await p.keyboard.press('Control+Enter');
          await expect(firstColumn(p).getByText(copy.publicPost)).toBeVisible();
          await p.waitForTimeout(2200);
        },
        caption: copy.captions.s2c1,
      });
    });

    test('S2: 返信してスレッドで続ける', async ({ page, browser }) => {
      await captureSceneStep(page, browser, {
        target: target('s2-conversation', 'c2', locale),
        url: APP_URL,
        anchor: (p) => topicSelect(p),
        prepare: async (p) => {
          await assertExperimentalSurfacesHidden(p);
          await selectDemoTopic(p, locale);
        },
        act: async (p) => {
          await p.waitForTimeout(1000);
          // 最初の投稿 (みなと) に返信する。
          await firstColumn(p).getByRole('button', { name: copy.reply, exact: true }).first().click();
          const composer = p.getByPlaceholder(copy.writeReply);
          await expect(composer).toBeVisible();
          await composer.pressSequentially(copy.replyText, { delay: 50 });
          await p.waitForTimeout(500);
          await p.keyboard.press('Control+Enter');
          await expect(p.getByText(copy.replyText).first()).toBeVisible();
          await p.waitForTimeout(2200);
        },
        caption: copy.captions.s2c2,
      });
    });

    test('S2: リアクションを返す', async ({ page, browser }) => {
      await captureSceneStep(page, browser, {
        target: target('s2-conversation', 'c3', locale),
        url: APP_URL,
        anchor: (p) => topicSelect(p),
        prepare: async (p) => {
          await assertExperimentalSurfacesHidden(p);
          await selectDemoTopic(p, locale);
        },
        act: async (p) => {
          // seed で 2 件付いている 👍 に、自分の分を足す。
          const thumbs = firstColumn(p).getByRole('button', { name: /👍/ }).first();
          await expect(thumbs).toHaveText(/2/);
          await p.waitForTimeout(1400);
          await thumbs.click();
          await expect(thumbs).toHaveText(/3/);
          await p.waitForTimeout(2400);
        },
        caption: copy.captions.s2c3,
      });
    });

    test('S3: 私的チャンネルの入口', async ({ page, browser }) => {
      await captureSceneStep(page, browser, {
        target: target('s3-private-channel', 'c1', locale),
        url: APP_URL,
        anchor: (p) => firstColumn(p).getByRole('button', { name: copy.privateChannelEntry }),
        prepare: async (p) => {
          await assertExperimentalSurfacesHidden(p);
          await selectDemoTopic(p, locale);
        },
        act: async (p) => {
          await p.waitForTimeout(1400);
          await openChannelDialog(p, locale);
          await p.waitForTimeout(2800);
        },
        caption: copy.captions.s3c1,
      });
    });

    test('S3: 公開範囲を選んで作成する', async ({ page, browser }) => {
      await captureSceneStep(page, browser, {
        target: target('s3-private-channel', 'c2', locale),
        url: APP_URL,
        anchor: (p) => firstColumn(p).getByRole('button', { name: copy.privateChannelEntry }),
        prepare: async (p) => {
          await assertExperimentalSurfacesHidden(p);
          await selectDemoTopic(p, locale);
          await openChannelDialog(p, locale);
          // 静止画でも名前が入った状態を見せる。空欄の画面は素材にしない。
          await p
            .getByRole('dialog', { name: copy.channelDialog })
            .getByPlaceholder(copy.channelName)
            .fill(DEMO_CHANNEL_LABEL[locale]);
        },
        act: async (p) => {
          const dialog = p.getByRole('dialog', { name: copy.channelDialog });
          await p.waitForTimeout(1400);
          await dialog.getByRole('button', { name: copy.createChannel }).click();
          await expect(p).toHaveURL(/channel=/);
          await waitForChannelOpened(p, locale);
          await p.waitForTimeout(2400);
        },
        caption: copy.captions.s3c2,
      });
    });

    test('S3: 作成したチャンネルで話す', async ({ page, browser }) => {
      await captureSceneStep(page, browser, {
        target: target('s3-private-channel', 'c3', locale),
        url: APP_URL,
        anchor: (p) => firstColumn(p).getByRole('button', { name: copy.privateChannelEntry }),
        prepare: async (p) => {
          await assertExperimentalSurfacesHidden(p);
          await closeEmptyColumns(p, locale);
          await selectDemoTopic(p, locale);
          await createDemoChannel(p, locale);
        },
        act: async (p) => {
          // 作ったばかりのチャンネルで、最初の一言を書く。
          await p.waitForTimeout(1000);
          await channelColumn(p, locale).locator('.shell-column-primary-action').click();
          const composer = p.getByPlaceholder(copy.writePost);
          await expect(composer).toBeVisible();
          await composer.pressSequentially(copy.firstChannelMessage, { delay: 45 });
          await p.waitForTimeout(500);
          await p.keyboard.press('Control+Enter');
          await expect(channelColumn(p, locale).getByText(copy.firstChannelMessage)).toBeVisible();
          await p.waitForTimeout(2200);
        },
        caption: copy.captions.s3c3,
      });
    });
  });
}
