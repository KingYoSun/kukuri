import { switchAccount, logoutAccount, listAccounts } from '@/lib/api/identity';
import { COLUMN_DRAFT_STORAGE_KEY } from '@/shell/columnDraftPersistence';

const retainedDraftKey = (id: string) => `kukuri:account:${id}:retained-column-drafts`;
const TRANSITION_KEY = 'kukuri:account-transition-drafts';

export async function reconcileAccountDrafts(): Promise<boolean> {
  const encoded = localStorage.getItem(TRANSITION_KEY);
  if (encoded === null) return false;
  const pending: { sourceId: string; draft: string | null } = JSON.parse(encoded);
  if (typeof pending.sourceId !== 'string' || (pending.draft !== null && typeof pending.draft !== 'string')) throw new Error('Invalid account draft transition');
  const snapshot = await listAccounts();
  const nextDraft = snapshot.active_account_id === pending.sourceId ? pending.draft : localStorage.getItem(retainedDraftKey(snapshot.active_account_id));
  if (nextDraft === null) localStorage.removeItem(COLUMN_DRAFT_STORAGE_KEY);
  else localStorage.setItem(COLUMN_DRAFT_STORAGE_KEY, nextDraft);
  localStorage.removeItem(TRANSITION_KEY);
  return true;
}

export async function changeAccountSession(accountId: string, logout = false) {
  const before = await listAccounts();
  const draft = localStorage.getItem(COLUMN_DRAFT_STORAGE_KEY);
  if (logout) {
    if (draft !== null) localStorage.setItem(retainedDraftKey(accountId), draft);
    else localStorage.removeItem(retainedDraftKey(accountId));
  } else localStorage.removeItem(retainedDraftKey(before.active_account_id));
  localStorage.setItem(TRANSITION_KEY, JSON.stringify({ sourceId: before.active_account_id, draft }));
  localStorage.removeItem(COLUMN_DRAFT_STORAGE_KEY);
  try {
    await (logout ? logoutAccount(accountId) : switchAccount(accountId));
  } catch (error) {
    try {
      const actual = await listAccounts();
      await reconcileAccountDrafts();
      if (actual.active_account_id !== before.active_account_id) window.location.reload();
    } catch { window.location.reload(); }
    throw error;
  }
  // On a storage failure the durable marker is reconciled before shell startup.
  try { await reconcileAccountDrafts(); } finally { window.location.reload(); }
}
