import { describe, it, expect, vi, beforeEach, afterEach, beforeAll, afterAll } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { ProfileSetup } from '@/components/auth/ProfileSetup';
import { useAuthStore } from '@/stores/authStore';
import { updateNostrMetadata } from '@/lib/api/nostr';
import { toast } from 'sonner';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { ReactNode } from 'react';
import { errorHandler } from '@/lib/errorHandler';

// モック
const mockNavigate = vi.fn();
vi.mock('@tanstack/react-router', () => ({
  useNavigate: () => mockNavigate,
}));

vi.mock('sonner', () => ({
  toast: {
    error: vi.fn(),
    success: vi.fn(),
  },
}));

vi.mock('@/stores/authStore');
vi.mock('@/lib/api/nostr');

vi.mock('@/lib/errorHandler', () => ({
  errorHandler: {
    log: vi.fn(),
  },
}));

const mockOpen = vi.fn();
const mockReadBinaryFile = vi.fn();
vi.mock('@tauri-apps/api/dialog', () => ({
  open: mockOpen,
}));
vi.mock('@tauri-apps/api/fs', () => ({
  readBinaryFile: mockReadBinaryFile,
}));

const mockUploadProfileAvatar = vi.fn();
const mockFetchProfileAvatar = vi.fn();
vi.mock('@/lib/api/tauri', () => ({
  TauriApi: {
    uploadProfileAvatar: mockUploadProfileAvatar,
    fetchProfileAvatar: mockFetchProfileAvatar,
  },
}));

const createWrapper = () => {
  const queryClient = new QueryClient({
    defaultOptions: {
      queries: { retry: false },
      mutations: { retry: false },
    },
  });

  return ({ children }: { children: ReactNode }) => (
    <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>
  );
};

const originalCreateObjectURL = global.URL.createObjectURL;
const originalRevokeObjectURL = global.URL.revokeObjectURL;

beforeAll(() => {
  global.URL.createObjectURL = vi.fn(() => 'blob:profile-setup');
  global.URL.revokeObjectURL = vi.fn();
});

afterAll(() => {
  global.URL.createObjectURL = originalCreateObjectURL;
  global.URL.revokeObjectURL = originalRevokeObjectURL;
});

