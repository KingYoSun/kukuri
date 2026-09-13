import { expect, test } from '@playwright/test';

test('account menu opens own profile and add dialog using pointer and keyboard', async ({ page }) => {
  await page.goto('/');
  const trigger = page.getByTestId('account-menu-trigger');
  await trigger.click();
  const menu = page.getByRole('menu', { name: 'Account menu' });
  await expect(menu.getByRole('menuitem').first()).toHaveText('View profile');
  await expect(menu.getByRole('menuitemradio').first()).toBeVisible();
  await menu.getByRole('menuitem', { name: 'View profile' }).click();
  await expect(page.locator('[data-column-id]:focus')).toHaveAttribute('aria-label', /^Profile/);
  const count = await page.locator('[data-column-id]').count();
  await trigger.focus();
  await page.keyboard.press('Enter');
  await page.keyboard.press('Enter');
  await expect(page.locator('[data-column-id]')).toHaveCount(count);
  await trigger.click();
  await menu.getByRole('menuitem', { name: 'Add account' }).click();
  const dialog = page.getByRole('dialog', { name: 'Add account' });
  await expect(dialog.getByRole('button', { name: 'Create a new account' })).toBeEnabled();
  await expect(dialog.getByTestId('import-input')).toBeVisible();
  await page.keyboard.press('Escape');
  await expect(trigger).toBeFocused();
});

test('logout cancellation retains the account in the menu', async ({ page }) => {
  await page.goto('/');
  const trigger = page.getByTestId('account-menu-trigger');
  await trigger.click();
  const menu = page.getByRole('menu', { name: 'Account menu' });
  await expect(menu.getByRole('menuitemradio')).toHaveCount(1);
  await menu.getByRole('menuitem', { name: 'Log out' }).click();
  const dialog = page.getByRole('dialog', { name: 'Log out of this account?' });
  await expect(dialog.getByText(/Local data stays here/)).toBeVisible();
  await expect(dialog.getByRole('button', { name: 'Cancel' })).toBeFocused();
  await dialog.getByRole('button', { name: 'Cancel' }).click();
  await trigger.click();
  await expect(menu.getByRole('menuitemradio')).toHaveCount(1);
});
