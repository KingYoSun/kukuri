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
  setOnlineStatus,
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

describe('\u30db\u30fc\u30e0/\u30c8\u30d4\u30c3\u30af/\u6295\u7a3f\u64cd\u4f5c', () => {
  before(async () => {
    await waitForAppReady();
    await resetAppState();
  });

  it('PostComposer\u3068PostCard\u306e\u64cd\u4f5c\u304b\u3089\u30c8\u30d4\u30c3\u30af\u4f5c\u6210/\u524a\u9664\u307e\u3067\u78ba\u8a8d\u3067\u304d\u308b', async function () {
    this.timeout(300000);

    await waitForWelcome();
    const profile: ProfileInfo = {
      name: 'E2E User',
      displayName: 'home-posts',
      about: '\u30db\u30fc\u30e0/\u30c8\u30d4\u30c3\u30af/\u6295\u7a3f\u306eE2E\u691c\u8a3c\u7528\u30a2\u30ab\u30a6\u30f3\u30c8',
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
      timeoutMsg: '\u753b\u50cf\u30de\u30fc\u30af\u30c0\u30a6\u30f3\u304c\u5165\u529b\u306b\u53cd\u6620\u3055\u308c\u306a\u3044',
    });
    await browser.waitUntil(
      async () => {
        const previewHost = await $('[data-testid="markdown-editor-pane"]').$(
          '.w-md-editor-preview',
        );
        const previewText = await previewHost.getText();
        return previewText.includes('E2E post');
      },
      { timeout: 20000, timeoutMsg: 'Markdown\u30d7\u30ec\u30d3\u30e5\u30fc\u304c\u66f4\u65b0\u3055\u308c\u306a\u3044' },
    );

    await $('[data-testid="save-draft-button"]').click();
    await browser.pause(300);

    await $('[data-testid="submit-post-button"]').click();
    await browser.waitUntil(async () => (await $$('[data-testid^="post-"]')).length > 0, {
      timeout: 40000,
      timeoutMsg: '\u6295\u7a3f\u30ea\u30b9\u30c8\u304c\u8868\u793a\u3055\u308c\u306a\u3044',
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
      { timeout: 20000, timeoutMsg: '\u3044\u3044\u306d\u306e\u30ab\u30a6\u30f3\u30c8\u304c\u5897\u3048\u306a\u3044' },
    );

    const boostButton = await firstPost.$('[data-testid$="-boost"]');
    const initialBoost = Number((await boostButton.getText()) || '0');
    await boostButton.click();
    await browser.waitUntil(
      async () => Number((await boostButton.getText()) || '0') > initialBoost,
      { timeout: 20000, timeoutMsg: '\u30d6\u30fc\u30b9\u30c8\u306e\u30ab\u30a6\u30f3\u30c8\u304c\u5897\u3048\u306a\u3044' },
    );

    const bookmarkButton = await firstPost.$('[data-testid$="-bookmark"]');
    await bookmarkButton.click();
    await browser.waitUntil(
      async () => (await bookmarkButton.getAttribute('aria-pressed')) === 'true',
      { timeout: 15000, timeoutMsg: '\u30d6\u30c3\u30af\u30de\u30fc\u30af\u304c\u6709\u52b9\u306b\u306a\u3089\u306a\u3044' },
    );
    await bookmarkButton.click();
    await browser.waitUntil(
      async () => (await bookmarkButton.getAttribute('aria-pressed')) === 'false',
      { timeout: 15000, timeoutMsg: '\u30d6\u30c3\u30af\u30de\u30fc\u30af\u304c\u89e3\u9664\u3055\u308c\u306a\u3044' },
    );

    const replyButton = await firstPost.$('[data-testid$="-reply"]');
    await replyButton.click();
    const replyInput = await $('[data-testid="reply-composer-input"]');
    await replyInput.waitForDisplayed({ timeout: 15000 });
    await replyInput.setValue('E2E reply content');
    await $('[data-testid="reply-submit-button"]').click();
    await browser.waitUntil(
      async () => (await $('[data-testid="posts-list"]').getText()).includes('E2E reply content'),
      { timeout: 30000, timeoutMsg: '\u8fd4\u4fe1\u6295\u7a3f\u304c\u30bf\u30a4\u30e0\u30e9\u30a4\u30f3\u306b\u898b\u3064\u304b\u3089\u306a\u3044' },
    );

    const quoteButton = await firstPost.$('[data-testid$="-quote"]');
    await quoteButton.click();
    const quoteInput = await $('[data-testid="quote-composer-input"]');
    await quoteInput.waitForDisplayed({ timeout: 30000 });
    await quoteInput.setValue('E2E quote content');
    await $('[data-testid="quote-submit-button"]').click();
    await browser.waitUntil(
      async () => (await $('[data-testid="posts-list"]').getText()).includes('E2E quote content'),
      { timeout: 30000, timeoutMsg: '\u5f15\u7528\u6295\u7a3f\u304c\u30bf\u30a4\u30e0\u30e9\u30a4\u30f3\u306b\u898b\u3064\u304b\u3089\u306a\u3044' },
    );

    await $('[data-testid="category-topics"]').click();
    await browser.waitUntil(async () => (await browser.getUrl()).includes('/topics'), {
      timeout: 15000,
      timeoutMsg: '\u30c8\u30d4\u30c3\u30af\u4e00\u89a7\u306b\u9077\u79fb\u3057\u306a\u3044',
    });

    await setOnlineStatus(false);
    await browser.waitUntil(async () => !(await getOfflineSnapshot()).isOnline, {
      timeout: 10000,
      interval: 200,
      timeoutMsg: '\u30aa\u30d5\u30e9\u30a4\u30f3\u72b6\u614b\u306b\u5207\u308a\u66ff\u308f\u308a\u307e\u305b\u3093',
    });
    const newTopicName = `e2e-offline-topic-${Date.now()}`;
    await $('[data-testid="open-topic-create"]').click();
    await $('[data-testid="topic-name-input"]').setValue(newTopicName);
    await $('[data-testid="topic-description-input"]').setValue('created offline');
    await $('[data-testid="topic-submit-button"]').click();
    await browser.waitUntil(
      async () => !(await $('[data-testid="topic-name-input"]').isDisplayed()),
      { timeout: 15000, timeoutMsg: '\u30c8\u30d4\u30c3\u30af\u4f5c\u6210\u30e2\u30fc\u30c0\u30eb\u304c\u9589\u3058\u306a\u3044' },
    );

    const offlineSnapshot = await getOfflineSnapshot();
    expect(offlineSnapshot.pendingActionCount).toBeGreaterThan(0);

    await setOnlineStatus(true);
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
      timeoutMsg: '\u30c8\u30d4\u30c3\u30af\u8a73\u7d30\u306b\u9077\u79fb\u3057\u306a\u3044',
    });

    await $('[data-testid="topic-actions-menu"]').click();
    await $('[data-testid="topic-delete-menu"]').click();
    const confirmDelete = await $('[data-testid="topic-delete-confirm"]');
    await confirmDelete.waitForDisplayed({ timeout: 10000 });
    await confirmDelete.scrollIntoView();
    try {
      await confirmDelete.click();
    } catch {
      await browser.execute(() => {
        const button = document.querySelector(
          '[data-testid="topic-delete-confirm"]',
        ) as HTMLButtonElement | null;
        button?.click();
      });
    }
    await browser.waitUntil(async () => (await browser.getUrl()).endsWith('/topics'), {
      timeout: 20000,
      timeoutMsg: '\u30c8\u30d4\u30c3\u30af\u4e00\u89a7\u306b\u623b\u3089\u306a\u3044',
    });
    await browser.waitUntil(
      async () => (await $$(`//h3[contains(., "${newTopicName}")]`)).length === 0,
      { timeout: 15000, timeoutMsg: '\u30c8\u30d4\u30c3\u30af\u30ab\u30fc\u30c9\u304c\u6d88\u3048\u306a\u3044' },
    );
  });
});
