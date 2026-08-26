import { expect, test, type Locator, type Page } from '@playwright/test';

import { DEVELOPER_MODE_STORAGE_KEY } from '../../src/lib/developerMode';

// Issue #768 T3: scope マトリクスの browser 裏付け。
// jsdom 版 DesktopShellPage.columnScope.test.tsx と同じシナリオ(Control Center の channel manager で
// private channel 作成 → Public / private の Timeline Column 並置)を browser mock 上で再現し、
// scope 別表示 / scope 別返信 / DM footer 送信 / Control Center Focus を固定する。
test.beforeEach(async ({ page }) => {
  await page.addInitScript((key) => {
    window.localStorage.setItem(key, 'true');
  }, DEVELOPER_MODE_STORAGE_KEY);
});

const DEMO_PUBLIC_SCOPE = 'Public · demo';
const DEMO_PRIVATE_SCOPE = 'core · demo';

function activeColumn(page: Page, title: string) {
  return page.getByRole('region', { name: new RegExp(`^${title} Column,.*Active,`) });
}

// Timeline Column を header の scope label(Public · demo / core · demo)で特定する。
// accessible name は scope を含まないため header テキストで絞り込む。
function timelineColumnByScope(page: Page, scopeLabel: string) {
  return page
    .getByRole('region', { name: /^Timeline Column,/ })
    .filter({ has: page.locator('.shell-column-header', { hasText: scopeLabel }) });
}

async function openControlCenter(page: Page) {
  const controlCenter = page.getByRole('complementary', { name: 'Control Center' });
  if (!(await controlCenter.isVisible().catch(() => false))) {
    await page.getByTestId('control-center-trigger').click();
  }
  await expect(controlCenter).toBeVisible();
  return controlCenter;
}

// Column footer の Publish 導線から投稿する(成功すると draft は破棄され composer は畳まれる)。
async function publishFromTimelineColumn(
  page: Page,
  column: Locator,
  scopeLabel: string,
  content: string
) {
  await column.getByRole('button', { name: `Publish to ${scopeLabel}` }).click();
  await column.getByPlaceholder('Write a post').fill(content);
  await column
    .locator('.shell-column-composer')
    .getByRole('button', { name: 'Publish', exact: true })
    .click();
  await expect(column.getByText(content)).toBeVisible();
}

