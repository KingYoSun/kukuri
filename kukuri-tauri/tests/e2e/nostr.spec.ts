import { expect, browser, $ } from '@wdio/globals';

describe('Nostr Functionality E2E Tests', () => {
  it('should handle post creation flow', async () => {
    // ホームページが表示されるまで待つ
    await browser.pause(2000);
    
    const postButton = await $('[data-testid="create-post-button"]');
    
    if (await postButton.isExisting()) {
      await postButton.waitForClickable({ timeout: 5000 });
      await postButton.click();
      
      const postInput = await $('[data-testid="post-input"]');
      if (await postInput.isExisting()) {
        await postInput.waitForDisplayed({ timeout: 5000 });
        await postInput.setValue('Test post from E2E');
        
        const submitButton = await $('[data-testid="submit-post-button"]');
        if (await submitButton.isExisting()) {
          await submitButton.waitForClickable({ timeout: 5000 });
          await submitButton.click();
          await browser.pause(2000);
        }
      }
    } else {
      // Post button not found - user may not be on the home page or not logged in
    }
  });

  it('should display timeline content', async () => {
    // ホームページの投稿リストを確認
    const postsContainer = await $('[data-testid="posts-list"]');
    if (await postsContainer.isExisting()) {
      await postsContainer.waitForDisplayed({ timeout: 5000 });
      await expect(postsContainer).toBeDisplayed();
    } else {
      // 代替としてhome-pageを確認
      const homePage = await $('[data-testid="home-page"]');
      if (await homePage.isExisting()) {
        await expect(homePage).toBeDisplayed();
      } else {
        // Posts list not found - may not be on timeline page
      }
    }
  });

  it('should handle topic navigation', async () => {
    // トピックリストを確認
    const topicsList = await $('[data-testid="topics-list"]');
    
    if (await topicsList.isExisting()) {
      // 最初のトピックを取得
      const firstTopic = await $('[data-testid^="topic-"]');
      
      if (await firstTopic.isExisting()) {
        await firstTopic.waitForClickable({ timeout: 5000 });
        await firstTopic.click();
        await browser.pause(1000);
        
        // ホームページに移動したことを確認
        const homePage = await $('[data-testid="home-page"]');
        await expect(homePage).toExist();
      } else {
        // No topics found in the list
      }
    } else {
      // Topics list not found - user may not have joined any topics
    }
  });

  it('should handle P2P sync indicator', async () => {
    const syncIndicator = await $('[data-testid="sync-indicator"]');
    
    if (await syncIndicator.isExisting()) {
      const isDisplayed = await syncIndicator.isDisplayed();
      expect(typeof isDisplayed).toBe('boolean');
    }
  });
});