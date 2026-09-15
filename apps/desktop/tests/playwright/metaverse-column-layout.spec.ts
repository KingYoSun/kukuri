import { expect, test, type Locator, type Page } from '@playwright/test';
import { DEVELOPER_MODE_STORAGE_KEY } from '../../src/lib/developerMode';
import en from '../../src/i18n/locales/en/metaverse.json' with { type: 'json' };
import ja from '../../src/i18n/locales/ja/metaverse.json' with { type: 'json' };
import zh from '../../src/i18n/locales/zh-CN/metaverse.json' with { type: 'json' };
import shellEn from '../../src/i18n/locales/en/shell.json' with { type: 'json' };
import shellJa from '../../src/i18n/locales/ja/shell.json' with { type: 'json' };
import shellZh from '../../src/i18n/locales/zh-CN/shell.json' with { type: 'json' };

async function openRoom(page: Page, locale = 'en', theme = 'dark', copy = en, discoveryOnly = false) {
  await page.addInitScript(key => localStorage.setItem(key, 'true'), DEVELOPER_MODE_STORAGE_KEY);
  await page.addInitScript(({ locale, theme }) => {
    localStorage.setItem('kukuri.desktop.locale', locale);
    localStorage.setItem('kukuri.desktop.theme', theme);
  }, { locale, theme });
  await page.setViewportSize({ width: 1283, height: 871 });
  await page.goto('/#/timeline?topic=kukuri%3Atopic%3Ageneral');
  await page.getByTestId('control-center-trigger').click();
  await page.locator('.shell-control-center').getByRole('button').filter({ hasText: 'Metaverse' }).last().click();
  const column = page.locator('.shell-column-surface').filter({ has: page.locator('.metaverse-panel') });
  await column.getByRole('button', { name: copy.create.action }).first().click();
  await column.getByPlaceholder(copy.create.titlePlaceholder).fill('Layout regression');
  if (discoveryOnly) return column;
  await column.getByRole('button', { name: copy.create.action }).last().click();
  await column.getByRole('button', { name: copy.management.startAndEnter }).click();
  await expect(column.locator('.metaverse-room-stage')).toBeVisible();
  return column;
}

async function span(page: Page, column: Locator, value: number) {
  await column.getByRole('button', { name: 'Open Metaverse menu' }).click();
  await page.getByRole('menuitemradio', { name: new RegExp(`^${value} spans?$`) }).click();
  await expect(column).toHaveAttribute('data-span', String(value));
}

async function headerVisible(column: Locator) {
  // No click/focus/scrollIntoView before measuring: Playwright would hide the bug.
  await expect.poll(() => column.evaluate(el => {
    const header = el.querySelector('.shell-column-header-actions')!.getBoundingClientRect();
    const canvas = el.closest('.shell-column-canvas')!.getBoundingClientRect();
    return header.left >= canvas.left && header.right <= canvas.right;
  })).toBe(true);
}

test('span expansion reveals the selected header before the next interaction', async ({ page }) => {
  const column = await openRoom(page);
  await span(page, column, 1);
  await span(page, column, 3);
  await headerVisible(column);
  await expect(column.getByRole('button', { name: 'Open Metaverse menu' })).toBeFocused();
});

test('narrow HUD and chat remain contained and keep their drafts across spans', async ({ page }) => {
  // Three span changes and three menu/chat round trips include a cold 3D startup.
  // Keep each assertion's deadline; allow the complete sequence on shared CI CPUs.
  test.slow();
  const column = await openRoom(page);
  await column.getByRole('button', { name: en.chat.open }).click();
  const chat = column.locator('.metaverse-chat-form input');
  await chat.fill('幅を変えても保持する');
  for (const value of [1, 2, 3]) {
    await span(page, column, value);
    if (process.env.KUKURI_LAYOUT_EVIDENCE) {
      await page.screenshot({ path: `${process.env.KUKURI_LAYOUT_EVIDENCE}-${value}.png` });
      console.info(await column.locator('.shell-column-body').evaluate(el => {
        const edge = el.getBoundingClientRect().right;
        return Array.from(el.querySelectorAll('*')).filter(child => child.getBoundingClientRect().right > edge + 1).slice(0, 15).map(child => ({ class: child.className, width: child.getBoundingClientRect().width }));
      }));
    }
    await expect.poll(() => column.locator('.shell-column-body').evaluate(el => el.scrollWidth - el.clientWidth)).toBeLessThanOrEqual(1);
    await expect(chat).toHaveValue('幅を変えても保持する');
    await expect(column.locator('.metaverse-room-hud')).toBeHidden();
    await column.getByRole('button', { name: en.menu.open }).click();
    await column.getByRole('button', { name: en.menu.categories.dome, exact: true }).click();
    await expect(column.locator('.metaverse-room-hud')).toBeVisible();
    await expect(chat).toHaveCount(0);
    await column.getByRole('button', { name: en.chat.open }).click();
    await expect(chat).toHaveValue('幅を変えても保持する');
  }
});

