import { invokeCommand, invokeCommandVoid } from '@/lib/api/tauriClient';

export interface AccountMetadata {
  npub: string;
  pubkey: string;
  name: string;
  display_name: string;
  picture?: string;
  last_used: string;
  public_profile?: boolean;
  show_online_status?: boolean;
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
    return await invokeCommand<AddAccountResponse>('add_account', { request });
  },

  /**
   * 保存されているアカウント一覧を取得
   */
  async listAccounts(): Promise<AccountMetadata[]> {
    return await invokeCommand<AccountMetadata[]>('list_accounts');
  },

  /**
   * アカウントを切り替え
   */
  async switchAccount(npub: string): Promise<SwitchAccountResponse> {
    return await invokeCommand<SwitchAccountResponse>('switch_account', { npub });
  },

  /**
   * アカウントを削除
   */
  async removeAccount(npub: string): Promise<void> {
    await invokeCommandVoid('remove_account', { npub });
  },

  /**
   * 現在のアカウント情報を取得（自動ログイン用）
   */
  async getCurrentAccount(): Promise<GetCurrentAccountResponse | null> {
    return await invokeCommand<GetCurrentAccountResponse | null>('get_current_account');
  },

  /**
   * セキュアストレージからログイン
   */
  async secureLogin(npub: string): Promise<LoginResponse> {
    return await invokeCommand<LoginResponse>('secure_login', { npub });
  },

};
