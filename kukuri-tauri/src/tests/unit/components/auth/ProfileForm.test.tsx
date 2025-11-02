import { describe, it, expect, vi } from 'vitest';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { ProfileForm, type ProfileFormValues } from '@/components/auth/ProfileForm';

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
});
