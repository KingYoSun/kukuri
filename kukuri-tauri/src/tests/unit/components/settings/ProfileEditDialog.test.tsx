import type { ComponentProps } from 'react';
import { describe, it, expect, vi, beforeEach, beforeAll, afterAll } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';

import { ProfileEditDialog } from '@/components/settings/ProfileEditDialog';
import { useAuthStore } from '@/stores/authStore';
import { updateNostrMetadata } from '@/lib/api/nostr';
import { toast } from 'sonner';
import { errorHandler } from '@/lib/errorHandler';

vi.mock('@/stores/authStore');
vi.mock('@/lib/api/nostr');

vi.mock('@/lib/api/tauri', () => ({
  TauriApi: {
    uploadProfileAvatar: vi.fn(),
    fetchProfileAvatar: vi.fn(),
    updatePrivacySettings: vi.fn(),
    profileAvatarSync: vi.fn(),
  },
}));

vi.mock('sonner', () => ({
  toast: {
    error: vi.fn(),
    success: vi.fn(),
  },
}));

vi.mock('@/lib/errorHandler', () => ({
  errorHandler: {
    log: vi.fn(),
  },
}));

vi.mock('@tauri-apps/plugin-dialog', () => ({
  open: vi.fn(),
}));

vi.mock('@tauri-apps/plugin-fs', () => ({
  readFile: vi.fn(),
}));

let mockOpen: ReturnType<typeof vi.fn>;
let mockReadFile: ReturnType<typeof vi.fn>;
let mockUploadProfileAvatar: ReturnType<typeof vi.fn>;
let mockFetchProfileAvatar: ReturnType<typeof vi.fn>;
let mockUpdatePrivacySettings: ReturnType<typeof vi.fn>;
let mockProfileAvatarSync: ReturnType<typeof vi.fn>;

beforeAll(async () => {
  const dialogModule = await import('@tauri-apps/plugin-dialog');
  const fsModule = await import('@tauri-apps/plugin-fs');
  const tauriModule = await import('@/lib/api/tauri');
  mockOpen = dialogModule.open as unknown as ReturnType<typeof vi.fn>;
  mockReadFile = fsModule.readFile as unknown as ReturnType<typeof vi.fn>;
  mockUploadProfileAvatar = tauriModule.TauriApi.uploadProfileAvatar as unknown as ReturnType<
    typeof vi.fn
  >;
  mockFetchProfileAvatar = tauriModule.TauriApi.fetchProfileAvatar as unknown as ReturnType<
    typeof vi.fn
  >;
  tauriModule.TauriApi.updatePrivacySettings = vi.fn();
  tauriModule.TauriApi.profileAvatarSync = vi.fn();
  mockUpdatePrivacySettings = tauriModule.TauriApi.updatePrivacySettings as unknown as ReturnType<
    typeof vi.fn
  >;
  mockProfileAvatarSync = tauriModule.TauriApi.profileAvatarSync as unknown as ReturnType<
    typeof vi.fn
  >;
});

const mockUseAuthStore = useAuthStore as unknown as vi.Mock;
const mockUpdateNostrMetadata = updateNostrMetadata as unknown as vi.Mock;

const originalCreateObjectURL = global.URL.createObjectURL;
const originalRevokeObjectURL = global.URL.revokeObjectURL;

beforeAll(() => {
  global.URL.createObjectURL = vi.fn(() => 'blob:profile-edit');
  global.URL.revokeObjectURL = vi.fn();
});

afterAll(() => {
  global.URL.createObjectURL = originalCreateObjectURL;
  global.URL.revokeObjectURL = originalRevokeObjectURL;
});

