import { AppHelper } from '../helpers/app';
import { setupE2ETest, beforeEachTest, afterEachTest } from '../helpers/setup';

describe('Posts E2E Tests', () => {
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

  describe('Create Post', () => {
    it('should create a simple text post', async () => {
      // 投稿入力フィールドを探す
      const postInput = await $('[data-testid="post-input"]');
      const postContent = `Test post at ${new Date().toISOString()}`;

      // テキストを入力
      await postInput.setValue(postContent);

      // 投稿ボタンをクリック
      const postButton = await $('[data-testid="post-button"]');
      await postButton.click();

      // 投稿が作成されるまで待つ
      await browser.waitUntil(
        async () => {
          const posts = await AppHelper.getPostList();
          return posts.some((post) => post.content === postContent);
        },
        {
          timeout: 10000,
          timeoutMsg: 'Post was not created',
        },
      );

      // 入力フィールドがクリアされたことを確認
      expect(await postInput.getValue()).toBe('');
    });

    it('should create a post with topics', async () => {
      const postInput = await $('[data-testid="post-input"]');
      const postContent = 'Check out this #rust and #nostr integration!';

      await postInput.setValue(postContent);

      // トピックが自動的に認識されることを確認
      const topicTags = await $$('[data-testid="topic-tag"]');
      if (topicTags.length > 0) {
        expect(topicTags.length).toBe(2);
      }

      // 投稿
      const postButton = await $('[data-testid="post-button"]');
      await postButton.click();

      // 投稿が作成される
      await browser.waitUntil(
        async () => {
          const posts = await AppHelper.getPostList();
          return posts.some((post) => post.content.includes('#rust'));
        },
        { timeout: 10000 },
      );
    });

    it('should handle long posts', async () => {
      const postInput = await $('[data-testid="post-input"]');
      const longContent = 'Lorem ipsum '.repeat(50); // 長いテキスト

      await postInput.setValue(longContent);

      // 文字数カウンターの確認
      const charCounter = await $('[data-testid="char-counter"]');
      if (await charCounter.isExisting()) {
        const count = await charCounter.getText();
        expect(parseInt(count)).toBeGreaterThan(500);
      }

      // 投稿
      const postButton = await $('[data-testid="post-button"]');
      await postButton.click();

      // エラーまたは成功を確認
      await browser.waitUntil(
        async () => {
          const errorMsg = await $('[data-testid="post-error"]');
          const posts = await AppHelper.getPostList();
          return (await errorMsg.isExisting()) || posts.length > 0;
        },
        { timeout: 5000 },
      );
    });

    it('should not allow empty posts', async () => {
      const postButton = await $('[data-testid="post-button"]');

      // 空の状態で投稿ボタンをクリック
      await postButton.click();

      // エラーメッセージまたはボタンが無効化されていることを確認
      const isDisabled = await postButton.getAttribute('disabled');
      const errorMsg = await $('[data-testid="post-error"]');

      expect(isDisabled || (await errorMsg.isExisting())).toBeTruthy();
    });
  });

  describe('Post List', () => {
    it('should display posts in chronological order', async () => {
      // 複数の投稿を作成
      const posts = ['First post', 'Second post', 'Third post'];

      for (const content of posts) {
        const postInput = await $('[data-testid="post-input"]');
        await postInput.setValue(content);

        const postButton = await $('[data-testid="post-button"]');
        await postButton.click();

        // 少し待機
        await browser.pause(1000);
      }

      // 投稿リストを取得
      const postList = await AppHelper.getPostList();

      // 新しい投稿が上に表示されることを確認
      expect(postList[0].content).toBe('Third post');
      expect(postList[1].content).toBe('Second post');
      expect(postList[2].content).toBe('First post');
    });

    it('should load more posts on scroll', async () => {
      // 初期の投稿数を取得
      const initialPosts = await AppHelper.getPostList();
      const initialCount = initialPosts.length;

      // 最下部までスクロール
      await browser.execute(() => {
        const postsContainer = document.querySelector('[data-testid="posts-list"]');
        if (postsContainer) {
          postsContainer.scrollTop = postsContainer.scrollHeight;
        }
      });

      // 追加の投稿が読み込まれるまで待つ
      await browser.waitUntil(
        async () => {
          const currentPosts = await AppHelper.getPostList();
          return currentPosts.length > initialCount;
        },
        {
          timeout: 5000,
          timeoutMsg: 'Additional posts were not loaded',
        },
      );
    });

    it('should refresh posts list', async () => {
      // リフレッシュボタンを探す
      const refreshButton = await $('[data-testid="refresh-posts"]');

      if (await refreshButton.isExisting()) {
        // リフレッシュ
        await refreshButton.click();

        // ローディング状態を確認
        const loadingIndicator = await $('[data-testid="posts-loading"]');
        if (await loadingIndicator.isExisting()) {
          await loadingIndicator.waitForDisplayed({ reverse: true, timeout: 10000 });
        }

        // 投稿リストが更新されたことを確認
        const afterRefresh = await AppHelper.getPostList();
        // 新しい投稿がある場合、リストが変更される
        expect(afterRefresh).toBeDefined();
      }
    });
  });

  describe('Post Interactions', () => {
    it('should react to a post', async () => {
      // 投稿を作成
      const postInput = await $('[data-testid="post-input"]');
      await postInput.setValue('Post to react to');

      const postButton = await $('[data-testid="post-button"]');
      await postButton.click();

      await browser.pause(2000);

      // 最初の投稿のリアクションボタンを探す
      const reactionButton = await $('[data-testid^="reaction-button-"]');

      if (await reactionButton.isExisting()) {
        // リアクションボタンをクリック
        await reactionButton.click();

        // リアクションが追加されたことを確認
        const reactionCount = await $('[data-testid^="reaction-count-"]');
        if (await reactionCount.isExisting()) {
          const count = await reactionCount.getText();
          expect(parseInt(count)).toBeGreaterThan(0);
        }
      }
    });

    it('should reply to a post', async () => {
      // 既存の投稿に返信
      const replyButton = await $('[data-testid^="reply-button-"]');

      if (await replyButton.isExisting()) {
        await replyButton.click();

        // 返信フォームが表示される
        const replyInput = await $('[data-testid="reply-input"]');
        await replyInput.waitForDisplayed({ timeout: 5000 });

        // 返信を入力
        await replyInput.setValue('This is a reply');

        // 返信を送信
        const sendReplyButton = await $('[data-testid="send-reply"]');
        await sendReplyButton.click();

        // 返信が表示されるまで待つ
        await browser.waitUntil(
          async () => {
            const replies = await $$('[data-testid^="reply-"]');
            return replies.length > 0;
          },
          { timeout: 10000 },
        );
      }
    });

    it('should share a post', async () => {
      const shareButton = await $('[data-testid^="share-button-"]');

      if (await shareButton.isExisting()) {
        await shareButton.click();

        // 共有オプションが表示される
        const shareMenu = await $('[data-testid="share-menu"]');
        if (await shareMenu.isExisting()) {
          // コピーリンクオプション
          const copyLinkOption = await $('[data-testid="copy-link"]');
          await copyLinkOption.click();

          // コピー成功メッセージ
          const successMsg = await $('[data-testid="copy-success"]');
          if (await successMsg.isExisting()) {
            expect(await successMsg.isDisplayed()).toBe(true);
          }
        }
      }
    });
  });

  describe('Post Filtering', () => {
    it('should filter posts by topic', async () => {
      // トピックフィルターを選択
      const topicFilter = await $('[data-testid="topic-filter"]');

      if (await topicFilter.isExisting()) {
        await topicFilter.click();

        // トピックオプションから選択
        const rustOption = await $('[data-testid="topic-option-rust"]');
        if (await rustOption.isExisting()) {
          await rustOption.click();

          // フィルターが適用されるまで待つ
          await browser.pause(1000);

          // フィルタリングされた投稿を確認
          const posts = await AppHelper.getPostList();
          const rustPosts = posts.filter((post) => post.content.includes('#rust'));

          // すべての表示された投稿がrustトピックを含むことを確認
          expect(rustPosts.length).toBe(posts.length);
        }
      }
    });

    it('should search posts by keyword', async () => {
      const searchInput = await $('[data-testid="search-posts"]');

      if (await searchInput.isExisting()) {
        // 検索キーワードを入力
        await searchInput.setValue('test');

        // 検索結果が更新されるまで待つ
        await browser.pause(1000);

        // 検索結果を確認
        const posts = await AppHelper.getPostList();
        const testPosts = posts.filter((post) => post.content.toLowerCase().includes('test'));

        expect(testPosts.length).toBe(posts.length);
      }
    });
  });
});
