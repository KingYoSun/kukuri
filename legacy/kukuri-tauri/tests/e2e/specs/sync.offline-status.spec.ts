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

const OFFLINE_LABELS = ['オフライン', 'Offline', '离线'] as const;
const SYNCING_LABELS = ['同期中', 'Syncing', '同步中'] as const;
const SYNC_TRANSITION_LABELS = [
  ...SYNCING_LABELS,
  '未同期',
  'Unsynced',
  '未同步',
  '競合',
  'Conflict',
  '冲突',
  '同期エラー',
  'Sync error',
  '同步错误',
  '同期済み',
  'Synced',
  '已同步',
] as const;
const SYNC_PENDING_LABELS = [
  ...SYNCING_LABELS,
  '未同期',
  'Unsynced',
  '未同步',
  '競合',
  'Conflict',
  '冲突',
  '同期エラー',
  'Sync error',
  '同步错误',
] as const;
const SYNC_STABLE_LABELS = [
  '同期済み',
  'Synced',
  '已同步',
  '未同期',
  'Unsynced',
  '未同步',
  '競合',
  'Conflict',
  '冲突',
] as const;
const SYNCED_LABELS = ['同期済み', 'Synced', '已同步'] as const;
const RETRY_QUEUE_BUTTON_LABELS = [
  '再送キューを更新',
  'Update retry queue',
  '更新重试队列',
] as const;
const SYNC_NOW_BUTTON_LABELS = ['今すぐ同期', 'Sync now', '立即同步'] as const;

const includesAny = (text: string, labels: readonly string[]) =>
  labels.some((label) => text.includes(label));

const isDisplayedSafe = async (element: WebdriverIO.Element) => {
  try {
    return await element.isDisplayed();
  } catch {
    return false;
  }
};

const findButtonByLabel = async (
  labels: readonly string[],
  options?: { visibleOnly?: boolean },
) => {
  const visibleOnly = options?.visibleOnly ?? false;

  for (const label of labels) {
    const ariaButton = await $(`button[aria-label="${label}"]`);
    if ((await ariaButton.isExisting()) && (!visibleOnly || (await isDisplayedSafe(ariaButton)))) {
      return ariaButton;
    }

    const button = await $(`button=${label}`);
    if ((await button.isExisting()) && (!visibleOnly || (await isDisplayedSafe(button)))) {
      return button;
    }

    const partialTextButton = await $(`//button[contains(normalize-space(.), "${label}")]`);
    if (
      (await partialTextButton.isExisting()) &&
      (!visibleOnly || (await isDisplayedSafe(partialTextButton)))
    ) {
      return partialTextButton;
    }
  }
  return null;
};