describe('ProfileEditDialog', () => {
  const mockCurrentUser = {
    id: 'pubkey_current',
    pubkey: 'pubkey_current',
    npub: 'npub1current',
    name: '現在のユーザー',
    displayName: 'Current User',
    about: '自己紹介テキスト',
    picture: 'https://example.com/avatar.png',
    nip05: 'user@example.com',
    avatar: {
      blobHash: 'blob123',
      format: 'image/png',
      sizeBytes: 1024,
      accessLevel: 'contacts_only' as const,
      shareTicket: 'ticket-1',
      docVersion: 4,
      updatedAt: '2025-11-02T00:00:00Z',
      contentSha256: 'deadbeef',
      nostrUri: 'iroh+avatar://profile_avatars?npub=npub1current&hash=blob123&v=4',
    },
  };

  const mockUpdateUser = vi.fn();

  type ProfileEditDialogProps = ComponentProps<typeof ProfileEditDialog>;

  const renderDialog = (props?: Partial<ProfileEditDialogProps>) => {
    const onOpenChange = vi.fn();
    render(<ProfileEditDialog open onOpenChange={onOpenChange} {...props} />);
    return { onOpenChange };
  };

  beforeEach(() => {
    vi.clearAllMocks();
    mockUseAuthStore.mockReturnValue({
      currentUser: mockCurrentUser,
      updateUser: mockUpdateUser,
    });
    mockUpdateNostrMetadata.mockResolvedValue(undefined);
    mockUploadProfileAvatar.mockResolvedValue({
      npub: mockCurrentUser.npub,
      blob_hash: 'hash999',
      format: 'image/png',
      size_bytes: 4,
      access_level: 'contacts_only',
      share_ticket: 'ticket-new',
      doc_version: 8,
      updated_at: '2025-11-03T00:00:00Z',
      content_sha256: 'cafebabe',
    });
    mockFetchProfileAvatar.mockResolvedValue({
      npub: mockCurrentUser.npub,
      blob_hash: 'hash999',
      format: 'image/png',
      size_bytes: 4,
      access_level: 'contacts_only',
      share_ticket: 'ticket-new',
      doc_version: 8,
      updated_at: '2025-11-03T00:00:00Z',
      content_sha256: 'cafebabe',
      data_base64: 'AQIDBA==',
    });
    mockUpdatePrivacySettings.mockResolvedValue(undefined);
    mockProfileAvatarSync.mockResolvedValue({
      npub: mockCurrentUser.npub,
      currentVersion: 8,
      updated: false,
      avatar: undefined,
    });
    mockOpen.mockResolvedValue(null);
    mockReadFile.mockResolvedValue(new Uint8Array([1, 2, 3, 4]));
  });

  it('初期値をフォームに表示する', () => {
    renderDialog();

    expect(screen.getByLabelText('名前 *')).toHaveValue('現在のユーザー');
    expect(screen.getByLabelText('表示名')).toHaveValue('Current User');
    expect(screen.getByLabelText('自己紹介')).toHaveValue('自己紹介テキスト');
    expect(screen.getByLabelText('アバター画像URL')).toHaveValue('https://example.com/avatar.png');
    expect(screen.getByLabelText('NIP-05認証')).toHaveValue('user@example.com');
  });

  it('フォーム送信でプロフィールを更新する', async () => {
    const user = userEvent.setup();
    const { onOpenChange } = renderDialog();

    await user.clear(screen.getByLabelText('名前 *'));
    await user.type(screen.getByLabelText('名前 *'), '更新後ユーザー');
    await user.clear(screen.getByLabelText('表示名'));
    await user.type(screen.getByLabelText('表示名'), '@updated');
    await user.clear(screen.getByLabelText('自己紹介'));
    await user.type(screen.getByLabelText('自己紹介'), '更新後の自己紹介');
    await user.clear(screen.getByLabelText('アバター画像URL'));
    await user.type(screen.getByLabelText('アバター画像URL'), 'https://example.com/new.png');
    await user.clear(screen.getByLabelText('NIP-05認証'));
    await user.type(screen.getByLabelText('NIP-05認証'), 'updated@example.com');

    await user.click(screen.getByRole('button', { name: '保存' }));

    await waitFor(() => {
      expect(mockUpdateNostrMetadata).toHaveBeenCalledWith(
        expect.objectContaining({
          name: '更新後ユーザー',
          display_name: '@updated',
          about: '更新後の自己紹介',
          picture: 'https://example.com/new.png',
          nip05: 'updated@example.com',
          kukuri_privacy: expect.objectContaining({
            public_profile: true,
            show_online_status: false,
          }),
        }),
      );
    });
    await waitFor(() => {
      expect(mockUpdatePrivacySettings).toHaveBeenCalledWith({
        npub: mockCurrentUser.npub,
        publicProfile: true,
        showOnlineStatus: false,
      });
    });
    expect(mockProfileAvatarSync).toHaveBeenCalledWith({
      npub: mockCurrentUser.npub,
      knownDocVersion: null,
    });

    expect(mockUpdateUser).toHaveBeenCalledWith(
      expect.objectContaining({
        name: '更新後ユーザー',
        displayName: '@updated',
        about: '更新後の自己紹介',
        picture: 'https://example.com/new.png',
        nip05: 'updated@example.com',
        avatar: mockCurrentUser.avatar,
      }),
    );
    await waitFor(() => {
      expect(toast.success).toHaveBeenCalledWith('プロフィールを更新しました');
    });
    expect(onOpenChange).toHaveBeenCalledWith(false);
  });

  it('画像をアップロードして保存した場合、Tauri API を呼び出す', async () => {
    const user = userEvent.setup();
    renderDialog();

    mockOpen.mockResolvedValue('C:/temp/avatar.png');
    mockReadFile.mockResolvedValue(new Uint8Array([10, 11, 12]));

    await user.click(screen.getByRole('button', { name: /画像をアップロード/ }));
    await user.click(screen.getByRole('button', { name: '保存' }));

    await waitFor(() => {
      expect(mockUploadProfileAvatar).toHaveBeenCalledWith({
        npub: mockCurrentUser.npub,
        data: new Uint8Array([10, 11, 12]),
        format: 'image/png',
        accessLevel: 'contacts_only',
      });
    });

    expect(mockFetchProfileAvatar).toHaveBeenCalledWith(mockCurrentUser.npub);

    const expectedParams = new URLSearchParams({
      npub: mockCurrentUser.npub,
      hash: 'hash999',
      v: '8',
    }).toString();
    const expectedUri = `iroh+avatar://profile_avatars?${expectedParams}`;

    await waitFor(() => {
      expect(mockUpdateNostrMetadata).toHaveBeenCalledWith(
        expect.objectContaining({
          picture: expectedUri,
          kukuri_privacy: expect.objectContaining({
            public_profile: true,
            show_online_status: false,
          }),
        }),
      );
    });

    expect(mockUpdateUser).toHaveBeenCalledWith(
      expect.objectContaining({
        picture: 'data:image/png;base64,AQIDBA==',
        avatar: expect.objectContaining({
          blobHash: 'hash999',
          nostrUri: expectedUri,
          docVersion: 8,
        }),
      }),
    );
  });

  it('名前が未入力の場合はエラーを表示して送信しない', async () => {
    const user = userEvent.setup();
    renderDialog();

    await user.clear(screen.getByLabelText('名前 *'));
    await user.click(screen.getByRole('button', { name: '保存' }));

    expect(toast.error).toHaveBeenCalledWith('名前を入力してください');
    expect(mockUpdateNostrMetadata).not.toHaveBeenCalled();
  });

  it('currentUser が存在しない場合はエラーを表示する', async () => {
    const user = userEvent.setup();
    mockUseAuthStore.mockReturnValue({
      currentUser: null,
      updateUser: mockUpdateUser,
    });

    renderDialog();

    await user.type(screen.getByLabelText('名前 *'), 'テストユーザー');
    await user.click(screen.getByRole('button', { name: '保存' }));

    expect(toast.error).toHaveBeenCalledWith('アカウント情報が見つかりません');
    expect(mockUpdateNostrMetadata).not.toHaveBeenCalled();
  });

  it('更新処理で例外が発生した場合はエラートーストを表示する', async () => {
    const user = userEvent.setup();
    const error = new Error('update failed');
    mockUpdateNostrMetadata.mockRejectedValueOnce(error);

    renderDialog();

    await user.type(screen.getByLabelText('名前 *'), '再試行ユーザー');
    await user.click(screen.getByRole('button', { name: '保存' }));

    await waitFor(() => {
      expect(toast.error).toHaveBeenCalledWith('プロフィールの更新に失敗しました');
    });
    expect(errorHandler.log).toHaveBeenCalledWith(
      'ProfileEditDialog.submitFailed',
      error,
      expect.objectContaining({ context: 'ProfileEditDialog.handleSubmit' }),
    );
  });
});
