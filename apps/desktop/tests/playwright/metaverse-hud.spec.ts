import { expect, test, type Page } from '@playwright/test';
import { DEVELOPER_MODE_STORAGE_KEY } from '../../src/lib/developerMode';

async function enter(page: Page, width = 1283) {
  await page.addInitScript(key => localStorage.setItem(key, 'true'), DEVELOPER_MODE_STORAGE_KEY);
  await page.setViewportSize({ width, height: 871 });
  await page.goto('/#/timeline?topic=kukuri%3Atopic%3Ageneral');
  await page.getByTestId('control-center-trigger').click();
  await page.getByRole('button', { name: 'Add Metaverse Column' }).click();
  const column = page.getByRole('region', { name: /^Metaverse Column/ });
  await column.getByRole('button', { name: 'Create metaverse room' }).first().click();
  await column.getByPlaceholder('Atrium').fill('HUD regression');
  await column.getByRole('button', { name: 'Create metaverse room' }).last().click();
  await column.getByRole('button', { name: 'Start hosting and enter' }).click();
  await expect(column.locator('.metaverse-room-stage')).toBeFocused();
  return column;
}

test('six categories, keyboard and IME preserve drafts without dispatching domain actions', async ({ page }) => {
  const column = await enter(page);
  const stage = column.locator('.metaverse-room-stage');
  await expect(column.locator('.metaverse-room-hud')).toBeHidden();
  await expect(column.locator('.metaverse-chat-form')).toHaveCount(0);
  await page.evaluate(() => {
    const api = window.__KUKURI_DESKTOP__ as unknown as Record<string, (...args: unknown[]) => unknown>;
    const probe = window as unknown as { hudWrites: string[] };
    probe.hudWrites = [];
    for (const key of ['updateMetaverseRoom', 'importMetaverseRoomAsset', 'startOwnerDomeHosting', 'closeDomeHosting', 'delegateDomeHosting', 'deleteDome', 'commitDomeLayout', 'createDomeConnectionProposal', 'acceptDomeConnectionProposal', 'withdrawDomeConnectionProposal', 'revokeDomeConnection']) {
      const original = api[key];
      api[key] = (...args) => { probe.hudWrites.push(key); return original.apply(api, args); };
    }
    const publish = api.publishMetaverseRoomEvent;
    api.publishMetaverseRoomEvent = (...args) => {
      if ((args[4] as { type: string }).type === 'chat_message') probe.hudWrites.push('chat');
      return publish.apply(api, args);
    };
  });
  await page.keyboard.press('Tab');
  await expect(stage.getByRole('button', { name: 'Dome settings', exact: true })).toBeFocused();
  await page.keyboard.press('ArrowRight');
  await expect(stage.getByRole('button', { name: 'Hosting', exact: true })).toBeFocused();
  await page.keyboard.press('Enter');
  await expect(stage.getByRole('tab', { name: 'Hosting', exact: true })).toBeFocused();
  expect((await stage.locator('.metaverse-category-details').boundingBox())!.width).toBeGreaterThanOrEqual(600);
  await expect(stage.getByText(/Lease epoch/)).toBeHidden();
  for (const name of ['Dome settings', 'Connections', 'Avatar', 'Shared objects', 'Diagnostics', 'Hosting']) {
    await stage.getByRole('tab', { name, exact: true }).click();
    await expect(stage.getByRole('tab', { name, exact: true })).toHaveAttribute('aria-selected', 'true');
    await expect(stage.getByRole('tabpanel')).toHaveCount(1);
  }
  await stage.getByRole('tab', { name: 'Dome settings', exact: true }).click();
  await stage.getByLabel('Wall material', { exact: true }).selectOption('wood');
  await stage.getByRole('tab', { name: 'Avatar', exact: true }).click();
  await page.keyboard.press('Escape');
  await expect(stage).toBeFocused();
  await page.keyboard.press('Tab');
  await expect(stage.getByRole('button', { name: 'Avatar', exact: true })).toBeFocused();
  await page.keyboard.press('Home');
  await page.keyboard.press('Enter');
  await expect(stage.getByLabel('Wall material', { exact: true })).toHaveValue('wood');
  await page.keyboard.press('Escape');
  await page.keyboard.press('Enter');
  const chat = stage.getByRole('textbox', { name: 'Room chat message' });
  await expect(chat).toBeFocused();
  await chat.fill('未送信の下書き');
  await chat.dispatchEvent('compositionstart');
  await page.keyboard.press('Enter');
  await chat.dispatchEvent('keydown', { key: 'Escape', isComposing: true });
  await expect(chat).toBeVisible();
  await chat.dispatchEvent('compositionend');
  await page.keyboard.press('Tab');
  await expect(stage.getByRole('button', { name: 'Send', exact: true })).toBeFocused();
  await page.keyboard.press('Escape');
  await page.keyboard.press('Enter');
  await expect(chat).toHaveValue('未送信の下書き');
  expect(await page.evaluate(() => (window as unknown as { hudWrites: string[] }).hudWrites)).toEqual([]);
});

test('narrow category circle and details keep all pointer targets inside the Dome', async ({ page }) => {
  const column = await enter(page, 390);
  const stage = column.locator('.metaverse-room-stage');
  await stage.getByRole('button', { name: 'Menu (Tab)' }).click();
  const menu = stage.getByRole('region', { name: 'Dome menu' });
  await page.screenshot({ path: '../../.codex/plans/issue-1024-narrow-menu.png' });
  await expect(menu).toBeInViewport();
  for (const button of await menu.getByRole('button').all()) {
    await expect(button).toBeInViewport();
    const rect = await button.boundingBox();
    expect(rect!.width).toBeGreaterThanOrEqual(24);
    expect(rect!.height).toBeGreaterThanOrEqual(44);
    expect(await button.evaluate(el => {
      const r = el.getBoundingClientRect();
      return el.contains(document.elementFromPoint(r.x + r.width / 2, r.y + r.height / 2));
    })).toBe(true);
  }
  await menu.getByRole('button', { name: 'Dome settings', exact: true }).click();
  await stage.getByRole('tab', { name: 'Diagnostics', exact: true }).click();
  await stage.getByRole('button', { name: 'Close', exact: true }).click();
  expect(await page.evaluate(() => document.documentElement.scrollWidth - innerWidth)).toBeLessThanOrEqual(1);
  await expect(stage).toBeFocused();
});
