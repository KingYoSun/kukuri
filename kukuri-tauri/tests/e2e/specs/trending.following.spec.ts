import { $, $$, browser, expect } from '@wdio/globals';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';

import {
  completeProfileSetup,
  waitForHome,
  waitForWelcome,
  type ProfileInfo,
} from '../helpers/appActions';
import {
  resetAppState,
  seedTrendingFixture,
  type SeedTrendingFixtureResult,
  type TrendingFixture,
} from '../helpers/bridge';
import { waitForAppReady } from '../helpers/waitForAppReady';

describe('トレンド/フォロー導線', () => {
  before(async () => {
    await waitForAppReady();
    await resetAppState();
  });

  it('トレンドとフォローのサマリーをフィクスチャどおりに表示できる', async function () {
    this.timeout(240000);

    await waitForWelcome();
    const profile: ProfileInfo = {
      name: 'E2E Trend',
      displayName: 'trending-following',
      about: 'トレンド/フォローのE2E検証',
    };
    await $('[data-testid="welcome-create-account"]').click();
    await completeProfileSetup(profile);
    await waitForHome();

    const fixturePath = join(process.cwd(), 'tests', 'e2e', 'fixtures', 'trending_feed.json');
    const fixture: TrendingFixture = JSON.parse(readFileSync(fixturePath, 'utf-8'));
    const seeded: SeedTrendingFixtureResult = await seedTrendingFixture(fixture);

    await $('[data-testid="category-trending"]').click();
    await $('[data-testid="trending-page"]').waitForDisplayed({ timeout: 30000 });

    const topicsCount = fixture.topics.length;
    const postsCount = fixture.topics.reduce(
      (total, topic) => total + (topic.posts?.length ?? 0),
      0,
    );

    await browser.waitUntil(
      async () => (await $('[data-testid="trending-summary-topics"]').getText()).includes(`${topicsCount}`),
      { timeout: 20000, timeoutMsg: 'Trending summary topics did not update' },
    );

    const summaryPostsText = await $('[data-testid="trending-summary-posts"]').getText();
    expect(summaryPostsText).toContain(`${postsCount}`);

    const targetTopic =
      seeded.topics.find((topic) => topic.name === fixture.topics[0]?.title) ??
      seeded.topics[0];
    const topicCard = await $(`[data-testid="trending-topic-${targetTopic.id}"]`);
    await topicCard.waitForDisplayed({ timeout: 20000 });
    expect((await topicCard.getText()).toLowerCase()).toContain(targetTopic.name.toLowerCase());

    const firstPostTitle = fixture.topics[0]?.posts?.[0]?.title;
    if (firstPostTitle) {
      const postsList = await topicCard.$(
        `[data-testid="trending-topic-${targetTopic.id}-posts"]`,
      );
      await postsList.waitForDisplayed({ timeout: 15000 });
      expect((await postsList.getText()).toLowerCase()).toContain(firstPostTitle.toLowerCase());
    }

    await $('[data-testid="category-following"]').click();
    await $('[data-testid="following-page"]').waitForDisplayed({ timeout: 30000 });

    await browser.waitUntil(
      async () => (await $('[data-testid="following-summary-posts"]').getText()).includes(`${postsCount}`),
      { timeout: 20000, timeoutMsg: 'Following summary posts did not update' },
    );

    const authorCount = new Set(
      fixture.topics.flatMap((topic) => (topic.posts ?? []).map((post) => post.author ?? '')),
    );
    const authorsText = await $('[data-testid="following-summary-authors"]').getText();
    expect(authorsText).toContain(`${authorCount.size}`);

    await browser.waitUntil(
      async () => (await $$('[data-testid^="following-post-"]')).length >= postsCount,
      { timeout: 20000, interval: 500, timeoutMsg: 'Following posts did not render' },
    );
    const followingPostsText = await $('[data-testid="following-posts"]').getText();
    if (firstPostTitle) {
      expect(followingPostsText.toLowerCase()).toContain(firstPostTitle.toLowerCase());
    }
  });
});
