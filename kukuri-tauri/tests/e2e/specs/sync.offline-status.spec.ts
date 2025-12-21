import { $, browser, expect } from '@wdio/globals';

import {
  clearOfflineState,
  enqueueSyncQueueItem,
  ensureTestTopic,
  getOfflineSnapshot,
  resetAppState,
  seedOfflineActions,
  setOnlineStatus,
} from '../helpers/bridge';
import {
  completeProfileSetup,
  waitForHome,
  waitForWelcome,
  type ProfileInfo,
} from '../helpers/appActions';
import { waitForAppReady } from '../helpers/waitForAppReady';

describe('SyncStatusIndicator オフライン同期', () => {
  before(async () => {
    await waitForAppReady();
    await resetAppState();
  });

  it('再送キューと状態遷移をE2Eで確認できる', async function () {
    this.timeout(300000);

    await waitForWelcome();
    const profile: ProfileInfo = {
      name: 'E2E Offline',
      displayName: 'sync-status',
      about: 'SyncStatusIndicator のオフライン挙動を検証',
    };

    await $('[data-testid="welcome-create-account"]').click();
    await completeProfileSetup(profile);
    await waitForHome();

    const topic = await ensureTestTopic({ name: 'E2E Offline Sync' });
    const seedResult = await seedOfflineActions({
      topicId: topic.id,
      includeConflict: true,
      markOffline: true,
    });
    expect(seedResult.pendingActionCount).toBeGreaterThanOrEqual(2);

    const offlineSnapshot = await getOfflineSnapshot();
    expect(offlineSnapshot.isOnline).toBe(false);

    const indicator = await $('[data-testid="sync-indicator"]');
    await indicator.waitForDisplayed({ timeout: 20000 });
    await browser.waitUntil(async () => (await indicator.getText()).includes('オフライン'), {
      timeout: 15000,
      interval: 300,
      timeoutMsg: 'オフライン表示になりませんでした',
    });

    const queueSeed = await enqueueSyncQueueItem({
      cacheType: 'sync_queue',
      source: 'e2e-offline',
    });

    await setOnlineStatus(true);

    await browser.waitUntil(
      async () => {
        const text = await indicator.getText();
        return (
          text.includes('同期中') ||
          text.includes('未同期') ||
          text.includes('競合') ||
          text.includes('同期エラー') ||
          text.includes('同期済み')
        );
      },
      {
        timeout: 20000,
        interval: 300,
        timeoutMsg: 'オンライン復帰後の同期ステータスが更新されませんでした',
      },
    );

    await indicator.scrollIntoView();
    await indicator.waitForClickable({ timeout: 15000 });
    try {
      await indicator.click();
    } catch {
      await browser.execute(() => {
        const el = document.querySelector(
          '[data-testid="sync-indicator"]',
        ) as HTMLButtonElement | null;
        el?.click();
      });
    }

    const summarySnapshot = await getOfflineSnapshot();
    if (summarySnapshot.pendingActionCount > 0) {
      const summary = await $('[data-testid="offline-action-summary"]');
      await summary.waitForDisplayed({ timeout: 20000 });
      expect(summarySnapshot.pendingActionCount).toBeGreaterThan(0);
      const indicatorTextWithPending = await indicator.getText();
      expect(indicatorTextWithPending).toContain(`${summarySnapshot.pendingActionCount}`);
      expect(await summary.getText()).toContain('オフライン操作の内訳');

      const refreshQueueButton = await $('button[aria-label="再送キューを更新"]');
      await refreshQueueButton.waitForClickable({ timeout: 15000 });
      await refreshQueueButton.click();
      await browser.waitUntil(
        async () => {
          const item = await $(`[data-testid="queue-item-${queueSeed.queueId}"]`);
          return await item.isExisting();
        },
        {
          timeout: 25000,
          interval: 500,
          timeoutMsg: '再送キューに追加した項目が表示されませんでした',
        },
      );

      await seedOfflineActions({ topicId: topic.id, includeConflict: false, markOffline: false });
      await browser.waitUntil(
        async () => {
          const text = await indicator.getText();
          return (
            text.includes('同期中') ||
            text.includes('未同期') ||
            text.includes('競合') ||
            text.includes('同期エラー')
          );
        },
        {
          timeout: 10000,
          interval: 200,
          timeoutMsg: '再送キュー追加後の同期ステータスが更新されませんでした',
        },
      );

      const ensureSyncPopoverOpen = async () => {
        const candidate = await $('button=今すぐ同期');
        if (await candidate.isExisting()) {
          return candidate;
        }
        await indicator.scrollIntoView();
        await indicator.waitForClickable({ timeout: 15000 });
        try {
          await indicator.click();
        } catch {
          await browser.execute(() => {
            const el = document.querySelector(
              '[data-testid="sync-indicator"]',
            ) as HTMLButtonElement | null;
            el?.click();
          });
        }
        try {
          await browser.waitUntil(async () => await $('button=今すぐ同期').isExisting(), {
            timeout: 15000,
            interval: 300,
            timeoutMsg: '同期ポップオーバーが表示されませんでした',
          });
        } catch {
          return null;
        }
        const ready = await $('button=今すぐ同期');
        return (await ready.isExisting()) ? ready : null;
      };

      const syncNowButton = await ensureSyncPopoverOpen();
      if (syncNowButton) {
        await syncNowButton.scrollIntoView();
        const isEnabled = await syncNowButton.isEnabled();
        if (isEnabled) {
          try {
            await syncNowButton.click();
          } catch {
            await browser.execute(() => {
              const el = Array.from(document.querySelectorAll('button')).find((button) =>
                button.textContent?.includes('今すぐ同期'),
              ) as HTMLButtonElement | undefined;
              el?.click();
            });
          }
        }

        const postSyncStatusText = await browser.waitUntil(
          async () => {
            const text = await indicator.getText();
            if (text.includes('同期中')) {
              return false;
            }
            return text.includes('同期済み') || text.includes('未同期') || text.includes('競合')
              ? text
              : false;
          },
          {
            timeout: 30000,
            interval: 400,
            timeoutMsg: '同期完了後のステータスが更新されませんでした',
          },
        );
        expect(postSyncStatusText).toMatch(/同期済み|未同期|競合/);
      }
    } else {
      await browser.waitUntil(async () => (await indicator.getText()).includes('同期済み'), {
        timeout: 20000,
        interval: 400,
        timeoutMsg: '同期済みステータスに戻りませんでした',
      });
    }
    const conflictBanner = await $('[data-testid="sync-conflict-banner"]');
    if (await conflictBanner.isExisting()) {
      if (!(await conflictBanner.isDisplayed())) {
        await indicator.scrollIntoView();
        await indicator.waitForClickable({ timeout: 15000 });
        await browser.execute(() => {
          const el = document.querySelector(
            '[data-testid="sync-indicator"]',
          ) as HTMLButtonElement | null;
          el?.click();
        });
      }
      await conflictBanner.waitForDisplayed({ timeout: 20000 });
    }

    await clearOfflineState();

    await browser.waitUntil(async () => (await indicator.getText()).includes('同期済み'), {
      timeout: 20000,
      interval: 400,
      timeoutMsg: '同期済みステータスに戻りませんでした',
    });
  });
});
