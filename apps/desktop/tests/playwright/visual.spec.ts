import { expect, test, type Page } from '@playwright/test';

import { DEVELOPER_MODE_STORAGE_KEY } from '../../src/lib/developerMode';
import { DESKTOP_THEME_STORAGE_KEY, type DesktopTheme } from '../../src/lib/theme';

// src/i18n/index.ts の DESKTOP_LOCALE_STORAGE_KEY と一致（実読で確認）。
// i18n/index.ts を import すると全 locale JSON ロード + react-i18next.init の副作用が
// テスト評価時に走るため、副作用のない軽量な theme.ts だけを import し、locale キーは
// リテラルで持つ。
const DESKTOP_LOCALE_STORAGE_KEY = 'kukuri.desktop.locale';

// 視覚回帰スモーク（WP-S7）。到達操作は既存 E2E（shell.smoke / hash-routing /
// extended-flow）の deep link + testid パターンを再利用する。付録 B の安定化前処理を
// 適用し、決定的な名前で toHaveScreenshot を撮る。
//
// Windows ローカルでは playwright.config の ignoreSnapshots: !process.env.CI により
// 比較が skip され、本 spec は「到達操作の smoke」として流れる（baseline PNG は生成しない）。
// baseline は Linux CI（CI=1）でのみ --update-snapshots で生成する。

const WIDE = { width: 1400, height: 980 } as const;
const NARROW = { width: 700, height: 980 } as const;

const LOCALE_EN = 'en';

/**
 * テーマ / ロケールを localStorage へ事前注入する（UI 操作でのテーマ切替は
 * トランジション残りのリスクがあるため addInitScript で注入する）。goto より前に呼ぶ。
 */
async function seedAppearance(page: Page, theme: DesktopTheme): Promise<void> {
  await page.addInitScript(
    ({ themeKey, themeValue, localeKey, localeValue, developerModeKey }) => {
      window.localStorage.setItem(themeKey, themeValue);
      window.localStorage.setItem(localeKey, localeValue);
      // 既存 baseline は Live/Game タブとステータスバッジを含むため developer mode を有効化する。
      window.localStorage.setItem(developerModeKey, 'true');
    },
    {
      themeKey: DESKTOP_THEME_STORAGE_KEY,
      themeValue: theme,
      localeKey: DESKTOP_LOCALE_STORAGE_KEY,
      localeValue: LOCALE_EN,
      developerModeKey: DEVELOPER_MODE_STORAGE_KEY,
    }
  );
}

/**
 * :focus-visible ring（base.css の 2px）や caret の写り込みを避けるため、撮影前に
 * アクティブ要素を blur し、body へフォーカスを移す。
 */
async function clearFocus(page: Page): Promise<void> {
  await page.evaluate(() => {
    const active = document.activeElement as HTMLElement | null;
    active?.blur?.();
    if (document.body) {
      document.body.focus();
    }
    window.scrollTo(0, 0);
  });
}

/**
 * data-theme が反映され、対象要素が visible になるまで待ってから撮影前処理を行う。
 */
async function settleForShot(page: Page, theme: DesktopTheme): Promise<void> {
  await expect(page.locator('html')).toHaveAttribute('data-theme', theme);
  await clearFocus(page);
}

async function openComposerDialog(page: Page): Promise<void> {
  await activeColumn(page, 'Timeline').getByRole('button', { name: /^Post to / }).click();
  await expect(page.getByPlaceholder('Write a post')).toBeVisible();
}

function activeColumn(page: Page, title: string) {
  return page.getByRole('region', {
    name: new RegExp(`^${title} Column,.*Active,`),
  });
}