// private channel「core」を作成し、channel scope の投稿を 1 件入れたうえで
// global 選択を Public に戻し、Public / core の Timeline Column 2 本(Public 側 active)を返す。
async function setUpPublicAndPrivateColumns(page: Page) {
  const controlCenter = await openControlCenter(page);
  await controlCenter.getByRole('button', { name: 'Create or join channel' }).click();
  const channelDialog = page.getByRole('dialog', { name: 'Create / Join Private Channel' });
  await expect(channelDialog).toBeVisible();
  await channelDialog.getByPlaceholder('Channel name').fill('core');
  await channelDialog.getByRole('button', { name: 'Create Channel' }).click();
  await expect(page).toHaveURL(/#\/timeline\?topic=kukuri%3Atopic%3Ademo&channel=channel-1/);
  await page.keyboard.press('Escape');

  const privateColumn = timelineColumnByScope(page, DEMO_PRIVATE_SCOPE);
  await expect(privateColumn).toBeVisible();
  await publishFromTimelineColumn(page, privateColumn, DEMO_PRIVATE_SCOPE, 'private scoped post');

  // Control Center の demo topic 行で「Public」を選び、global 選択を public に戻す。
  const reopened = await openControlCenter(page);
  const demoTopicRow = reopened
    .getByRole('button', { name: 'demo', exact: true })
    .locator('xpath=ancestor::li[1]');
  await demoTopicRow.getByRole('button', { name: /^Public/ }).click();
  await expect(page).toHaveURL(/#\/timeline\?topic=kukuri%3Atopic%3Ademo$/);

  const publicColumn = timelineColumnByScope(page, DEMO_PUBLIC_SCOPE);
  await expect(publicColumn).toHaveAttribute('aria-current', 'true');
  await expect(privateColumn).toBeVisible();
  return { privateColumn, publicColumn };
}

test('Public and private Timeline Columns coexist and keep posts within their own scope', async ({
  page,
}) => {
  await page.setViewportSize({ width: 1400, height: 980 });
  await page.goto('/');
  const { privateColumn, publicColumn } = await setUpPublicAndPrivateColumns(page);

  // seed の Public 投稿は Public 側だけに、channel 投稿は private 側だけに現れる。
  await expect(publicColumn.getByText('browser mock peer post')).toBeVisible();
  await expect(privateColumn.getByText('browser mock peer post')).toHaveCount(0);
  await expect(privateColumn.getByText('private scoped post')).toBeVisible();
  await expect(publicColumn.getByText('private scoped post')).toHaveCount(0);

  // Public Column から投稿しても private Column には現れない。
  await publishFromTimelineColumn(page, publicColumn, DEMO_PUBLIC_SCOPE, 'public scoped post');
  await expect(privateColumn.getByText('public scoped post')).toHaveCount(0);
});

test('Thread opened from the private Column keeps the private scope for header and footer reply', async ({
  page,
}) => {
  await page.setViewportSize({ width: 1400, height: 980 });
  await page.goto('/');
  const { privateColumn, publicColumn } = await setUpPublicAndPrivateColumns(page);

  // 画面外のprivate Columnをactiveにしてから、投稿本文クリックでThreadを開く。
  await privateColumn.locator('.shell-column-header h2').first().dispatchEvent('pointerdown');
  await expect(privateColumn).toHaveAttribute('aria-current', 'true');
  await privateColumn
    .getByRole('button', { name: /^private scoped post/ })
    .click();
  const threadColumn = activeColumn(page, 'Thread');
  await expect(threadColumn).toBeVisible();
  await expect(threadColumn.locator('.shell-column-header')).toContainText(
    `Thread · ${DEMO_PRIVATE_SCOPE}`
  );
  await expect(page).toHaveURL(/channel=channel-1/);
  await expect(page).toHaveURL(/context=thread/);

  // footer 返信は private channel scope のラベルのまま送信できる。
  await threadColumn
    .getByRole('button', { name: `Reply to Thread · ${DEMO_PRIVATE_SCOPE}` })
    .click();
  await threadColumn.getByPlaceholder('Write a reply').fill('private scoped reply');
  await threadColumn
    .locator('.shell-column-composer')
    .getByRole('button', { name: 'Reply', exact: true })
    .click();

  // 送信後は Thread と private timeline に反映され、Public timeline には漏れない。
  await expect(threadColumn.getByText('private scoped reply')).toBeVisible();
  await expect(privateColumn.getByText('private scoped reply')).toBeVisible();
  await expect(publicColumn.getByText('private scoped reply')).toHaveCount(0);
});

test('Conversation Column footer sends a direct message into the conversation timeline', async ({
  page,
}) => {
  await page.setViewportSize({ width: 1400, height: 980 });
  await page.goto('/');

  await page.getByRole('button', { name: 'browser peer' }).first().click();
  const profileColumn = activeColumn(page, 'Profile');
  await expect(profileColumn).toBeVisible();
  await profileColumn.getByRole('button', { name: 'Message' }).click();
  const conversationColumn = activeColumn(page, 'Conversation');
  await expect(conversationColumn).toBeVisible();

  // Conversation Column footer(Message to …)から composer を開いて送信する。
  await conversationColumn.getByRole('button', { name: 'Message to browser peer' }).click();
  await conversationColumn.getByPlaceholder('Write a message').fill('dm footer message');
  await conversationColumn
    .locator('.shell-column-composer')
    .getByRole('button', { name: 'Send', exact: true })
    .click();

  await expect(conversationColumn.getByText('dm footer message')).toBeVisible();
});

// Control Center の Columns 一覧「Focus …」で activeColumn が切り替わり route が同期する。
// mobile では active 切替に伴う Canvas の scroll 同期(page indicator 表示)も確認する。
for (const viewport of [
  { width: 1400, height: 980, label: 'desktop' },
  { width: 390, height: 844, label: 'mobile' },
]) {
  test(`Control Center Focus switches the active Column and syncs the route (${viewport.label} ${viewport.width}x${viewport.height})`, async ({
    page,
  }) => {
    await page.setViewportSize({ width: viewport.width, height: viewport.height });
    await page.goto('/#/timeline?topic=kukuri%3Atopic%3Ademo');

    // fresh defaultの末尾にThreadを追加した6 Column状態を作る。
    const timelineColumn = activeColumn(page, 'Timeline');
    await publishFromTimelineColumn(page, timelineColumn, DEMO_PUBLIC_SCOPE, 'focus sync post');
    await page.getByText('focus sync post').click();
    await expect(activeColumn(page, 'Thread')).toBeVisible();

    const canvas = page.locator('.shell-column-canvas');

    let controlCenter = await openControlCenter(page);
    await controlCenter.getByRole('button', { name: 'Focus Timeline' }).click();
    await expect(activeColumn(page, 'Timeline')).toBeVisible();
    await expect(page).toHaveURL(/#\/timeline\?topic=kukuri%3Atopic%3Ademo$/);
    if (viewport.label === 'mobile') {
      await expect(page.getByText('1 / 6')).toBeVisible();
      await expect
        .poll(() => canvas.evaluate((element) => Math.round(element.scrollLeft)))
        .toBeLessThanOrEqual(1);
    }

    controlCenter = await openControlCenter(page);
    await controlCenter.getByRole('button', { name: 'Focus Thread' }).click();
    await expect(activeColumn(page, 'Thread')).toBeVisible();
    await expect(page).toHaveURL(/context=thread&threadId=/);
    if (viewport.label === 'mobile') {
      await expect(page.getByText('2 / 6')).toBeVisible();
      await expect
        .poll(() => canvas.evaluate((element) => Math.round(element.scrollLeft)))
        .toBeGreaterThan(0);
    }
  });
}