describe('ProfileSetup', () => {
  const mockUpdateUser = vi.fn();
  const mockCurrentUser = {
    id: 'test-id',
    pubkey: 'test-pubkey',
    npub: 'npub1test',
    name: '',
    displayName: '',
    about: '',
    picture: '',
    nip05: '',
    avatar: null,
  };

  beforeEach(() => {
    vi.clearAllMocks();
    mockOpen.mockResolvedValue(null);
    mockReadBinaryFile.mockReset();
    mockUploadProfileAvatar.mockReset();
    mockFetchProfileAvatar.mockReset();
    (useAuthStore as unknown as vi.Mock).mockReturnValue({
      currentUser: mockCurrentUser,
      updateUser: mockUpdateUser,
    });
  });

  afterEach(() => {
    vi.clearAllMocks();
  });

  it('プロフィール設定フォームが正しく表示される', () => {
    render(<ProfileSetup />, { wrapper: createWrapper() });

    // ヘッダー
    const cardTitle = screen.getByText('プロフィール設定').closest('[data-slot="card-title"]');
    expect(cardTitle).toBeInTheDocument();
    expect(screen.getByText('あなたの情報を設定しましょう')).toBeInTheDocument();

    // アバターセクション - アバターコンポーネントはimgタグではないかもしれない
    expect(screen.getByRole('button', { name: /画像をアップロード/ })).toBeInTheDocument();
    expect(screen.getByText('またはURLを下に入力')).toBeInTheDocument();

    // フォームフィールド
    expect(screen.getByLabelText('名前 *')).toBeInTheDocument();
    expect(screen.getByLabelText('表示名')).toBeInTheDocument();
    expect(screen.getByLabelText('自己紹介')).toBeInTheDocument();
    expect(screen.getByLabelText('アバター画像URL')).toBeInTheDocument();
    expect(screen.getByLabelText('NIP-05認証')).toBeInTheDocument();

    // ボタン
    expect(screen.getByRole('button', { name: '後で設定' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: '設定を完了' })).toBeInTheDocument();
  });

  it('既存ユーザーのデータが初期値として表示される', () => {
    const existingUser = {
      ...mockCurrentUser,
      name: '既存ユーザー',
      displayName: '@existing',
      about: '既存の自己紹介',
      picture: 'https://example.com/avatar.jpg',
      nip05: 'existing@example.com',
      avatar: null,
    };

    (useAuthStore as unknown as vi.Mock).mockReturnValue({
      currentUser: existingUser,
      updateUser: mockUpdateUser,
    });

    render(<ProfileSetup />, { wrapper: createWrapper() });

    expect(screen.getByDisplayValue('既存ユーザー')).toBeInTheDocument();
    expect(screen.getByDisplayValue('@existing')).toBeInTheDocument();
    expect(screen.getByDisplayValue('既存の自己紹介')).toBeInTheDocument();
    expect(screen.getByDisplayValue('https://example.com/avatar.jpg')).toBeInTheDocument();
    expect(screen.getByDisplayValue('existing@example.com')).toBeInTheDocument();
  });

  it('後で設定ボタンがクリックされた時、ホーム画面に遷移する', async () => {
    const user = userEvent.setup();

    render(<ProfileSetup />, { wrapper: createWrapper() });

    const skipButton = screen.getByRole('button', { name: '後で設定' });
    await user.click(skipButton);

    expect(mockNavigate).toHaveBeenCalledWith({ to: '/' });
  });

  it('名前が未入力の場合、エラーを表示する', async () => {
    const user = userEvent.setup();

    render(<ProfileSetup />, { wrapper: createWrapper() });

    const submitButton = screen.getByRole('button', { name: '設定を完了' });
    await user.click(submitButton);

    expect(toast.error).toHaveBeenCalledWith('名前を入力してください');
    expect(updateNostrMetadata).not.toHaveBeenCalled();
  });

  it('プロフィール設定に成功する', async () => {
    (updateNostrMetadata as vi.Mock).mockResolvedValue(undefined);
    const user = userEvent.setup();

    render(<ProfileSetup />, { wrapper: createWrapper() });

    // フォーム入力
    await user.type(screen.getByLabelText('名前 *'), 'テストユーザー');
    await user.type(screen.getByLabelText('表示名'), '@testuser');
    await user.type(screen.getByLabelText('自己紹介'), 'テストユーザーです');
    await user.type(screen.getByLabelText('アバター画像URL'), 'https://example.com/test.jpg');
    await user.type(screen.getByLabelText('NIP-05認証'), 'test@example.com');

    const submitButton = screen.getByRole('button', { name: '設定を完了' });
    await user.click(submitButton);

    // Nostrメタデータ更新が呼ばれる
    await waitFor(() => {
      expect(updateNostrMetadata).toHaveBeenCalledWith({
        name: 'テストユーザー',
        display_name: '@testuser',
        about: 'テストユーザーです',
        picture: 'https://example.com/test.jpg',
        nip05: 'test@example.com',
      });
    });

    // ローカルストア更新が呼ばれる
    await waitFor(() => {
      expect(mockUpdateUser).toHaveBeenCalledWith({
        name: 'テストユーザー',
        displayName: '@testuser',
        about: 'テストユーザーです',
        picture: 'https://example.com/test.jpg',
        nip05: 'test@example.com',
        avatar: null,
      });
    });

    // 成功メッセージ
    await waitFor(() => {
      expect(toast.success).toHaveBeenCalledWith('プロフィールを設定しました');
    });

    // ホーム画面への遷移
    await waitFor(() => {
      expect(mockNavigate).toHaveBeenCalledWith({ to: '/' });
    });
  });

  it('画像をアップロードして保存すると Tauri API が呼び出される', async () => {
    (updateNostrMetadata as vi.Mock).mockResolvedValue(undefined);
    const user = userEvent.setup();
    const mockBytes = Uint8Array.from([1, 2, 3, 4]);

    mockOpen.mockResolvedValue('C:/temp/avatar.png');
    mockReadBinaryFile.mockResolvedValue(mockBytes);
    mockUploadProfileAvatar.mockResolvedValue({
      npub: 'npub1test',
      blob_hash: 'hash123',
      format: 'image/png',
      size_bytes: mockBytes.byteLength,
      access_level: 'contacts_only',
      share_ticket: 'ticket-1',
      doc_version: 2,
      updated_at: '2025-11-03T00:00:00Z',
      content_sha256: 'deadbeef',
    });
    mockFetchProfileAvatar.mockResolvedValue({
      npub: 'npub1test',
      blob_hash: 'hash123',
      format: 'image/png',
      size_bytes: mockBytes.byteLength,
      access_level: 'contacts_only',
      share_ticket: 'ticket-1',
      doc_version: 2,
      updated_at: '2025-11-03T00:00:00Z',
      content_sha256: 'deadbeef',
      data_base64: 'AQIDBA==',
    });

    render(<ProfileSetup />, { wrapper: createWrapper() });

    await user.type(screen.getByLabelText('名前 *'), 'テストユーザー');
    await user.click(screen.getByRole('button', { name: /画像をアップロード/ }));
    await user.click(screen.getByRole('button', { name: '設定を完了' }));

    await waitFor(() => {
      expect(mockUploadProfileAvatar).toHaveBeenCalledWith({
        npub: 'npub1test',
        data: mockBytes,
        format: 'image/png',
        accessLevel: 'contacts_only',
      });
    });

    expect(mockFetchProfileAvatar).toHaveBeenCalledWith('npub1test');

    const expectedNostrUri = `iroh+avatar://profile_avatars?${new URLSearchParams({
      npub: 'npub1test',
      hash: 'hash123',
      v: '2',
    }).toString()}`;

    await waitFor(() => {
      expect(updateNostrMetadata).toHaveBeenCalledWith(
        expect.objectContaining({
          picture: expectedNostrUri,
        }),
      );
    });

    await waitFor(() => {
      expect(mockUpdateUser).toHaveBeenCalledWith(
        expect.objectContaining({
          picture: 'data:image/png;base64,AQIDBA==',
          avatar: expect.objectContaining({
            blobHash: 'hash123',
            docVersion: 2,
            nostrUri: expectedNostrUri,
          }),
        }),
      );
    });
  });

  it('表示名が未入力の場合、名前が使用される', async () => {
    (updateNostrMetadata as vi.Mock).mockResolvedValue(undefined);
    const user = userEvent.setup();

    render(<ProfileSetup />, { wrapper: createWrapper() });

    // 名前のみ入力
    await user.type(screen.getByLabelText('名前 *'), 'テストユーザー');

    const submitButton = screen.getByRole('button', { name: '設定を完了' });
    await user.click(submitButton);

    // display_nameには名前が使用される
    await waitFor(() => {
      expect(updateNostrMetadata).toHaveBeenCalledWith(
        expect.objectContaining({
          name: 'テストユーザー',
          display_name: 'テストユーザー', // 名前と同じ
        }),
      );
    });

    // ローカルストアも同様
    await waitFor(() => {
      expect(mockUpdateUser).toHaveBeenCalledWith(
        expect.objectContaining({
          name: 'テストユーザー',
          displayName: 'テストユーザー', // 名前と同じ
          avatar: null,
        }),
      );
    });
  });

  it('プロフィール設定に失敗した場合、エラーメッセージを表示する', async () => {
    const error = new Error('Failed to update metadata');
    (updateNostrMetadata as vi.Mock).mockRejectedValue(error);
    const user = userEvent.setup();

    render(<ProfileSetup />, { wrapper: createWrapper() });

    await user.type(screen.getByLabelText('名前 *'), 'テストユーザー');

    const submitButton = screen.getByRole('button', { name: '設定を完了' });
    await user.click(submitButton);

    // エラーメッセージ
    await waitFor(() => {
      expect(toast.error).toHaveBeenCalledWith('プロフィールの設定に失敗しました');
    });

    // errorHandlerが呼ばれる
    expect(errorHandler.log).toHaveBeenCalledWith('Profile setup failed', error, {
      context: 'ProfileSetup.handleSubmit',
    });

    // ナビゲーションは発生しない
    expect(mockNavigate).not.toHaveBeenCalled();

    // ローディング状態が解除される
    expect(screen.getByRole('button', { name: '設定を完了' })).not.toBeDisabled();
  });

  it('アバターのイニシャルが正しく表示される', async () => {
    const user = userEvent.setup();

    render(<ProfileSetup />, { wrapper: createWrapper() });

    // 名前を入力
    await user.type(screen.getByLabelText('名前 *'), 'Test User');

    // アバターフォールバックにイニシャルが表示される
    const avatarFallback = screen.getByText('TU');
    expect(avatarFallback).toBeInTheDocument();
  });

  it('複数単語の名前から正しくイニシャルを生成する', async () => {
    const user = userEvent.setup();

    render(<ProfileSetup />, { wrapper: createWrapper() });

    // 3単語以上の名前
    await user.type(screen.getByLabelText('名前 *'), 'John Paul Smith');

    // 最初の2文字のみ使用
    const avatarFallback = screen.getByText('JP');
    expect(avatarFallback).toBeInTheDocument();
  });
});