test.describe('visual regression smoke', () => {
  // 1: fresh default wide dark（5 Columnの初期構成とシェル全体の基準）
  test('fresh default wide dark', async ({ page }) => {
    await seedAppearance(page, 'dark');
    await page.setViewportSize(WIDE);
    await page.goto('/');
    await expect(page.getByText('browser mock peer post')).toBeVisible();
    await settleForShot(page, 'dark');
    await expect(page).toHaveScreenshot('timeline-wide-dark.png');
  });

  // 2: fresh default wide light（トークン退行）
  test('fresh default wide light', async ({ page }) => {
    await seedAppearance(page, 'light');
    await page.setViewportSize(WIDE);
    await page.goto('/');
    await expect(page.getByText('browser mock peer post')).toBeVisible();
    await settleForShot(page, 'light');
    await expect(page).toHaveScreenshot('timeline-wide-light.png');
  });

  // 3: fresh default narrow dark（760px 未満 breakpoint、nav オフキャンバス）
  test('fresh default narrow dark', async ({ page }) => {
    await seedAppearance(page, 'dark');
    await page.setViewportSize(NARROW);
    await page.goto('/');
    await expect(page.getByText('browser mock peer post')).toBeVisible();
    await expect(page.getByTestId('control-center-trigger')).toBeVisible();
    await settleForShot(page, 'dark');
    await expect(page).toHaveScreenshot('timeline-narrow-dark.png');
  });

  // 4: fresh default narrow light（narrow × light）
  test('fresh default narrow light', async ({ page }) => {
    await seedAppearance(page, 'light');
    await page.setViewportSize(NARROW);
    await page.goto('/');
    await expect(page.getByText('browser mock peer post')).toBeVisible();
    await expect(page.getByTestId('control-center-trigger')).toBeVisible();
    await settleForShot(page, 'light');
    await expect(page).toHaveScreenshot('timeline-narrow-light.png');
  });

  // 5: thread pane open（context pane 層）
  test('thread pane wide dark', async ({ page }) => {
    await seedAppearance(page, 'dark');
    await page.setViewportSize(WIDE);
    await page.goto('/');
    await page.getByText('browser mock peer post').click();
    await expect(activeColumn(page, 'Thread')).toBeVisible();
    await settleForShot(page, 'dark');
    await expect(page).toHaveScreenshot('thread-pane-wide-dark.png');
  });

  // 6: author pane open（detail pane スタック）
  test('author pane wide dark', async ({ page }) => {
    await seedAppearance(page, 'dark');
    await page.setViewportSize(WIDE);
    await page.goto('/');
    await page.getByText('browser mock peer post').click();
    const threadPane = activeColumn(page, 'Thread');
    await expect(threadPane).toBeVisible();
    // Thread 内のシード post の著者（browser peer）を開いて Author pane を積む。
    await threadPane
      .getByRole('button', { name: 'browser peer' })
      .first()
      .click();
    await expect(activeColumn(page, 'Profile')).toBeVisible();
    await settleForShot(page, 'dark');
    await expect(page).toHaveScreenshot('author-pane-wide-dark.png');
  });

  // 7: messages（DM workspace）
  test('messages wide dark', async ({ page }) => {
    await seedAppearance(page, 'dark');
    await page.setViewportSize(WIDE);
    await page.goto('/#/messages');
    await expect(activeColumn(page, 'Messages')).toBeVisible();
    await settleForShot(page, 'dark');
    await expect(page).toHaveScreenshot('messages-wide-dark.png');
  });

  // 8: notifications（deep link、notification-item 系 CSS）
  test('notifications wide dark', async ({ page }) => {
    await seedAppearance(page, 'dark');
    await page.setViewportSize(WIDE);
    await page.goto('/#/notifications?topic=kukuri%3Atopic%3Ageneral');
    await expect(activeColumn(page, 'Notifications')).toBeVisible();
    await expect(page.getByText('browser mock reply notification')).toBeVisible();
    await settleForShot(page, 'dark');
    await expect(page).toHaveScreenshot('notifications-wide-dark.png');
  });

  // 9: settings drawer / appearance（Radix Portal + drawer、H8 最重要）
  test('settings appearance wide dark', async ({ page }) => {
    await seedAppearance(page, 'dark');
    await page.setViewportSize(WIDE);
    await page.goto('/#/timeline?topic=kukuri%3Atopic%3Ageneral&settings=appearance');
    const settingsDialog = page.getByRole('dialog', { name: 'Settings' });
    await expect(settingsDialog).toBeVisible();
    await expect(settingsDialog.getByTestId('settings-section-appearance')).toHaveAttribute(
      'aria-current',
      'location'
    );
    await settleForShot(page, 'dark');
    await expect(page).toHaveScreenshot('settings-appearance-wide-dark.png');
  });

  // 10: settings drawer / appearance light（portal × light）
  test('settings appearance wide light', async ({ page }) => {
    await seedAppearance(page, 'light');
    await page.setViewportSize(WIDE);
    await page.goto('/#/timeline?topic=kukuri%3Atopic%3Ageneral&settings=appearance');
    const settingsDialog = page.getByRole('dialog', { name: 'Settings' });
    await expect(settingsDialog).toBeVisible();
    await expect(settingsDialog.getByTestId('settings-section-appearance')).toHaveAttribute(
      'aria-current',
      'location'
    );
    await settleForShot(page, 'light');
    await expect(page).toHaveScreenshot('settings-appearance-wide-light.png');
  });

  // 11: settings drawer / connectivity（スクロール drawer・フォーム群）
  test('settings connectivity wide dark', async ({ page }) => {
    await seedAppearance(page, 'dark');
    await page.setViewportSize(WIDE);
    await page.goto('/#/timeline?topic=kukuri%3Atopic%3Ageneral&settings=connectivity');
    const settingsDialog = page.getByRole('dialog', { name: 'Settings' });
    await expect(settingsDialog).toBeVisible();
    await expect(settingsDialog.getByTestId('settings-section-connectivity')).toHaveAttribute(
      'aria-current',
      'location'
    );
    // スクロール top を固定してから撮る（付録 B: スクロール top 固定）。
    await settingsDialog
      .locator('.shell-settings-content')
      .evaluate((element) => element.scrollTo(0, 0));
    await settleForShot(page, 'dark');
    await expect(page).toHaveScreenshot('settings-connectivity-wide-dark.png');
  });

  // 12: composer dialog open（Radix portal dialog）
  test('composer dialog wide dark', async ({ page }) => {
    await seedAppearance(page, 'dark');
    await page.setViewportSize(WIDE);
    await page.goto('/');
    await expect(page.getByText('browser mock peer post')).toBeVisible();
    await openComposerDialog(page);
    await expect(page.getByPlaceholder('Write a post')).toBeVisible();
    await settleForShot(page, 'dark');
    await expect(page).toHaveScreenshot('composer-dialog-wide-dark.png');
  });

  // 13: live workspace（シード空状態、extended 面）
  test('live wide dark', async ({ page }) => {
    await seedAppearance(page, 'dark');
    await page.setViewportSize(WIDE);
    await page.goto('/#/live');
    await expect(activeColumn(page, 'Live')).toBeVisible();
    await settleForShot(page, 'dark');
    await expect(page).toHaveScreenshot('live-wide-dark.png');
  });

  // 14: game workspace（Metaverse Rooms 一覧、WebGL 手前まで）
  test('game wide dark', async ({ page }) => {
    await seedAppearance(page, 'dark');
    await page.setViewportSize(WIDE);
    await page.goto('/#/game');
    await expect(activeColumn(page, 'Metaverse')).toBeVisible();
    await settleForShot(page, 'dark');
    await expect(page).toHaveScreenshot('game-wide-dark.png');
  });
});
