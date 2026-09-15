import { expect, test, type Page } from '@playwright/test';
import { DEVELOPER_MODE_STORAGE_KEY } from '../../src/lib/developerMode';
test.use({ screenshot: 'only-on-failure' });

async function enter(page: Page, width: number) {
  await page.addInitScript(key => localStorage.setItem(key, 'true'), DEVELOPER_MODE_STORAGE_KEY);
  await page.setViewportSize({ width, height: 871 });
  await page.goto('/#/timeline?topic=kukuri%3Atopic%3Ageneral');
  await page.getByTestId('control-center-trigger').click();
  await page.getByRole('button', { name: 'Add Metaverse Column' }).click();
  const column = page.getByRole('region', { name: /^Metaverse Column/ });
  await column.getByRole('button', { name: 'Create metaverse room' }).first().click();
  await column.getByPlaceholder('Atrium').fill('Connections review');
  await column.getByRole('button', { name: 'Create metaverse room' }).last().click();
  await column.getByRole('button', { name: 'Start hosting and enter' }).click();
  await expect(column.locator('.metaverse-room-stage')).toBeFocused();
  return column;
}

for (const width of [1283, 390]) test(`connection map keyboard and pointer at ${width}px`, async ({ page }) => {
  test.slow();
  const column = await enter(page, width); const stage = column.locator('.metaverse-room-stage');
  await page.evaluate(() => {
    const api = window.__KUKURI_DESKTOP__ as unknown as Record<string, (...args: unknown[]) => unknown>;
    const probe = window as unknown as { connectionWrites: string[] }; probe.connectionWrites = [];
    for (const name of ['createDomeConnectionProposal', 'acceptDomeConnectionProposal', 'withdrawDomeConnectionProposal', 'revokeDomeConnection']) {
      const original = api[name]; api[name] = (...args) => { probe.connectionWrites.push(name); return original.apply(api, args); };
    }
  });
  await stage.getByRole('button', { name: 'Menu (Tab)' }).click();
  await stage.getByRole('button', { name: 'Connections', exact: true }).click();
  const compass = stage.getByRole('group', { name: 'Choose a direction' });
  await expect(compass).toBeVisible();
  await compass.getByRole('button', { name: /^North/ }).click();
  await page.keyboard.press('ArrowRight');
  await expect(compass.getByRole('button', { name: /^East/ })).toBeFocused();
  await expect(compass.getByRole('button', { name: /^East/ })).toHaveAttribute('aria-pressed', 'true');
  await expect(stage.getByRole('img', { name: 'Confirmed connection map' })).toBeVisible();
  for (const button of await compass.getByRole('button').all()) {
    // Details deliberately scroll inside the existing HUD; each control must remain reachable.
    await button.scrollIntoViewIfNeeded();
    await expect(button).toBeInViewport(); const rect = await button.boundingBox();
    expect(rect!.width).toBeGreaterThanOrEqual(24); expect(rect!.height).toBeGreaterThanOrEqual(44);
  }
  await page.keyboard.press('Escape'); await expect(stage).toBeFocused();
  await page.keyboard.press('Tab'); await page.keyboard.press('Enter');
  await expect(compass.getByRole('button', { name: /^East/ })).toHaveAttribute('aria-pressed', 'true');
  await stage.getByRole('tab', { name: 'Avatar', exact: true }).click();
  await stage.getByRole('tab', { name: 'Connections', exact: true }).click();
  await expect(compass.getByRole('button', { name: /^East/ })).toHaveAttribute('aria-pressed', 'true');
  expect(await page.evaluate(() => (window as unknown as { connectionWrites: string[] }).connectionWrites)).toEqual([]);
  expect(await column.evaluate(el => el.scrollWidth - el.clientWidth)).toBeLessThanOrEqual(1);
});
