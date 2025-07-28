import { invoke } from '@tauri-apps/api/core';

export interface AccountMetadata {
  npub: string;
  pubkey: string;
  name: string;
  display_name: string;
  picture?: string;
  last_used: string;
}

export interface AddAccountRequest {
  nsec: string;
  name: string;
  display_name: string;
  picture?: string;
}

export interface AddAccountResponse {
  npub: string;
  pubkey: string;
}

export interface SwitchAccountResponse {
  npub: string;
  pubkey: string;
}

export interface GetCurrentAccountResponse {
  npub: string;
  nsec: string;
  pubkey: string;
  metadata: AccountMetadata;
}

export interface LoginResponse {
  public_key: string;
  npub: string;
}

/**
 * セキュアストレージAPIのラッパー
 * プラットフォーム固有のセキュアストレージ（Keychain、Credential Manager等）を使用
 */
export const SecureStorageApi = {
  /**
   * 新しいアカウントを追加
   */
  async addAccount(request: AddAccountRequest): Promise<AddAccountResponse> {
    return await invoke<AddAccountResponse>('add_account', { request });
  },

  /**
   * 保存されているアカウント一覧を取得
   */
  async listAccounts(): Promise<AccountMetadata[]> {
    return await invoke<AccountMetadata[]>('list_accounts');
  },

  /**
   * アカウントを切り替え
   */
  async switchAccount(npub: string): Promise<SwitchAccountResponse> {
    return await invoke<SwitchAccountResponse>('switch_account', { npub });
  },

  /**
   * アカウントを削除
   */
  async removeAccount(npub: string): Promise<void> {
    return await invoke<void>('remove_account', { npub });
  },

  /**
   * 現在のアカウント情報を取得（自動ログイン用）
   */
  async getCurrentAccount(): Promise<GetCurrentAccountResponse | null> {
    return await invoke<GetCurrentAccountResponse | null>('get_current_account');
  },

  /**
   * セキュアストレージからログイン
   */
  async secureLogin(npub: string): Promise<LoginResponse> {
    return await invoke<LoginResponse>('secure_login', { npub });
  },
};
