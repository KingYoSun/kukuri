import { describe, it, expect, vi, beforeAll, afterAll, beforeEach } from 'vitest';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { ProfileForm, type ProfileFormValues } from '@/components/auth/ProfileForm';
import { MAX_PROFILE_AVATAR_BYTES } from '@/lib/profile/avatar';
import { toast } from 'sonner';
import { errorHandler } from '@/lib/errorHandler';

const mockOpen = vi.fn();
const mockReadBinaryFile = vi.fn();

vi.mock('@tauri-apps/api/dialog', () => ({
  open: mockOpen,
}));

vi.mock('@tauri-apps/api/fs', () => ({
  readBinaryFile: mockReadBinaryFile,
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

const originalCreateObjectURL = global.URL.createObjectURL;
const originalRevokeObjectURL = global.URL.revokeObjectURL;

beforeAll(() => {
  global.URL.createObjectURL = vi.fn(() => 'blob:mock-url');
  global.URL.revokeObjectURL = vi.fn();
});

afterAll(() => {
  global.URL.createObjectURL = originalCreateObjectURL;
  global.URL.revokeObjectURL = originalRevokeObjectURL;
});

beforeEach(() => {
  vi.clearAllMocks();
});

describe('ProfileForm', () => {
  const initialValues: ProfileFormValues = {
    name: 'テスター',
    displayName: '@tester',
    about: '自己紹介',
    picture: 'https://example.com/avatar.png',
    nip05: 'tester@example.com',
  };

  it('初期値を表示する', () => {
    render(
      <ProfileForm
        initialValues={initialValues}
        onSubmit={vi.fn()}
        submitLabel="保存"
      />,
    );

    expect(screen.getByDisplayValue('テスター')).toBeInTheDocument();
    expect(screen.getByDisplayValue('@tester')).toBeInTheDocument();
    expect(screen.getByDisplayValue('自己紹介')).toBeInTheDocument();
    expect(screen.getByDisplayValue('https://example.com/avatar.png')).toBeInTheDocument();
    expect(screen.getByDisplayValue('tester@example.com')).toBeInTheDocument();
  });

  it('送信ボタンでonSubmitが呼ばれる', async () => {
    const user = userEvent.setup();
    const handleSubmit = vi.fn();

    render(
      <ProfileForm
        initialValues={initialValues}
        onSubmit={handleSubmit}
        submitLabel="保存"
      />,
    );

    await user.clear(screen.getByLabelText('名前 *'));
    await user.type(screen.getByLabelText('名前 *'), '新しい名前');

    await user.click(screen.getByRole('button', { name: '保存' }));

    expect(handleSubmit).toHaveBeenCalledWith({
      ...initialValues,
      name: '新しい名前',
      avatarFile: undefined,
    });
  });

  it('キャンセルボタンでonCancelが呼ばれる', async () => {
    const user = userEvent.setup();
    const handleCancel = vi.fn();

    render(
      <ProfileForm
        initialValues={initialValues}
        onSubmit={vi.fn()}
        onCancel={handleCancel}
        cancelLabel="キャンセル"
        submitLabel="保存"
      />,
    );

    await user.click(screen.getByRole('button', { name: 'キャンセル' }));

    expect(handleCancel).toHaveBeenCalledTimes(1);
  });

  it('スキップボタンでonSkipが呼ばれる', async () => {
    const user = userEvent.setup();
    const handleSkip = vi.fn();

    render(
      <ProfileForm
        initialValues={initialValues}
        onSubmit={vi.fn()}
        onSkip={handleSkip}
        skipLabel="後で設定"
        submitLabel="保存"
      />,
    );

    await user.click(screen.getByRole('button', { name: '後で設定' }));

    expect(handleSkip).toHaveBeenCalledTimes(1);
  });

  it('画像をアップロードして送信すると avatarFile が渡される', async () => {
    const user = userEvent.setup();
    const handleSubmit = vi.fn();
    const mockBytes = Uint8Array.from([1, 2, 3]);

    mockOpen.mockResolvedValue('C:/temp/avatar.png');
    mockReadBinaryFile.mockResolvedValue(mockBytes);

    render(
      <ProfileForm
        initialValues={initialValues}
        onSubmit={handleSubmit}
        submitLabel="保存"
      />,
    );

    await user.click(screen.getByRole('button', { name: /画像をアップロード/ }));
    await user.click(screen.getByRole('button', { name: '保存' }));

    expect(mockOpen).toHaveBeenCalledTimes(1);
    expect(mockReadBinaryFile).toHaveBeenCalledWith('C:/temp/avatar.png');
    expect(handleSubmit).toHaveBeenCalledWith(
      expect.objectContaining({
        avatarFile: expect.objectContaining({
          format: 'image/png',
          sizeBytes: mockBytes.byteLength,
          bytes: mockBytes,
          fileName: 'avatar.png',
        }),
      }),
    );
  });

  it('ファイル選択をキャンセルすると avatarFile は設定されない', async () => {
    const user = userEvent.setup();
    const handleSubmit = vi.fn();

    mockOpen.mockResolvedValue(null);

    render(
      <ProfileForm
        initialValues={initialValues}
        onSubmit={handleSubmit}
        submitLabel="保存"
      />,
    );

    await user.click(screen.getByRole('button', { name: /画像をアップロード/ }));
    await user.click(screen.getByRole('button', { name: '保存' }));

    expect(mockReadBinaryFile).not.toHaveBeenCalled();
    expect(handleSubmit).toHaveBeenCalledWith(expect.objectContaining({ avatarFile: undefined }));
  });

  it('許可サイズを超える画像は拒否される', async () => {
    const user = userEvent.setup();
    const handleSubmit = vi.fn();
    const largeBytes = new Uint8Array(MAX_PROFILE_AVATAR_BYTES + 1);

    mockOpen.mockResolvedValue('C:/temp/large.png');
    mockReadBinaryFile.mockResolvedValue(largeBytes);

    render(
      <ProfileForm
        initialValues={initialValues}
        onSubmit={handleSubmit}
        submitLabel="保存"
      />,
    );

    await user.click(screen.getByRole('button', { name: /画像をアップロード/ }));
    expect(toast.error).toHaveBeenCalledWith('画像サイズが大きすぎます（最大2MBまで）');

    await user.click(screen.getByRole('button', { name: '保存' }));
    expect(handleSubmit).toHaveBeenCalledWith(expect.objectContaining({ avatarFile: undefined }));
  });

  it('読み込みに失敗した場合はエラートーストとログを送出する', async () => {
    const user = userEvent.setup();
    const handleSubmit = vi.fn();
    const readError = new Error('read failure');

    mockOpen.mockResolvedValue('C:/temp/avatar.png');
    mockReadBinaryFile.mockRejectedValue(readError);

    render(
      <ProfileForm
        initialValues={initialValues}
        onSubmit={handleSubmit}
        submitLabel="保存"
      />,
    );

    await user.click(screen.getByRole('button', { name: /画像をアップロード/ }));
    expect(toast.error).toHaveBeenCalledWith('画像の読み込みに失敗しました');
    expect(errorHandler.log).toHaveBeenCalledWith(
      'ProfileForm.avatarLoadFailed',
      readError,
      expect.objectContaining({ context: 'ProfileForm.handleAvatarSelect' }),
    );

    await user.click(screen.getByRole('button', { name: '保存' }));
    expect(handleSubmit).toHaveBeenCalledWith(expect.objectContaining({ avatarFile: undefined }));
  });
});
