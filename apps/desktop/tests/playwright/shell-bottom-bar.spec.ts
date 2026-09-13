import { expect, test, type Page } from '@playwright/test';

// 幅狭(759px以下)の下部ボタン: 投稿ボタンは右寄せ、アバター・Control Center・
// フィードバックの cluster は左寄せで、4つの高さと下辺を揃える。
async function bottomBarGeometry(page: Page) {
  const post = page.locator('.shell-column-primary-action').first();
  await expect(post).toBeVisible();
  await expect(page.getByTestId('tester-feedback-trigger')).toBeVisible();
  return post.evaluate((postButton) => {
    const rect = (selector: string) =>
      document.querySelector(selector)!.getBoundingClientRect();
    const post = postButton.getBoundingClientRect();
    const cluster = rect('.shell-control-cluster');
    const avatar = rect('[data-testid="account-menu-trigger"]');
    const controlCenter = rect('[data-testid="control-center-trigger"]');
    const feedback = rect('[data-testid="tester-feedback-trigger"]');
    const footer = rect('.shell-column-footer');
    return {
      viewportWidth: window.innerWidth,
      post: { left: post.left, right: post.right, bottom: post.bottom, height: post.height },
      cluster: { left: cluster.left, right: cluster.right },
      heights: [avatar.height, controlCenter.height, feedback.height],
      bottoms: [avatar.bottom, controlCenter.bottom, feedback.bottom],
      footerRight: footer.right,
      footerPaddingRight: parseFloat(getComputedStyle(document.querySelector('.shell-column-footer')!).paddingRight),
    };
  });
}

test('narrow bottom bar aligns the cluster left and the post button right', async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await page.goto('/');
  const geometry = await bottomBarGeometry(page);

  // 高さは投稿ボタンに合わせる。
  for (const height of geometry.heights) {
    expect(Math.abs(height - geometry.post.height)).toBeLessThanOrEqual(1);
  }
  // 下辺も揃える。
  for (const bottom of geometry.bottoms) {
    expect(Math.abs(bottom - geometry.post.bottom)).toBeLessThanOrEqual(1);
  }
  // cluster は左寄せ、投稿ボタンは右寄せで重ならない。
  expect(geometry.cluster.left).toBeLessThan(geometry.viewportWidth / 4);
  expect(geometry.cluster.right).toBeLessThanOrEqual(geometry.post.left);
  expect(Math.abs(geometry.post.right - (geometry.footerRight - geometry.footerPaddingRight))).toBeLessThanOrEqual(1);
});

test('desktop keeps the cluster at the bottom left', async ({ page }) => {
  await page.setViewportSize({ width: 1280, height: 800 });
  await page.goto('/');
  const geometry = await bottomBarGeometry(page);
  expect(geometry.cluster.left).toBeLessThan(geometry.viewportWidth / 4);
});
