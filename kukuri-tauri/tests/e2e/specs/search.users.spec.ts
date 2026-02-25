import { $, $$, browser, expect } from '@wdio/globals';

import {
  resetAppState,
  primeUserSearchRateLimit,
  seedUserSearchFixture,
  type SeedUserSearchFixtureResult,
} from '../helpers/bridge';
import {
  completeProfileSetup,
  waitForHome,
  waitForWelcome,
  type ProfileInfo,
} from '../helpers/appActions';
import { waitForAppReady } from '../helpers/waitForAppReady';

const ERROR_SELECTOR = '[data-testid="user-search-error"]';
const RESULTS_SELECTOR = '[data-testid="user-search-result"]';

async function isErrorVisible(): Promise<boolean> {
  const cards = await $$(ERROR_SELECTOR);
  if (cards.length === 0) {
    return false;
  }
  try {
    return await cards[0]!.isDisplayed();
  } catch {
    return false;
  }
}

describe('ユーザー検索', () => {
  before(async () => {
    await waitForAppReady();
    await resetAppState();
  });

  it('補助検索・ページネーション・レート制限UIを検証する', async function () {
    this.timeout(300000);

    await waitForWelcome();
    const profile: ProfileInfo = {
      name: 'E2E Search',
      displayName: 'user-search-e2e',
      about: 'ユーザー検索のE2E検証',
    };

    await $('[data-testid="welcome-create-account"]').click();
    await completeProfileSetup(profile);
    await waitForHome();
    await waitForAppReady();

    const fillerUsers = Array.from({ length: 22 }, (_, index) => ({
      displayName: `Search Extra ${index + 1}`,
      about: `extra user ${index + 1}`,
      follow: false,
    }));

    const fixtureUsers = [
      { displayName: 'Search Alpha', about: 'alpha user', follow: true },
      { displayName: 'Search Beta', about: 'beta user', follow: false },
      { displayName: 'Search Gamma', about: 'gamma user', follow: false },
      ...fillerUsers,
    ];
    const seededUsers: SeedUserSearchFixtureResult['users'] = [];
    const chunkSize = 8;
    for (let start = 0; start < fixtureUsers.length; start += chunkSize) {
      const chunk = fixtureUsers.slice(start, start + chunkSize);
      if (chunk.length === 0) {
        continue;
      }
      const chunkResult = await seedUserSearchFixture({ users: chunk });
      seededUsers.push(...chunkResult.users);
    }
    const searchQuery = 'npub1';
    const targetUser =
      seededUsers.find((user) => user.displayName === 'Search Beta') ??
      seededUsers[1] ??
      seededUsers[0];
    const targetQuery = targetUser?.npub ?? searchQuery;

    await $('[data-testid="category-search"]').click();
    await $('[data-testid="search-page"]').waitForDisplayed({ timeout: 20000 });
    const usersTab = await $('[data-testid="search-tab-users"]');
    await usersTab.click();
    await $('[data-testid="user-search-results"]').waitForDisplayed({ timeout: 30000 });

    const searchInput = await $('[data-testid="search-input"]');

    await searchInput.setValue('s');
    await $(ERROR_SELECTOR).waitForDisplayed({ timeout: 15000 });
    const validationMessage = await $('[data-testid="search-validation-message"]');
    expect(await validationMessage.getText()).toContain('2');
    await $('[data-testid="search-clear"]').click();

    await searchInput.setValue('@s');
    await browser.waitUntil(async () => !(await isErrorVisible()), {
      timeout: 10000,
      timeoutMsg: '補助検索でエラー表示が解除されない',
    });
    const helperLabel = await $('[data-testid="search-helper-label"]');
    expect(await helperLabel.getText()).toContain('@s');

    await searchInput.setValue(searchQuery);
    await browser.waitUntil(async () => (await $$(RESULTS_SELECTOR)).length >= 3, {
      timeout: 30000,
      interval: 500,
      timeoutMsg: '検索結果が表示されない',
    });
    const resultsBeforeSort = await $$(RESULTS_SELECTOR);
    expect(resultsBeforeSort.length).toBeGreaterThanOrEqual(3);

    const relevanceButton = await $('[data-testid="user-search-sort-relevance"]');
    const recencyButton = await $('[data-testid="user-search-sort-recency"]');
    expect(await relevanceButton.getAttribute('aria-pressed')).toBe('true');

    await recencyButton.click();
    await browser.waitUntil(
      async () => (await recencyButton.getAttribute('aria-pressed')) === 'true',
      { timeout: 10000, timeoutMsg: '最新順ソートが反映されない' },
    );

    const initialResults = await $$(RESULTS_SELECTOR);
    const loadMore = await $('[data-testid="user-search-load-more"]');
    await loadMore.waitForDisplayed({ timeout: 10000 });
    await loadMore.scrollIntoView();
    await loadMore.waitForClickable({ timeout: 10000 });
    await loadMore.click();
    await browser.waitUntil(
      async () => (await $$(RESULTS_SELECTOR)).length > initialResults.length,
      { timeout: 20000, interval: 500, timeoutMsg: 'さらに表示で結果が増えない' },
    );

    const primeResult = await primeUserSearchRateLimit({ query: searchQuery });
    expect(primeResult.triggered).toBe(true);

    await searchInput.setValue('Search limit');
    await $(ERROR_SELECTOR).waitForDisplayed({ timeout: 15000 });

    const retryButton = await $('[data-testid="user-search-retry-button"]');
    expect(await retryButton.isEnabled()).toBe(false);

    await searchInput.setValue(targetQuery);
    await browser.waitUntil(async () => !(await isErrorVisible()), {
      timeout: 20000,
      interval: 500,
      timeoutMsg: 'レート制限の解除が反映されない',
    });

    await browser.waitUntil(
      async () => {
        const cards = await $$(RESULTS_SELECTOR);
        for (const card of cards) {
          const text = await card.getText();
          if (text.includes(targetQuery)) {
            return true;
          }
        }
        return false;
      },
      { timeout: 20000, interval: 500, timeoutMsg: 'クールダウン後に結果が復帰しない' },
    );
  });
});
