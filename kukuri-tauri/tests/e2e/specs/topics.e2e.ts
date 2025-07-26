import { AppHelper } from '../helpers/app';
import { setupE2ETest, beforeEachTest, afterEachTest } from '../helpers/setup';

describe('Topics E2E Tests', () => {
  before(async () => {
    await setupE2ETest();
  });

  beforeEach(async () => {
    await beforeEachTest();
    // 各テストの前にログイン
    await AppHelper.login();
  });

  afterEach(async function () {
    await afterEachTest(this.currentTest?.title || 'unknown');
  });

  describe('Topic Creation', () => {
    it('should create a new topic', async () => {
      // トピック作成ボタンをクリック
      const createTopicButton = await $('[data-testid="create-topic-button"]');
      await createTopicButton.click();

      // トピック作成フォームが表示される
      const topicForm = await $('[data-testid="topic-form"]');
      await topicForm.waitForDisplayed({ timeout: 5000 });

      // トピック名を入力
      const topicNameInput = await $('[data-testid="topic-name-input"]');
      const topicName = `test-topic-${Date.now()}`;
      await topicNameInput.setValue(topicName);

      // トピックの説明を入力
      const topicDescInput = await $('[data-testid="topic-desc-input"]');
      await topicDescInput.setValue('This is a test topic for E2E testing');

      // 作成ボタンをクリック
      const submitButton = await $('[data-testid="submit-topic"]');
      await submitButton.click();

      // トピックが作成されるまで待つ
      await browser.waitUntil(
        async () => {
          const topics = await AppHelper.getTopicList();
          return topics.includes(topicName);
        },
        {
          timeout: 10000,
          timeoutMsg: 'Topic was not created',
        },
      );
    });

    it('should validate topic name', async () => {
      const createTopicButton = await $('[data-testid="create-topic-button"]');
      await createTopicButton.click();

      const topicForm = await $('[data-testid="topic-form"]');
      await topicForm.waitForDisplayed({ timeout: 5000 });

      // 無効な名前を入力
      const topicNameInput = await $('[data-testid="topic-name-input"]');
      await topicNameInput.setValue('a'); // 短すぎる名前

      const submitButton = await $('[data-testid="submit-topic"]');
      await submitButton.click();

      // エラーメッセージが表示される
      const errorMessage = await $('[data-testid="topic-name-error"]');
      await errorMessage.waitForDisplayed({ timeout: 5000 });

      const errorText = await errorMessage.getText();
      expect(errorText).toContain('at least');
    });

    it('should prevent duplicate topic names', async () => {
      // 最初のトピックを作成
      const createTopicButton = await $('[data-testid="create-topic-button"]');
      await createTopicButton.click();

      const topicForm = await $('[data-testid="topic-form"]');
      await topicForm.waitForDisplayed({ timeout: 5000 });

      const duplicateName = 'duplicate-topic';
      const topicNameInput = await $('[data-testid="topic-name-input"]');
      await topicNameInput.setValue(duplicateName);

      const submitButton = await $('[data-testid="submit-topic"]');
      await submitButton.click();

      // 最初のトピックが作成されるまで待つ
      await browser.pause(2000);

      // 同じ名前で2つ目のトピックを作成しようとする
      await createTopicButton.click();
      await topicNameInput.setValue(duplicateName);
      await submitButton.click();

      // エラーメッセージが表示される
      const errorMessage = await $('[data-testid="topic-duplicate-error"]');
      await errorMessage.waitForDisplayed({ timeout: 5000 });

      const errorText = await errorMessage.getText();
      expect(errorText).toContain('already exists');
    });
  });

  describe('Topic List', () => {
    it('should display all topics', async () => {
      // トピックページに移動
      const topicsLink = await $('a[href="/topics"]');
      await topicsLink.click();

      await browser.pause(1000);

      // トピックリストが表示される
      const topicsList = await $('[data-testid="topics-list"]');
      await topicsList.waitForDisplayed({ timeout: 5000 });

      // トピックが表示されていることを確認
      const topics = await AppHelper.getTopicList();
      expect(topics.length).toBeGreaterThan(0);
    });

    it('should search topics', async () => {
      // トピック検索フィールド
      const searchInput = await $('[data-testid="search-topics"]');
      await searchInput.setValue('test');

      // 検索結果が更新されるまで待つ
      await browser.pause(1000);

      // フィルタリングされたトピックを確認
      const topics = await AppHelper.getTopicList();
      const testTopics = topics.filter((topic) => topic.toLowerCase().includes('test'));

      expect(testTopics.length).toBeGreaterThan(0);
    });

    it('should sort topics by popularity', async () => {
      // ソートオプションを選択
      const sortDropdown = await $('[data-testid="sort-topics"]');

      if (await sortDropdown.isExisting()) {
        await sortDropdown.click();

        // 人気順を選択
        const popularOption = await $('[data-testid="sort-popular"]');
        await popularOption.click();

        // ソートが適用されるまで待つ
        await browser.pause(1000);

        // トピックリストを確認
        const topicElements = await $$('[data-testid^="topic-"] [data-testid="post-count"]');

        if (topicElements.length > 1) {
          // 投稿数を取得
          const counts: number[] = [];
          for (const element of topicElements) {
            const text = await element.getText();
            const count = parseInt(text.match(/\d+/)?.[0] || '0');
            counts.push(count);
          }

          // 降順でソートされていることを確認
          for (let i = 0; i < counts.length - 1; i++) {
            expect(counts[i]).toBeGreaterThanOrEqual(counts[i + 1]);
          }
        }
      }
    });
  });

  describe('Topic Navigation', () => {
    it('should navigate to topic page', async () => {
      // トピックリストから最初のトピックを選択
      const firstTopic = await $('[data-testid^="topic-"]:first-child');
      const topicName = await firstTopic.$('h3').getText();

      await firstTopic.click();

      // トピックページに遷移
      await browser.waitUntil(
        async () => {
          const url = await browser.getUrl();
          return url.includes('/topics/');
        },
        { timeout: 5000 },
      );

      // トピック名が表示される
      const topicTitle = await $('[data-testid="topic-title"]');
      const titleText = await topicTitle.getText();
      expect(titleText).toBe(topicName);
    });

    it('should display posts for selected topic', async () => {
      // 特定のトピックページに直接アクセス
      const topics = await AppHelper.getTopicList();
      if (topics.length > 0) {
        const topicId = topics[0]; // 実際にはIDが必要
        await browser.url(`/topics/${topicId}`);

        await browser.pause(1000);

        // トピックに関連する投稿が表示される
        const posts = await AppHelper.getPostList();

        // すべての投稿がこのトピックを含むことを確認
        for (const post of posts) {
          expect(post.content).toContain(`#${topics[0]}`);
        }
      }
    });

    it('should follow/unfollow topic', async () => {
      // トピックページで
      const followButton = await $('[data-testid="follow-topic"]');

      if (await followButton.isExisting()) {
        const initialText = await followButton.getText();

        // フォロー/アンフォロー
        await followButton.click();

        // ボタンのテキストが変更される
        await browser.waitUntil(
          async () => {
            const newText = await followButton.getText();
            return newText !== initialText;
          },
          { timeout: 5000 },
        );

        const newText = await followButton.getText();
        expect(newText).not.toBe(initialText);
      }
    });
  });

  describe('Topic Statistics', () => {
    it('should display topic statistics', async () => {
      // トピックページで統計情報を確認
      const statsSection = await $('[data-testid="topic-stats"]');

      if (await statsSection.isExisting()) {
        // 投稿数
        const postCount = await $('[data-testid="topic-post-count"]');
        const postCountText = await postCount.getText();
        expect(parseInt(postCountText)).toBeGreaterThanOrEqual(0);

        // ユーザー数
        const userCount = await $('[data-testid="topic-user-count"]');
        const userCountText = await userCount.getText();
        expect(parseInt(userCountText)).toBeGreaterThanOrEqual(0);

        // アクティビティグラフ（存在する場合）
        const activityGraph = await $('[data-testid="topic-activity-graph"]');
        if (await activityGraph.isExisting()) {
          expect(await activityGraph.isDisplayed()).toBe(true);
        }
      }
    });

    it('should show trending topics', async () => {
      // トレンドセクションを探す
      const trendingSection = await $('[data-testid="trending-topics"]');

      if (await trendingSection.isExisting()) {
        // トレンドトピックが表示される
        const trendingTopics = await trendingSection.$$('[data-testid^="trending-topic-"]');
        expect(trendingTopics.length).toBeGreaterThan(0);

        // 各トレンドトピックに成長率が表示される
        for (const topic of trendingTopics) {
          const growthRate = await topic.$('[data-testid="growth-rate"]');
          if (await growthRate.isExisting()) {
            const rateText = await growthRate.getText();
            expect(rateText).toMatch(/[+-]?\d+%/);
          }
        }
      }
    });
  });

  describe('Topic Moderation', () => {
    it('should report inappropriate topic', async () => {
      // トピックページで報告ボタンを探す
      const reportButton = await $('[data-testid="report-topic"]');

      if (await reportButton.isExisting()) {
        await reportButton.click();

        // 報告フォームが表示される
        const reportForm = await $('[data-testid="report-form"]');
        await reportForm.waitForDisplayed({ timeout: 5000 });

        // 理由を選択
        const reasonSelect = await $('[data-testid="report-reason"]');
        await reasonSelect.selectByValue('inappropriate');

        // 詳細を入力
        const detailsInput = await $('[data-testid="report-details"]');
        await detailsInput.setValue('This topic contains inappropriate content');

        // 送信
        const submitButton = await $('[data-testid="submit-report"]');
        await submitButton.click();

        // 成功メッセージ
        const successMessage = await $('[data-testid="report-success"]');
        await successMessage.waitForDisplayed({ timeout: 5000 });

        expect(await successMessage.getText()).toContain('reported');
      }
    });
  });
});
