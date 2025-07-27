import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { ProfileSetup } from '../ProfileSetup';
import { useAuthStore } from '@/stores/authStore';
import { updateNostrMetadata } from '@/lib/api/nostr';
import { toast } from 'sonner';

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
  };
  
  beforeEach(() => {
    vi.clearAllMocks();
    (useAuthStore as unknown as vi.Mock).mockReturnValue({
      currentUser: mockCurrentUser,
      updateUser: mockUpdateUser,
    });
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('プロフィール設定フォームが正しく表示される', () => {
    render(<ProfileSetup />);

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
    };
    
    (useAuthStore as unknown as vi.Mock).mockReturnValue({
      currentUser: existingUser,
      updateUser: mockUpdateUser,
    });
    
    render(<ProfileSetup />);

    expect(screen.getByDisplayValue('既存ユーザー')).toBeInTheDocument();
    expect(screen.getByDisplayValue('@existing')).toBeInTheDocument();
    expect(screen.getByDisplayValue('既存の自己紹介')).toBeInTheDocument();
    expect(screen.getByDisplayValue('https://example.com/avatar.jpg')).toBeInTheDocument();
    expect(screen.getByDisplayValue('existing@example.com')).toBeInTheDocument();
  });

  it('後で設定ボタンがクリックされた時、ホーム画面に遷移する', async () => {
    const user = userEvent.setup();
    
    render(<ProfileSetup />);
    
    const skipButton = screen.getByRole('button', { name: '後で設定' });
    await user.click(skipButton);
    
    expect(mockNavigate).toHaveBeenCalledWith({ to: '/' });
  });

  it('名前が未入力の場合、エラーを表示する', async () => {
    const user = userEvent.setup();
    
    render(<ProfileSetup />);
    
    const submitButton = screen.getByRole('button', { name: '設定を完了' });
    await user.click(submitButton);
    
    expect(toast.error).toHaveBeenCalledWith('名前を入力してください');
    expect(updateNostrMetadata).not.toHaveBeenCalled();
  });

  it('プロフィール設定に成功する', async () => {
    (updateNostrMetadata as vi.Mock).mockResolvedValue(undefined);
    const user = userEvent.setup();
    
    render(<ProfileSetup />);
    
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

  it('表示名が未入力の場合、名前が使用される', async () => {
    (updateNostrMetadata as vi.Mock).mockResolvedValue(undefined);
    const user = userEvent.setup();
    
    render(<ProfileSetup />);
    
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
        })
      );
    });
    
    // ローカルストアも同様
    await waitFor(() => {
      expect(mockUpdateUser).toHaveBeenCalledWith(
        expect.objectContaining({
          name: 'テストユーザー',
          displayName: 'テストユーザー', // 名前と同じ
        })
      );
    });
  });

  it('プロフィール設定に失敗した場合、エラーメッセージを表示する', async () => {
    const error = new Error('Failed to update metadata');
    (updateNostrMetadata as vi.Mock).mockRejectedValue(error);
    const consoleErrorSpy = vi.spyOn(console, 'error').mockImplementation(() => {});
    const user = userEvent.setup();
    
    render(<ProfileSetup />);
    
    await user.type(screen.getByLabelText('名前 *'), 'テストユーザー');
    
    const submitButton = screen.getByRole('button', { name: '設定を完了' });
    await user.click(submitButton);
    
    // エラーメッセージ
    await waitFor(() => {
      expect(toast.error).toHaveBeenCalledWith('プロフィールの設定に失敗しました');
    });
    
    // コンソールエラー
    expect(consoleErrorSpy).toHaveBeenCalledWith('Profile setup failed:', error);
    
    // ナビゲーションは発生しない
    expect(mockNavigate).not.toHaveBeenCalled();
    
    // ローディング状態が解除される
    expect(screen.getByRole('button', { name: '設定を完了' })).not.toBeDisabled();
    
    consoleErrorSpy.mockRestore();
  });

  it('アバターのイニシャルが正しく表示される', async () => {
    const user = userEvent.setup();
    
    render(<ProfileSetup />);
    
    // 名前を入力
    await user.type(screen.getByLabelText('名前 *'), 'Test User');
    
    // アバターフォールバックにイニシャルが表示される
    const avatarFallback = screen.getByText('TU');
    expect(avatarFallback).toBeInTheDocument();
  });

  it('複数単語の名前から正しくイニシャルを生成する', async () => {
    const user = userEvent.setup();
    
    render(<ProfileSetup />);
    
    // 3単語以上の名前
    await user.type(screen.getByLabelText('名前 *'), 'John Paul Smith');
    
    // 最初の2文字のみ使用
    const avatarFallback = screen.getByText('JP');
    expect(avatarFallback).toBeInTheDocument();
  });
});