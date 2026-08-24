import { expect, test, type Page } from '@playwright/test';

import { DEVELOPER_MODE_STORAGE_KEY } from '../../src/lib/developerMode';

// Issue #748 設計判断 12 / 13 の browser 固定:
// - Metaverse stage(gesture owner)内の pointer / touch は Column 切替を奪わない
// - 画面外の Stream / Metaverse Column は suspended へ縮退し、戻ると復帰する(session は維持)
// Live / Metaverse Column は developer mode でのみ Control Center から追加できる。
test.beforeEach(async ({ page }) => {
  await page.addInitScript((key) => {
    window.localStorage.setItem(key, 'true');
  }, DEVELOPER_MODE_STORAGE_KEY);
});

function activeColumn(page: Page, title: string) {
  return page.getByRole('region', { name: new RegExp(`^${title} Column,.*Active,`) });
}

async function openControlCenter(page: Page) {
  const controlCenter = page.getByRole('complementary', { name: 'Control Center' });
  if (!(await controlCenter.isVisible().catch(() => false))) {
    await page.getByTestId('control-center-trigger').click();
  }
  await expect(controlCenter).toBeVisible();
  return controlCenter;
}

async function addColumn(page: Page, name: 'Add Live Column' | 'Add Metaverse Column') {
  const controlCenter = await openControlCenter(page);
  await controlCenter.getByRole('button', { name }).click();
}

test('Stream and Metaverse fullscreen return to the same Column workspace state', async ({
  page,
}) => {
  await page.setViewportSize({ width: 1920, height: 1080 });
  await page.goto('/#/timeline?topic=kukuri%3Atopic%3Ademo');
  await addColumn(page, 'Add Live Column');
  await addColumn(page, 'Add Metaverse Column');
  const { metaverse, stage } = await createMetaverseRoom(page);
  const live = page.getByRole('region', { name: /^Live Column/ });
  const before = await page.locator('[data-column-id]').evaluateAll((columns) =>
    columns.map((column) => ({
      id: (column as HTMLElement).dataset.columnId,
      span: (column as HTMLElement).dataset.span,
    }))
  );

  await live.scrollIntoViewIfNeeded();
  await page.waitForTimeout(150);
  await live.getByRole('button', { name: 'Open Live menu' }).click();
  await page.getByRole('menuitem', { name: 'Enter Live fullscreen' }).click();
  await expect.poll(() => live.evaluate((element) => document.fullscreenElement === element)).toBe(true);
  // Headless Chromium は browser chrome が所有する Escape を合成keyから実行しないため、
  // 標準 Escape と同じ exitFullscreen -> fullscreenchange 経路を直接発火する。
  await page.evaluate(() => document.exitFullscreen());
  await expect.poll(() => page.evaluate(() => document.fullscreenElement === null)).toBe(true);
  await expect(live).toBeFocused();

  await metaverse.getByRole('button', { name: 'Open Metaverse menu' }).click();
  await page.getByRole('menuitem', { name: 'Enter Metaverse fullscreen' }).click();
  await expect.poll(() => metaverse.evaluate((element) => document.fullscreenElement === element)).toBe(true);
  await metaverse.getByRole('button', { name: 'Open Metaverse menu' }).click();
  await page.getByRole('menuitem', { name: 'Exit Metaverse fullscreen' }).click();
  await expect.poll(() => page.evaluate(() => document.fullscreenElement === null)).toBe(true);

  expect(
    await page.locator('[data-column-id]').evaluateAll((columns) =>
      columns.map((column) => ({
        id: (column as HTMLElement).dataset.columnId,
        span: (column as HTMLElement).dataset.span,
      }))
    )
  ).toEqual(before);
  await expect(stage).toHaveCount(1);
});

// Metaverse Column 内で room を作成して stage(data-column-gesture-owner)を表示する。
async function createMetaverseRoom(page: Page) {
  const metaverse = page.getByRole('region', { name: /^Metaverse Column/ });
  await metaverse.getByRole('button', { name: 'Create metaverse room' }).first().click();
  await metaverse.getByPlaceholder('Atrium').fill('Gesture lab');
  await metaverse.getByRole('button', { name: 'Create metaverse room' }).last().click();
  const stage = metaverse.locator('[data-column-gesture-owner="metaverse"]');
  await expect(stage).toBeVisible();
  return { metaverse, stage };
}

test('desktop Metaverse stage keeps pointer ownership and does not steal Column activation', async ({
  page,
}) => {
  await page.setViewportSize({ width: 1920, height: 1080 });
  await page.goto('/#/timeline?topic=kukuri%3Atopic%3Ademo');
  await addColumn(page, 'Add Metaverse Column');
  const { metaverse, stage } = await createMetaverseRoom(page);
  await expect(metaverse).toHaveAttribute('aria-current', 'true');

  // Timeline header(非 interactive 領域)で Timeline を active にする。
  const timeline = page.getByRole('region', { name: /^Timeline Column/ });
  await timeline.locator('.shell-column-header h2').first().dispatchEvent('pointerdown');
  await expect(activeColumn(page, 'Timeline')).toBeVisible();

  // stage 内の pointerdown は scene 操作として扱われ、Column activation を起こさない。
  const box = (await stage.boundingBox())!;
  await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2);
  await page.mouse.down();
  await page.mouse.move(box.x + box.width / 2 + 120, box.y + box.height / 2 + 40, { steps: 6 });
  await page.mouse.up();
  await expect(activeColumn(page, 'Timeline')).toBeVisible();
  await expect(metaverse).not.toHaveAttribute('aria-current', 'true');

  // Metaverse Column 自身の header では通常どおり activate できる。
  await metaverse.locator('.shell-column-header h2').first().dispatchEvent('pointerdown');
  await expect(activeColumn(page, 'Metaverse')).toBeVisible();
});