test('manual departure and inactive span changes do not pull the Canvas back', async ({ page }) => {
  const column = await openRoom(page);
  await span(page, column, 1);
  const canvas = page.locator('.shell-column-canvas');
  await canvas.hover();
  await page.mouse.wheel(-10000, 0);
  await expect.poll(() => canvas.evaluate(el => el.scrollLeft)).toBe(0);
  // External layout application changes width without clicking/activating the offscreen Column.
  await column.evaluate(el => { el.setAttribute('data-span', '3'); });
  await page.waitForTimeout(250);
  expect(await canvas.evaluate(el => el.scrollLeft)).toBe(0);
  await page.setViewportSize({ width: 1200, height: 871 });
  await page.waitForTimeout(250);
  expect(await canvas.evaluate(el => el.scrollLeft)).toBe(0);
});

test('wide forms use the adopted bounds and reflow without losing their values', async ({ page }) => {
  const column = await openRoom(page, 'en', 'light', en, true);
  const form = column.locator('.metaverse-create-form');
  await expect.poll(() => form.evaluate(el => el.getBoundingClientRect().width)).toBeLessThanOrEqual(640);
  for (const value of [1, 2, 3, 4]) {
    await span(page, column, value);
    await headerVisible(column);
    await expect(column.getByPlaceholder(en.create.titlePlaceholder)).toHaveValue('Layout regression');
    expect(await form.evaluate(el => el.scrollWidth - el.clientWidth)).toBeLessThanOrEqual(1);
  }
  // 200% equivalent reflow and the mobile breakpoint use the real Column width.
  for (const width of [760, 759, 640, 390, 320, 1283]) {
    await page.setViewportSize({ width, height: 871 });
    await headerVisible(column);
    expect(await page.evaluate(() => document.documentElement.scrollWidth - innerWidth)).toBeLessThanOrEqual(1);
    expect(await column.locator('.shell-column-body').evaluate(el => el.scrollWidth - el.clientWidth)).toBeLessThanOrEqual(1);
  }
});

for (const [locale, copy, shell] of [['en', en, shellEn], ['ja', ja, shellJa], ['zh-CN', zh, shellZh]] as const) {
  for (const theme of ['light', 'dark']) {
    for (const value of [1, 2, 3]) {
      test(`layout and operation continuity (${locale}, ${theme}, ${value} spans)`, async ({ page }) => {
        test.setTimeout(60_000);
        const column = await openRoom(page, locale, theme, copy);
        // Each case checks one resize and all its controls. Keep the full 3→1→2→3
        // sequence above; grouping all three hit-test passes approached 60s on CI.
        if (value === 3) {
          await column.getByRole('button', { name: shell.columnMenu.open.replace('{{title}}', 'Metaverse') }).click();
          await page.getByRole('menuitemradio').filter({ hasText: /^1\s/ }).click();
        }
        await column.getByRole('button', { name: copy.chat.open }).click();
        const chat = column.locator('.metaverse-chat-form input');
        const light = column.locator('.metaverse-dome-customization input[type=number]').first();
        await chat.fill('未送信 draft');
        await column.getByRole('button', { name: copy.menu.open }).click();
        await column.getByRole('button', { name: copy.menu.categories.dome, exact: true }).click();
        await light.fill('2.3');
        const sceneElement = await column.locator('.metaverse-room-stage').elementHandle();
        await page.evaluate(() => {
          const api = window.__KUKURI_DESKTOP__ as unknown as Record<string, (...args: unknown[]) => unknown>;
          const probe = window as unknown as { layoutMutations: string[] };
          probe.layoutMutations = [];
          for (const key of ['createMetaverseRoom', 'updateMetaverseRoom', 'startOwnerDomeHosting', 'closeDomeHosting', 'deleteDome', 'publishMetaverseRoomEvent', 'commitDomeLayout', 'createDomeConnectionProposal']) {
            const original = api[key];
            api[key] = (...args) => {
              // Existing peer presence continues independently of layout.
              if (key !== 'publishMetaverseRoomEvent' || (args[4] as { type: string }).type !== 'presence_join') probe.layoutMutations.push(key);
              return original.apply(api, args);
            };
          }
        });
        await column.getByRole('button', { name: shell.columnMenu.open.replace('{{title}}', 'Metaverse') }).click();
        await page.getByRole('menuitemradio').filter({ hasText: new RegExp(`^${value}\\s`) }).click();
        await headerVisible(column);
        await expect(light).toHaveValue('2.3');
        await column.getByRole('button', { name: copy.chat.open }).click();
        await expect(chat).toHaveValue('未送信 draft');
        await expect.poll(() => column.locator('.shell-column-body').evaluate(el => el.scrollWidth - el.clientWidth)).toBeLessThanOrEqual(1);
        for (const target of [column.getByRole('button', { name: copy.chat.send, exact: true }), column.getByRole('button', { name: copy.chat.hide, exact: true }), column.getByRole('button', { name: copy.hud.leave, exact: true })]) {
          await target.scrollIntoViewIfNeeded();
          expect(await target.evaluate(el => {
            const r = el.getBoundingClientRect();
            const hit = document.elementFromPoint(r.x + r.width / 2, r.y + r.height / 2);
            return Boolean(hit && el.contains(hit));
          })).toBe(true);
        }
        // Layout must not submit the draft or save customization.
        await expect(chat).toHaveValue('未送信 draft');
        await expect(light).toHaveValue('2.3');
        expect(await sceneElement!.evaluate(el => el.isConnected)).toBe(true);
        expect(await page.evaluate(() => (window as unknown as { layoutMutations: string[] }).layoutMutations)).toEqual([]);
      });
    }
  }
}

