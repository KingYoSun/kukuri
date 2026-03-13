import { $, $$, browser, expect } from '@wdio/globals';

import {
  completeProfileSetup,
  waitForHome,
  type ProfileInfo,
} from '../helpers/appActions';
import { ensureTestTopic } from '../helpers/bridge';

describe('Topic timeline/thread 統合フロー', () => {
  it('timeline から preview/detail/list を横断し、realtime 切替まで検証できる', async function () {
    this.timeout(300000);
    const welcomeScreen = await $('[data-testid="welcome-screen"]');
    await welcomeScreen.waitForDisplayed({ timeout: 90000 });

    const profile: ProfileInfo = {
      name: 'E2E Timeline Thread',
      displayName: 'timeline-thread-flow',
      about: 'Issue #146 統合シナリオ検証',
    };

    await $('[data-testid="welcome-create-account"]').click();
    await completeProfileSetup(profile);
    await waitForHome();

    const topicName = `e2e-thread-flow-${Date.now()}`;
    const topic = await ensureTestTopic({ name: topicName });

    await $('[data-testid="category-topics"]').click();
    await browser.waitUntil(
      async () => {
        const pathname = await browser.execute(() => window.location.pathname);
        return pathname === '/topics';
      },
      { timeout: 20000, timeoutMsg: 'トピック一覧へ遷移しませんでした' },
    );

    const topicCard = await $(`//h3[contains(., "${topicName}")]`);
    await topicCard.waitForDisplayed({ timeout: 20000 });
    await topicCard.click();

    await browser.waitUntil(
      async () => {
        const pathname = await browser.execute(() => window.location.pathname);
        return pathname === `/topics/${topic.id}`;
      },
      { timeout: 20000, timeoutMsg: '対象トピックページへ遷移しませんでした' },
    );

    const rootContent = `Issue146 root ${Date.now()}`;
    const replyContent = `Issue146 reply ${Date.now()}`;

    await $('[data-testid="create-post-button"]').click();
    const postInput = await $('[data-testid="post-input"]');
    await postInput.waitForDisplayed({ timeout: 20000 });
    await postInput.setValue(rootContent);
    await $('[data-testid="submit-post-button"]').click();

    let threadUuid: string | null = null;
    await browser.waitUntil(
      async () => {
        const cards = await $$('[data-testid^="timeline-thread-card-"]');
        for (const card of cards) {
          const text = await card.getText();
          if (!text.includes(rootContent)) {
            continue;
          }
          const testId = await card.getAttribute('data-testid');
          if (!testId) {
            continue;
          }
          threadUuid = testId.replace('timeline-thread-card-', '');
          return threadUuid.length > 0;
        }
        return false;
      },
      { timeout: 40000, interval: 500, timeoutMsg: '作成したスレッドカードが見つかりませんでした' },
    );

    expect(threadUuid).not.toBeNull();
    const resolvedThreadUuid = threadUuid!;

    const replyButton = await $(
      `[data-testid="timeline-thread-card-${resolvedThreadUuid}"] [data-testid$="-reply"]`,
    );
    await replyButton.waitForClickable({ timeout: 20000 });
    await replyButton.click();

    const replyInput = await $('[data-testid="reply-composer-input"]');
    await replyInput.waitForDisplayed({ timeout: 20000 });
    await replyInput.setValue(replyContent);
    await $('[data-testid="reply-submit-button"]').click();

    const firstReplySelector = `[data-testid="timeline-thread-first-reply-${resolvedThreadUuid}"]`;
    await browser.waitUntil(
      async () => {
        const firstReply = await $(firstReplySelector);
        if (!(await firstReply.isExisting())) {
          return false;
        }
        const text = await firstReply.getText();
        return text.includes(replyContent);
      },
      { timeout: 40000, interval: 500, timeoutMsg: '返信がタイムラインへ反映されませんでした' },
    );

    const parentCard = await $(`[data-testid="timeline-thread-parent-${resolvedThreadUuid}"]`);
    await parentCard.waitForClickable({ timeout: 20000 });
    await parentCard.click();

    const previewPane = await $('[data-testid="thread-preview-pane"]');
    await previewPane.waitForDisplayed({ timeout: 20000 });
    await browser.waitUntil(
      async () => {
        const previewText = await previewPane.getText();
        return previewText.includes(replyContent);
      },
      { timeout: 20000, interval: 300, timeoutMsg: '右ペイン preview に返信が表示されませんでした' },
    );

    const openThreadButton = await $(`[data-testid="timeline-thread-open-${resolvedThreadUuid}"]`);
    await openThreadButton.waitForClickable({ timeout: 20000 });
    await openThreadButton.click();

    await browser.waitUntil(
      async () => {
        const pathname = await browser.execute(() => window.location.pathname);
        return pathname === `/topics/${topic.id}/threads/${resolvedThreadUuid}`;
      },
      { timeout: 20000, timeoutMsg: 'preview から thread 詳細へ遷移しませんでした' },
    );

    await $('[data-testid="thread-list-title"]').waitForDisplayed({ timeout: 20000 });
    const deepLinkBodyText = await $('body').getText();
    expect(deepLinkBodyText).toContain(rootContent);
    expect(deepLinkBodyText).toContain(replyContent);

    await $('[data-testid="thread-list-back-to-topic"]').click();
    await browser.waitUntil(
      async () => {
        const pathname = await browser.execute(() => window.location.pathname);
        return pathname === `/topics/${topic.id}`;
      },
      { timeout: 20000, timeoutMsg: 'thread 一覧 deep-link からタイムラインへ戻れませんでした' },
    );

    await $('[data-testid="open-topic-threads-button"]').waitForDisplayed({ timeout: 20000 });
    await $('[data-testid="open-topic-threads-button"]').click();
    await browser.waitUntil(
      async () => {
        const pathname = await browser.execute(() => window.location.pathname);
        return pathname === `/topics/${topic.id}/threads`;
      },
      { timeout: 20000, timeoutMsg: 'thread 一覧へ遷移しませんでした' },
    );

    await $('[data-testid="thread-list-items"]').waitForDisplayed({ timeout: 20000 });
    const threadCardInList = await $(`[data-testid="timeline-thread-card-${resolvedThreadUuid}"]`);
    await threadCardInList.waitForDisplayed({ timeout: 20000 });

    await $('[data-testid="thread-list-back-to-topic"]').click();
    await browser.waitUntil(
      async () => {
        const pathname = await browser.execute(() => window.location.pathname);
        return pathname === `/topics/${topic.id}`;
      },
      { timeout: 20000, timeoutMsg: 'タイムラインへ戻れませんでした' },
    );

    await $('[data-testid="open-topic-threads-button"]').waitForDisplayed({ timeout: 20000 });

    const realtimeButton = await $('[data-testid="timeline-mode-toggle-realtime"]');
    await realtimeButton.waitForClickable({ timeout: 20000 });
    await realtimeButton.click();
    await browser.pause(300);

    const standardButton = await $('[data-testid="timeline-mode-toggle-standard"]');
    await standardButton.waitForClickable({ timeout: 20000 });
    await standardButton.click();
  });
});