const clickButtonByLabel = async (
  labels: readonly string[],
  options?: { visibleOnly?: boolean; timeoutMs?: number },
) => {
  const visibleOnly = options?.visibleOnly ?? false;
  const timeoutMs = options?.timeoutMs ?? 5000;

  const button = await findButtonByLabel(labels, { visibleOnly });
  if (button) {
    try {
      await button.waitForClickable({ timeout: timeoutMs });
      await button.click();
      return true;
    } catch {
      // fall through to DOM click fallback
    }
  }

  return await browser.execute(
    (buttonLabels, requireVisible) => {
      const normalize = (value: string | null) => value?.replace(/\s+/g, ' ').trim() ?? '';
      const isVisible = (element: HTMLButtonElement) => {
        const style = window.getComputedStyle(element);
        const rect = element.getBoundingClientRect();
        return (
          style.display !== 'none' &&
          style.visibility !== 'hidden' &&
          style.opacity !== '0' &&
          rect.width > 0 &&
          rect.height > 0 &&
          rect.bottom > 0 &&
          rect.top < window.innerHeight
        );
      };

      const allButtons = Array.from(document.querySelectorAll('button')) as HTMLButtonElement[];
      const candidates = allButtons.filter((button) => {
        const ariaLabel = normalize(button.getAttribute('aria-label'));
        const text = normalize(button.textContent);
        return buttonLabels.some((label) => ariaLabel === label || text.includes(label));
      });
      if (candidates.length === 0) {
        return false;
      }

      const enabledCandidates = candidates.filter((candidate) => !candidate.disabled);
      const interactableCandidates = enabledCandidates.filter((candidate) => isVisible(candidate));
      const target =
        (requireVisible ? interactableCandidates[0] : undefined) ??
        interactableCandidates[0] ??
        enabledCandidates[0];
      if (!target) {
        return false;
      }
      target.click();
      return true;
    },
    [...labels],
    visibleOnly,
  );
};

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
    await browser.waitUntil(async () => includesAny(await indicator.getText(), OFFLINE_LABELS), {
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
        return includesAny(text, SYNC_TRANSITION_LABELS);
      },
      {
        timeout: 20000,
        interval: 300,
        timeoutMsg: 'オンライン復帰後の同期ステータスが更新されませんでした',
      },
    );

    await indicator.scrollIntoView();
    try {
      await indicator.waitForClickable({ timeout: 15000 });
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
      expect((await summary.getText()).trim().length).toBeGreaterThan(0);

      const queueRefreshed = await clickButtonByLabel(RETRY_QUEUE_BUTTON_LABELS, {
        visibleOnly: true,
      });
      if (!queueRefreshed) {
        throw new Error('再送キュー更新ボタンが見つかりませんでした');
      }
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
          return includesAny(text, SYNC_PENDING_LABELS);
        },
        {
          timeout: 10000,
          interval: 200,
          timeoutMsg: '再送キュー追加後の同期ステータスが更新されませんでした',
        },
      );

      const ensureSyncPopoverOpen = async () => {
        const candidate = await findButtonByLabel(SYNC_NOW_BUTTON_LABELS, {
          visibleOnly: true,
        });
        if (candidate) {
          return candidate;
        }
        await indicator.scrollIntoView();
        try {
          await indicator.waitForClickable({ timeout: 15000 });
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
          await browser.waitUntil(
            async () =>
              (await findButtonByLabel(SYNC_NOW_BUTTON_LABELS, { visibleOnly: true })) !== null,
            {
              timeout: 5000,
              interval: 300,
              timeoutMsg: '同期ポップオーバーが表示されませんでした',
            },
          );
        } catch {
          return null;
        }
        return await findButtonByLabel(SYNC_NOW_BUTTON_LABELS, { visibleOnly: true });
      };

      const syncNowButton = await ensureSyncPopoverOpen();
      if (syncNowButton) {
        await clickButtonByLabel(SYNC_NOW_BUTTON_LABELS, { visibleOnly: true, timeoutMs: 3000 });

        const postSyncStatusText = await browser.waitUntil(
          async () => {
            const text = await indicator.getText();
            if (includesAny(text, SYNCING_LABELS)) {
              return false;
            }
            return includesAny(text, SYNC_STABLE_LABELS) ? text : false;
          },
          {
            timeout: 30000,
            interval: 400,
            timeoutMsg: '同期完了後のステータスが更新されませんでした',
          },
        );
        expect(postSyncStatusText).toMatch(
          /同期済み|Synced|已同步|未同期|Unsynced|未同步|競合|Conflict|冲突/,
        );
      }
    } else {
      await browser.waitUntil(async () => includesAny(await indicator.getText(), SYNCED_LABELS), {
        timeout: 20000,
        interval: 400,
        timeoutMsg: '同期済みステータスに戻りませんでした',
      });
    }
    const conflictBanner = await $('[data-testid="sync-conflict-banner"]');
    if (await conflictBanner.isExisting()) {
      if (!(await conflictBanner.isDisplayed())) {
        await indicator.scrollIntoView();
        try {
          await indicator.waitForClickable({ timeout: 15000 });
          await indicator.click();
        } catch {
          await browser.execute(() => {
            const el = document.querySelector(
              '[data-testid="sync-indicator"]',
            ) as HTMLButtonElement | null;
            el?.click();
          });
        }
      }
      await conflictBanner.waitForDisplayed({ timeout: 20000 });
    }

    await clearOfflineState();

    await browser.waitUntil(async () => includesAny(await indicator.getText(), SYNCED_LABELS), {
      timeout: 20000,
      interval: 400,
      timeoutMsg: '同期済みステータスに戻りませんでした',
    });
  });
});
