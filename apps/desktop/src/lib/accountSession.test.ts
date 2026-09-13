import { expect, test, vi } from 'vitest';
import * as identity from '@/lib/api/identity';
import { reconcileAccountDrafts, changeAccountSession, accountCreationOperationId } from './accountSession';
import { COLUMN_DRAFT_STORAGE_KEY } from '@/shell/columnDraftPersistence';

test('creation operation survives a module restart and clears only after activation', async () => {
  const operation = accountCreationOperationId('a');
  vi.resetModules();
  const reloaded = await import('./accountSession');
  expect(reloaded.accountCreationOperationId('a')).toBe(operation);
  localStorage.setItem('kukuri:account-transition-drafts', JSON.stringify({ sourceId: 'a', draft: null, createOperationId: operation }));
  vi.spyOn(identity, 'listAccounts').mockResolvedValue({ active_account_id: 'a', accounts: [] });
  await reconcileAccountDrafts();
  expect(accountCreationOperationId('a')).toBe(operation);
  localStorage.setItem('kukuri:account-transition-drafts', JSON.stringify({ sourceId: 'a', draft: null, createOperationId: operation }));
  vi.spyOn(identity, 'listAccounts').mockResolvedValue({ active_account_id: 'b', accounts: [] });
  await reconcileAccountDrafts();
  expect(accountCreationOperationId('a')).not.toBe(operation);
});

test('restart after committed account change restores only the active account draft', async () => {
  localStorage.setItem('kukuri:account-transition-drafts', JSON.stringify({ sourceId: 'a', draft: 'private-a' }));
  localStorage.setItem('kukuri:account:b:retained-column-drafts', 'private-b');
  localStorage.setItem(COLUMN_DRAFT_STORAGE_KEY, 'private-a');
  vi.spyOn(identity, 'listAccounts').mockResolvedValue({ active_account_id: 'b', accounts: [] });
  expect(await reconcileAccountDrafts()).toBe(true);
  expect(localStorage.getItem(COLUMN_DRAFT_STORAGE_KEY)).toBe('private-b');
  expect(await reconcileAccountDrafts()).toBe(false);
});

test('restart before account commit restores the original draft', async () => {
  localStorage.setItem('kukuri:account-transition-drafts', JSON.stringify({ sourceId: 'a', draft: 'private-a' }));
  vi.spyOn(identity, 'listAccounts').mockResolvedValue({ active_account_id: 'a', accounts: [] });
  await reconcileAccountDrafts();
  expect(localStorage.getItem(COLUMN_DRAFT_STORAGE_KEY)).toBe('private-a');
});

test('storage failure prevents logout and backend failure retains the draft', async () => {
  localStorage.setItem(COLUMN_DRAFT_STORAGE_KEY, 'private-a');
  vi.spyOn(identity, 'listAccounts').mockResolvedValue({ active_account_id: 'a', accounts: [] });
  const logout = vi.spyOn(identity, 'logoutAccount').mockRejectedValue(new Error('cannot switch'));
  const storage = vi.spyOn(Storage.prototype, 'setItem').mockImplementation(() => { throw new Error('full'); });
  await expect(changeAccountSession('a', true)).rejects.toThrow('full');
  expect(logout).not.toHaveBeenCalled();
  storage.mockRestore();
  await expect(changeAccountSession('a', true)).rejects.toThrow('cannot switch');
  expect(localStorage.getItem(COLUMN_DRAFT_STORAGE_KEY)).toBe('private-a');
  expect(localStorage.getItem('kukuri:account:a:retained-column-drafts')).toBe('private-a');
});
