import { expect, test, type Page } from '@playwright/test';

const image = Buffer.from('iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+a/A8AAAAASUVORK5CYII=', 'base64');

async function seedDiagnostics(page: Page, locale: string, theme: string) {
  await page.addInitScript(({ locale, theme }) => {
    localStorage.setItem('kukuri.desktop.locale', locale);
    localStorage.setItem('kukuri.desktop.theme', theme);
    localStorage.setItem('kukuri.desktop.developer-mode', 'true');
    let desktopApi = window.__KUKURI_DESKTOP__;
    Object.defineProperty(window, '__KUKURI_DESKTOP__', {
      configurable: true,
      get: () => desktopApi,
      set: (api: typeof desktopApi) => {
        desktopApi = api;
        if (!api) return;
        void api.setCommunityNodeConfig([{ base_url: 'https://node.example' }]);
        const getStatus = api.getSyncStatus.bind(api);
        api.getSyncStatus = async () => {
          const status = await getStatus();
          const error = 'topic join pending: timed out waiting for initial topic join';
          return {
            ...status, last_error: error, status_detail: error, active_path: 'direct_p2p',
            discovery: { ...status.discovery, mode: 'seeded_dht', connect_mode: 'direct_or_relay', last_discovery_error: error },
            topic_diagnostics: status.topic_diagnostics.map(topic => ({ ...topic, last_error: error, status_detail: error })),
          };
        };
      },
    });
  }, { locale, theme });
}

for (const copy of [
  { locale: 'ja', timeout: 'トピックへの初回参加がタイムアウトしました', mode: 'シード付き DHT', connect: '直接接続またはリレー', auth: '認証と同意', path: '直接 P2P' },
  { locale: 'en', timeout: 'The initial topic join timed out', mode: 'Seeded DHT', connect: 'Direct connections or relay', auth: 'Authentication and consent', path: 'Direct P2P' },
  { locale: 'zh-CN', timeout: '首次加入话题超时', mode: '带种子的 DHT', connect: '直接连接或中继', auth: '认证与同意', path: '直接 P2P' },
]) {
  for (const theme of ['dark', 'light']) {
    test(`${copy.locale} ${theme} reported diagnostics are readable in settings and Control Center`, async ({ page }, testInfo) => {
      await seedDiagnostics(page, copy.locale, theme);
      await page.setViewportSize({ width: 1280, height: 800 });
      await page.goto('/#/timeline?settings=connectivity');
      const drawer = page.locator('.shell-settings-drawer');
      await expect(drawer.getByText(copy.timeout, { exact: false }).first()).toBeVisible();
      await drawer.getByTestId('settings-section-discovery').click();
      await expect(drawer.getByText(`${copy.mode} (seeded_dht)`).first()).toBeVisible();
      await expect(drawer.getByText(`${copy.connect} (direct_or_relay)`)).toBeVisible();
      await expect(drawer.getByText(copy.timeout, { exact: false })).toBeVisible();
      await testInfo.attach('discovery', { body: await drawer.screenshot(), contentType: 'image/png' });
      await drawer.getByTestId('settings-section-community-node').click();
      await expect(drawer.getByText(copy.auth, { exact: false }).first()).toBeVisible();
      for (const width of [1280, 700, 390]) {
        await page.setViewportSize({ width, height: 800 });
        const content = drawer.locator('.shell-settings-content');
        expect(await content.evaluate(el => el.scrollWidth <= el.clientWidth + 1)).toBe(true);
      }
      await page.setViewportSize({ width: 1280, height: 800 });
      await testInfo.attach('community-node', { body: await drawer.screenshot(), contentType: 'image/png' });
      await drawer.locator('.shell-settings-close').click();
      // These fixture nodes are unconsented; dismiss their existing introduction.
      await page.locator('.ui-dialog-close').click();
      await page.getByTestId('control-center-trigger').click();
      await expect(page.getByText(copy.path, { exact: false }).first()).toBeVisible();
    });
  }
}

test('reaction crop draft survives visiting appearance to change the language', async ({ page }, testInfo) => {
  await page.goto('/#/timeline?settings=reactions');
  const drawer = page.locator('.shell-settings-drawer');
  const choose = drawer.getByRole('button', { name: 'Choose files', exact: true });
  await choose.focus();
  const pendingChooser = page.waitForEvent('filechooser');
  await choose.press('Enter');
  const chooser = await pendingChooser;
  expect(chooser.isMultiple()).toBe(false);
  await chooser.setFiles({ name: 'リアクション画像.png', mimeType: 'image/png', buffer: image });
  const crop = page.getByRole('dialog', { name: 'Crop reaction image' });
  await crop.getByRole('button', { name: 'Save', exact: true }).click();
  await expect(crop).toBeHidden();
  await drawer.getByLabel('Search key', { exact: true }).fill('preserved-draft');
  await drawer.getByTestId('settings-section-appearance').click();
  await drawer.getByRole('combobox', { name: 'Language', exact: true }).selectOption('ja');
  await expect(drawer.getByRole('combobox', { name: '言語', exact: true })).toHaveValue('ja');
  await drawer.getByTestId('settings-section-reactions').click();
  await expect(drawer.getByText('リアクション画像.png', { exact: true })).toBeVisible();
  await expect(drawer.getByRole('textbox', { name: '検索キーワード', exact: true })).toHaveValue('preserved-draft');
  await expect(drawer.getByRole('button', { name: 'ファイルを選択', exact: true })).toBeVisible();
  await expect(drawer.locator('input[type=file]')).toBeHidden();
  await testInfo.attach('reaction-draft-ja', { body: await drawer.screenshot(), contentType: 'image/png' });
});
