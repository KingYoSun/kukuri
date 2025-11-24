import { $, $$, browser, expect } from '@wdio/globals';
import { mkdtempSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

import {
  completeProfileSetup,
  waitForHome,
  waitForWelcome,
  type ProfileInfo,
} from '../helpers/appActions';
import {
  getOfflineSnapshot,
  getTopicSnapshot,
  resetAppState,
  syncPendingTopicQueue,
} from '../helpers/bridge';
import { waitForAppReady } from '../helpers/waitForAppReady';

const IMAGE_BASE64 =
  'iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8Xw8AAtEB/QZWS+sAAAAASUVORK5CYII=';

function createTempImage(): string {
  const dir = mkdtempSync(join(tmpdir(), 'kukuri-e2e-'));
  const filePath = join(dir, 'composer-image.png');
  writeFileSync(filePath, Buffer.from(IMAGE_BASE64, 'base64'));
  return filePath;
}

describe('ホーム/トピック/投稿操作', () => {
  before(async () => {
    await waitForAppReady();
    await resetAppState();
  });

  it('PostComposerとPostCardの操作からトピック作成/削除まで確認できる', async function () {
    this.timeout(300000);

    await waitForWelcome();
    const profile: ProfileInfo = {
      name: 'E2E User',
      displayName: 'home-posts',
      about: 'ホーム/トピック/投稿のE2E検証用アカウント',
    };

    await $('[data-testid="welcome-create-account"]').click();
    await completeProfileSetup(profile);
    await waitForHome();

    const createPostButton = await $('[data-testid="create-post-button"]');
    await createPostButton.waitForDisplayed({ timeout: 20000 });
    await createPostButton.click();
    const topicSelector = await $('[data-testid="topic-selector"]');
    await topicSelector.waitForDisplayed({ timeout: 15000 });
    expect((await topicSelector.getText()).toLowerCase()).toContain('#public');

    await $('[data-testid="composer-tab-markdown"]').click();
    const markdownInput = await $('.w-md-editor-text-input');
    await markdownInput.waitForDisplayed({ timeout: 20000 });
    const markdownContent = `# E2E post
**composer** markdown flow`;
    await markdownInput.setValue(markdownContent);

    const localImagePath = createTempImage();
    const fileInput = await $('input[type="file"][accept^="image"]');
    await fileInput.waitForExist({ timeout: 15000 });
    await browser.execute((input: HTMLInputElement | null) => {
      if (input) {
        input.classList.remove('hidden');
        input.style.display = 'block';
      }
    }, fileInput);
    await fileInput.setValue(localImagePath);
    await browser.waitUntil(async () => (await markdownInput.getValue()).includes('!['), {
      timeout: 20000,
      timeoutMsg: '画像マークダウンが入力に反映されない',
    });
    await browser.waitUntil(
      async () => {
        const previewHost = await $('[data-testid="markdown-editor-pane"]').$(
          '.w-md-editor-preview',
        );
        const previewText = await previewHost.getText();
        return previewText.includes('E2E post');
      },
      { timeout: 20000, timeoutMsg: 'Markdownプレビューが更新されない' },
    );

    await $('[data-testid="save-draft-button"]').click();
    await browser.pause(300);

    await $('[data-testid="submit-post-button"]').click();
    await browser.waitUntil(async () => (await $$('[data-testid^="post-"]')).length > 0, {
      timeout: 40000,
      timeoutMsg: '投稿リストが表示されない',
    });
    const postCards = await $$('[data-testid^="post-"]');
    let targetPost: WebdriverIO.Element | null = null;
    for (const card of postCards) {
      const text = await card.getText();
      if (text.includes('E2E post')) {
        targetPost = card;
        break;
      }
    }
    expect(targetPost).not.toBeNull();
    const firstPost = targetPost!;

    const likeButton = await firstPost.$('[data-testid$="-like"]');
    const initialLike = Number((await likeButton.getText()) || '0');
    await likeButton.click();
    await browser.waitUntil(
      async () => Number((await likeButton.getText()) || '0') > initialLike,
      { timeout: 20000, timeoutMsg: 'いいねのカウントが増えない' },
    );

    const boostButton = await firstPost.$('[data-testid$="-boost"]');
    const initialBoost = Number((await boostButton.getText()) || '0');
    await boostButton.click();
    await browser.waitUntil(
      async () => Number((await boostButton.getText()) || '0') > initialBoost,
      { timeout: 20000, timeoutMsg: 'ブーストのカウントが増えない' },
    );

    const bookmarkButton = await firstPost.$('[data-testid$="-bookmark"]');
    await bookmarkButton.click();
    await browser.waitUntil(
      async () => (await bookmarkButton.getAttribute('aria-pressed')) === 'true',
      { timeout: 15000, timeoutMsg: 'ブックマークが有効にならない' },
    );
    await bookmarkButton.click();
    await browser.waitUntil(
      async () => (await bookmarkButton.getAttribute('aria-pressed')) === 'false',
      { timeout: 15000, timeoutMsg: 'ブックマークが解除されない' },
    );

    const replyButton = await firstPost.$('[data-testid$="-reply"]');
    await replyButton.click();
    const replyInput = await $('[data-testid="reply-composer-input"]');
    await replyInput.waitForDisplayed({ timeout: 15000 });
    await replyInput.setValue('E2E reply content');
    await $('[data-testid="reply-submit-button"]').click();
    await browser.waitUntil(
      async () => (await $('[data-testid="posts-list"]').getText()).includes('E2E reply content'),
      { timeout: 30000, timeoutMsg: '返信投稿がタイムラインに見つからない' },
    );

    const quoteButton = await firstPost.$('[data-testid$="-quote"]');
    await quoteButton.click();
    const quoteInput = await $('[data-testid="quote-composer-input"]');
    await quoteInput.waitForDisplayed({ timeout: 15000 });
    await quoteInput.setValue('E2E quote content');
    await $('[data-testid="quote-submit-button"]').click();
    await browser.waitUntil(
      async () => (await $('[data-testid="posts-list"]').getText()).includes('E2E quote content'),
      { timeout: 30000, timeoutMsg: '引用投稿がタイムラインに見つからない' },
    );

    await $('[data-testid="category-topics"]').click();
    await browser.waitUntil(async () => (await browser.getUrl()).includes('/topics'), {
      timeout: 15000,
      timeoutMsg: 'トピック一覧に遷移しない',
    });

    await browser.execute(() => {
      // E2Eでオフライン挙動を強制するフラグとイベントの両方を送出する
      (window as unknown as { __E2E_FORCE_OFFLINE__?: boolean }).__E2E_FORCE_OFFLINE__ = true;
      window.dispatchEvent(new Event('offline'));
    });
    const newTopicName = `e2e-offline-topic-${Date.now()}`;
    await $('[data-testid="open-topic-create"]').click();
    await $('[data-testid="topic-name-input"]').setValue(newTopicName);
    await $('[data-testid="topic-description-input"]').setValue('created offline');
    await $('[data-testid="topic-submit-button"]').click();
    await browser.waitUntil(
      async () => !(await $('[data-testid="topic-name-input"]').isDisplayed()),
      { timeout: 15000, timeoutMsg: 'トピック作成モーダルが閉じない' },
    );

    const offlineSnapshot = await getOfflineSnapshot();
    expect(offlineSnapshot.pendingActionCount).toBeGreaterThan(0);

    await browser.execute(() => {
      (window as unknown as { __E2E_FORCE_OFFLINE__?: boolean }).__E2E_FORCE_OFFLINE__ = false;
      window.dispatchEvent(new Event('online'));
    });
    const syncResult = await syncPendingTopicQueue();
    const topicSnapshotAfterSync = await getTopicSnapshot();
    expect(topicSnapshotAfterSync.pendingTopics.length).toBe(syncResult.pendingCountAfter);
    expect(topicSnapshotAfterSync.topics.some((topic) => topic.name === newTopicName)).toBe(true);

    const targetCard = await $(`//h3[contains(., "${newTopicName}")]`);
    await targetCard.waitForExist({ timeout: 15000 });
    await targetCard.scrollIntoView();
    await targetCard.click();
    await browser.waitUntil(async () => (await browser.getUrl()).includes('/topics/'), {
      timeout: 15000,
      timeoutMsg: 'トピック詳細に遷移しない',
    });

    await $('[data-testid="topic-actions-menu"]').click();
    await $('[data-testid="topic-delete-menu"]').click();
    await $('[data-testid="topic-delete-confirm"]').click();
    await browser.waitUntil(async () => (await browser.getUrl()).endsWith('/topics'), {
      timeout: 20000,
      timeoutMsg: 'トピック一覧に戻らない',
    });
    await browser.waitUntil(
      async () => (await $$(`//h3[contains(., "${newTopicName}")]`)).length === 0,
      { timeout: 15000, timeoutMsg: 'トピックカードが消えない' },
    );
  });
});
