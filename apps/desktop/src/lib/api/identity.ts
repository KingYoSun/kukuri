import type {
  AccountKeyExport,
  AccountKeyImportPreview,
  AccountRecord,
  AccountsSnapshot,
  ExportAccountKeyRequest,
  ImportAccountKeyRequest,
  PreviewAccountKeyImportRequest,
  SwitchAccountRequest,
} from './types.generated';

import { invokeDesktop } from './invoke/desktop';
import { isDesktopMockActive } from './invoke/dispatch';

// #859: アカウント鍵の export / import と複数アカウント管理。DesktopApi 外の
// スタンドアロンコマンド(appConsent.ts と同じ形)。mock ビルドでは in-memory の
// アカウント一覧を返す。平文秘密鍵はどの経路にも現れない。

const MOCK_ACTIVE_ACCOUNT: AccountRecord = {
  id: 'a1b2c3d4e5f60718',
  pubkey: 'a1b2c3d4e5f60718293a4b5c6d7e8f90a1b2c3d4e5f60718293a4b5c6d7e8f90',
  label: null,
  created_at: 1_756_684_800_000,
  last_used_at: 1_756_684_800_000,
};

const mockAccounts: AccountsSnapshot = {
  active_account_id: MOCK_ACTIVE_ACCOUNT.id,
  accounts: [MOCK_ACTIVE_ACCOUNT],
};

const MOCK_EXPORT_PREFIX = 'kukuri-account-key.v1.';

export async function listAccounts(): Promise<AccountsSnapshot> {
  if (isDesktopMockActive()) {
    return {
      active_account_id: mockAccounts.active_account_id,
      accounts: mockAccounts.accounts.map((account) => ({ ...account })),
    };
  }
  return invokeDesktop<AccountsSnapshot>('list_accounts');
}

export async function exportAccountKey(passphrase: string): Promise<AccountKeyExport> {
  if (isDesktopMockActive()) {
    return {
      export: `${MOCK_EXPORT_PREFIX}bW9jay1lbmNyeXB0ZWQtZW52ZWxvcGU`,
      public_key: MOCK_ACTIVE_ACCOUNT.pubkey,
    };
  }
  return invokeDesktop<AccountKeyExport>('export_account_key', {
    request: { passphrase } satisfies ExportAccountKeyRequest,
  });
}

export async function previewAccountKeyImport(
  exportText: string
): Promise<AccountKeyImportPreview> {
  if (isDesktopMockActive()) {
    if (!exportText.startsWith(MOCK_EXPORT_PREFIX)) {
      throw new Error('unsupported account key export format or version');
    }
    return {
      version: 1,
      kdf: 'argon2id',
      public_key: 'f0e1d2c3b4a59687f0e1d2c3b4a59687f0e1d2c3b4a59687f0e1d2c3b4a59687',
      already_registered: false,
    };
  }
  return invokeDesktop<AccountKeyImportPreview>('preview_account_key_import', {
    request: { export: exportText } satisfies PreviewAccountKeyImportRequest,
  });
}

export async function importAccountKey(
  exportText: string,
  passphrase: string,
  label?: string
): Promise<AccountRecord> {
  if (isDesktopMockActive()) {
    const preview = await previewAccountKeyImport(exportText);
    const record: AccountRecord = {
      id: preview.public_key.slice(0, 16),
      pubkey: preview.public_key,
      label: label ?? null,
      created_at: Date.now(),
      last_used_at: Date.now(),
    };
    if (mockAccounts.accounts.some((account) => account.pubkey === record.pubkey)) {
      throw new Error(`account with public key \`${record.pubkey}\` already exists`);
    }
    mockAccounts.accounts.push(record);
    return { ...record };
  }
  return invokeDesktop<AccountRecord>('import_account_key', {
    request: {
      export: exportText,
      passphrase,
      label: label ?? null,
    } satisfies ImportAccountKeyRequest,
  });
}

export async function switchAccount(accountId: string): Promise<AccountRecord> {
  if (isDesktopMockActive()) {
    const record = mockAccounts.accounts.find((account) => account.id === accountId);
    if (!record) {
      throw new Error(`unknown account \`${accountId}\``);
    }
    mockAccounts.active_account_id = accountId;
    return { ...record };
  }
  return invokeDesktop<AccountRecord>('switch_account', {
    request: { account_id: accountId } satisfies SwitchAccountRequest,
  });
}