test.describe('mobile touch ownership', () => {
  test.use({ hasTouch: true, isMobile: true, viewport: { width: 390, height: 844 } });

  test('mobile Metaverse stage owns horizontal touch while Column header swipe still pages', async ({
    page,
  }) => {
    await page.goto('/#/timeline?topic=kukuri%3Atopic%3Ademo');
    await addColumn(page, 'Add Metaverse Column');
    const { metaverse, stage } = await createMetaverseRoom(page);
    await expect(activeColumn(page, 'Metaverse')).toBeVisible();
    await expect(page.getByText('2 / 2')).toBeVisible();

    const touchAction = await stage.evaluate((element) => getComputedStyle(element).touchAction);
    expect(touchAction).toBe('none');

    const canvas = page.locator('.shell-column-canvas');
    const startScrollLeft = await canvas.evaluate((element) => element.scrollLeft);
    expect(startScrollLeft).toBeGreaterThan(0);

    // stage 上の右スワイプ(前の Column へ戻る方向)は Column paging を起こさない。
    const client = await page.context().newCDPSession(page);
    const stageBox = (await stage.boundingBox())!;
    await swipe(client, stageBox.x + 40, stageBox.y + stageBox.height / 2, 300, 0);
    await page.waitForTimeout(400);
    expect(await canvas.evaluate((element) => element.scrollLeft)).toBe(startScrollLeft);
    await expect(activeColumn(page, 'Metaverse')).toBeVisible();

    // Column header 上の同じスワイプは Column paging を起こし Timeline が active になる。
    const headerBox = (await metaverse.locator('.shell-column-header').boundingBox())!;
    await swipe(client, headerBox.x + 40, headerBox.y + headerBox.height / 2, 300, 0);
    await expect(activeColumn(page, 'Timeline')).toBeVisible();
    expect(await canvas.evaluate((element) => element.scrollLeft)).toBeLessThan(startScrollLeft);
  });
});

test('offscreen Metaverse and Live Columns suspend rendering and resume without losing the session', async ({
  page,
}) => {
  // 1280px(Canvas 内幅 1232px)では Timeline(440) + Live(896) で Canvas が埋まり、
  // 3 本目の Metaverse(1352)は Canvas 先頭では完全に画面外、末尾では Live が完全に画面外になる。
  await page.setViewportSize({ width: 1280, height: 900 });
  await page.goto('/#/timeline?topic=kukuri%3Atopic%3Ademo');
  await addColumn(page, 'Add Live Column');
  await addColumn(page, 'Add Metaverse Column');
  const { metaverse, stage } = await createMetaverseRoom(page);
  const live = page.getByRole('region', { name: /^Live Column/ });
  await expect(live).toHaveAccessibleName(/Column 2 of 3/);
  await expect(metaverse).toHaveAccessibleName(/Column 3 of 3/);

  // 追加直後は Metaverse が active で画面内にあり、縮退していない。
  await expect(metaverse).not.toHaveAttribute('data-runtime-suspended', 'true');
  await expect(stage.locator('[data-render-suspended="true"]')).toHaveCount(0);
  // この時点で Live は画面外(Canvas 末尾までスクロール済み)のため縮退している。
  await expect(live).toHaveAttribute('data-runtime-suspended', 'true');

  // Timeline を active にして Canvas 先頭へ戻すと Metaverse は画面外になり縮退、Live は部分表示で復帰する。
  const canvas = page.locator('.shell-column-canvas');
  const timeline = page.getByRole('region', { name: /^Timeline Column/ });
  await timeline.locator('.shell-column-header h2').first().dispatchEvent('pointerdown');
  await canvas.evaluate((element) => {
    element.scrollLeft = 0;
  });
  await expect(metaverse).toHaveAttribute('data-runtime-suspended', 'true');
  await expect(stage.locator('[data-render-suspended="true"]')).toHaveCount(1);
  await expect(live).not.toHaveAttribute('data-runtime-suspended', 'true');
  // suspended 中も room session(stage)は mount されたまま維持される。
  await expect(stage).toHaveCount(1);
  await expect(stage.locator('canvas')).toHaveCount(1);

  // 画面内へ戻すと縮退が解除され、代わりに画面外へ出た Live が縮退する。
  await metaverse.scrollIntoViewIfNeeded();
  await expect(metaverse).not.toHaveAttribute('data-runtime-suspended', 'true');
  await expect(stage.locator('[data-render-suspended="true"]')).toHaveCount(0);
  await expect(live).toHaveAttribute('data-runtime-suspended', 'true');
});

// CDP で touch swipe を合成する(Playwright は touch drag API を持たない)。
async function swipe(
  client: Awaited<ReturnType<Page['context']>['newCDPSession']> extends (...args: never[]) => infer R
    ? Awaited<R>
    : never,
  x: number,
  y: number,
  dx: number,
  dy: number
) {
  await client.send('Input.dispatchTouchEvent', {
    type: 'touchStart',
    touchPoints: [{ x, y }],
  });
  const steps = 8;
  for (let step = 1; step <= steps; step += 1) {
    await client.send('Input.dispatchTouchEvent', {
      type: 'touchMove',
      touchPoints: [{ x: x + (dx * step) / steps, y: y + (dy * step) / steps }],
    });
  }
  await client.send('Input.dispatchTouchEvent', { type: 'touchEnd', touchPoints: [] });
}
