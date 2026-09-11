import { expect, test, type Locator, type Page } from '@playwright/test';

import { DEVELOPER_MODE_STORAGE_KEY } from '../../src/lib/developerMode';

// #967: 通常設定と Control Center からバックアップ／復元の既存 UI へ到達できること、
// 鍵だけの移行(Account)と区別できること、Tauri 既定 window(1280x840)で設定 nav の
// 全 section が nav の可視域に収まることを、実 pointer / keyboard で確認する。
const DESKTOP_LOCALE_STORAGE_KEY = 'kukuri.desktop.locale';
const DESKTOP_THEME_STORAGE_KEY = 'kukuri.desktop.theme';
// apps/desktop/src-tauri/tauri.conf.json の既定 window サイズ。
const TAURI_DEFAULT_WINDOW = { width: 1280, height: 840 } as const;

const COPY = {
  ja: {
    settings: '設定',
    controlCenter: 'コントロールセンター',
    backupSection: 'バックアップと復元',
    accountSection: 'アカウント',
    createHeading: 'バックアップを作成',
    restoreHeading: 'バックアップから復元',
    chooseFile: 'バックアップファイルを選択',
    exportHeading: 'アカウント鍵のエクスポート',
    openBackup: 'バックアップと復元を開く',
    openAccount: 'アカウント鍵の設定を開く',
  },
  en: {
    settings: 'Settings',
    controlCenter: 'Control Center',
    backupSection: 'Backup & restore',
    accountSection: 'Account',
    createHeading: 'Create backup',
    restoreHeading: 'Restore from backup',
    chooseFile: 'Choose backup file',
    exportHeading: 'Export account key',
    openBackup: 'Open backup & restore',
    openAccount: 'Open account key settings',
  },
} as const;

async function seed(page: Page, locale: keyof typeof COPY, theme: 'dark' | 'light') {
  await page.addInitScript(
    ({ locale, theme, localeKey, themeKey, developerKey }) => {
      window.localStorage.setItem(localeKey, locale);
      window.localStorage.setItem(themeKey, theme);
      window.localStorage.setItem(developerKey, 'false');
    },
    {
      locale,
      theme,
      localeKey: DESKTOP_LOCALE_STORAGE_KEY,
      themeKey: DESKTOP_THEME_STORAGE_KEY,
      developerKey: DEVELOPER_MODE_STORAGE_KEY,
    }
  );
}

// nav item がすべて nav の可視域(scroll なし)に入っているかを返す。
async function hiddenNavItems(settings: Locator) {
  return settings.evaluate((drawer) => {
    const nav = drawer.querySelector<HTMLElement>('.shell-settings-nav');
    if (!nav) return ['missing nav'];
    const navRect = nav.getBoundingClientRect();
    return [...drawer.querySelectorAll<HTMLElement>('.shell-settings-nav-item')]
      .filter((item) => {
        const rect = item.getBoundingClientRect();
        return rect.top < navRect.top - 1 || rect.bottom > navRect.bottom + 1;
      })
      .map((item) => item.textContent ?? '');
  });
}

async function expectNoHorizontalOverflow(settings: Locator) {
  const content = settings.locator('.shell-settings-content');
  expect(await content.evaluate((el) => el.scrollWidth <= el.clientWidth + 1)).toBe(true);
}

test('ja dark: every settings section stays visible in the navigation at the Tauri default window size', async ({
  page,
}) => {
  await seed(page, 'ja', 'dark');
  await page.setViewportSize(TAURI_DEFAULT_WINDOW);
  await page.goto('/#/timeline?topic=kukuri%3Atopic%3Ageneral&settings=connectivity');
  const settings = page.getByRole('dialog', { name: COPY.ja.settings, exact: true });
  await expect(settings).toBeVisible();
  await expect(settings.getByTestId('settings-section-backup')).toBeVisible();
  expect(await hiddenNavItems(settings)).toEqual([]);

  // 768px の短い window でも末尾の section が nav の可視域に残る。
  await page.setViewportSize({ width: 1280, height: 768 });
  expect(await hiddenNavItems(settings)).toEqual([]);
});

for (const { locale, theme, width, height } of [
  { locale: 'ja', theme: 'dark', ...TAURI_DEFAULT_WINDOW },
  { locale: 'en', theme: 'light', width: 390, height: 844 },
] as const) {
  const copy = COPY[locale];

  test(`${locale} ${theme} Control Center opens backup & restore and the sections cross-link`, async ({
    page,
  }) => {
    await seed(page, locale, theme);
    await page.setViewportSize({ width, height });
    await page.goto('/#/timeline?topic=kukuri%3Atopic%3Ageneral');
    await page.getByTestId('control-center-trigger').click();
    const controlCenter = page.getByRole('complementary', { name: copy.controlCenter });
    const entry = controlCenter.getByRole('button', { name: copy.backupSection, exact: true });
    await expect(entry).toBeVisible();
    await entry.focus();
    await page.keyboard.press('Enter');

    const settings = page.getByRole('dialog', { name: copy.settings, exact: true });
    await expect(settings).toBeVisible();
    await expect(settings.getByTestId('settings-section-backup')).toHaveAttribute('aria-current', 'location');
    await expect(page).toHaveURL(/settings=backup/);
    await expect(settings.getByRole('heading', { name: copy.createHeading, exact: true })).toBeVisible();
    await expect(settings.getByRole('button', { name: copy.chooseFile, exact: true })).toBeVisible();
    // 復元の見出しまで本文 scroll で到達できる(狭幅では下にある)。
    await settings.getByRole('heading', { name: copy.restoreHeading, exact: true }).scrollIntoViewIfNeeded();
    await expect(settings.getByRole('heading', { name: copy.restoreHeading, exact: true })).toBeVisible();
    await expectNoHorizontalOverflow(settings);

    // 鍵だけの移行へ移り、戻る。移動先の nav item に focus が乗る。
    await settings.getByRole('button', { name: copy.openAccount, exact: true }).click();
    await expect(settings.getByTestId('settings-section-account')).toHaveAttribute('aria-current', 'location');
    await expect(settings.getByTestId('settings-section-account')).toBeFocused();
    await expect(page).toHaveURL(/settings=account/);
    await expect(settings.getByRole('heading', { name: copy.exportHeading, exact: true })).toBeVisible();
    await expectNoHorizontalOverflow(settings);
    await settings.getByRole('button', { name: copy.openBackup, exact: true }).click();
    await expect(settings.getByTestId('settings-section-backup')).toHaveAttribute('aria-current', 'location');
    await expect(settings.getByTestId('settings-section-backup')).toBeFocused();
    await expect(page).toHaveURL(/settings=backup/);

    // keyboard で nav を移動し、Escape で閉じると URL から settings が消える。
    await page.keyboard.press('Escape');
    await expect(settings).toBeHidden();
    await expect(page).not.toHaveURL(/settings=/);
  });

  test(`${locale} ${theme} backup deep link opens the section without developer mode`, async ({ page }) => {
    await seed(page, locale, theme);
    await page.setViewportSize({ width, height });
    await page.goto('/#/timeline?topic=kukuri%3Atopic%3Ageneral&settings=backup');
    const settings = page.getByRole('dialog', { name: copy.settings, exact: true });
    await expect(settings).toBeVisible();
    await expect(settings.getByTestId('settings-section-backup')).toHaveAttribute('aria-current', 'location');
    await expect(settings.getByRole('heading', { name: copy.createHeading, exact: true })).toBeVisible();
    for (const viewport of [1280, 700, 390]) {
      await page.setViewportSize({ width: viewport, height });
      await expectNoHorizontalOverflow(settings);
    }
  });
}
