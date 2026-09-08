import { act, fireEvent, render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { createInstance } from 'i18next';
import { I18nextProvider } from 'react-i18next';
import { expect, test, vi } from 'vitest';

import { resources } from '@/i18n';
import { ComposerPanel } from './ComposerPanel';
import type { ComposerDraftMediaView } from './types';

const item: ComposerDraftMediaView = {
  id: 'video', sourceName: '長い動画ファイル.mp4', previewUrl: 'data:image/png;base64,AA==',
  attachments: [
    { key: 'video', label: 'video_manifest', mime: 'video/mp4', byteSizeLabel: '1 KB' },
    { key: 'poster', label: 'video_poster', mime: 'image/jpeg', byteSizeLabel: '1 KB' },
  ],
};

async function setup(locale = 'ja') {
  const i18n = createInstance();
  await i18n.init({ resources, lng: locale, fallbackLng: 'en', defaultNS: 'common' });
  const props = {
    value: 'draft', onChange: vi.fn(), onSubmit: vi.fn((event) => event.preventDefault()),
    attachmentInputKey: 0, onAttachmentSelection: vi.fn(), draftMediaItems: [] as ComposerDraftMediaView[],
    onRemoveDraftAttachment: vi.fn(), audienceLabel: 'Public', onClearReply: vi.fn(),
    attachmentsDisabled: false,
  };
  const view = render(<I18nextProvider i18n={i18n}><ComposerPanel {...props} /></I18nextProvider>);
  const update = (next: Partial<typeof props>) => {
    Object.assign(props, next);
    view.rerender(<I18nextProvider i18n={i18n}><ComposerPanel {...props} /></I18nextProvider>);
  };
  return { ...view, props, update, i18n };
}

test('Japanese attachment control owns visible copy and follows locale changes with a draft', async () => {
  const view = await setup();
  expect(screen.getByRole('button', { name: 'ファイルを選択' })).toBeVisible();
  expect(screen.getByText('ファイル未選択')).toBeVisible();
  expect(screen.getByLabelText('添付')).not.toBeVisible();
  view.update({ draftMediaItems: [item], attachmentInputKey: 1 });
  expect(screen.getByText('添付 1 件')).toBeVisible();
  expect(screen.getByText(item.sourceName)).toBeVisible();
  await act(() => view.i18n.changeLanguage('en'));
  expect(screen.getByRole('button', { name: 'Choose files' })).toBeVisible();
  expect(screen.getByText('Attached files: 1')).toBeVisible();
  await act(() => view.i18n.changeLanguage('zh-CN'));
  expect(screen.getByRole('button', { name: '选择文件' })).toBeVisible();
  expect(screen.getByText('已附加 1 个文件')).toBeVisible();
  expect(screen.getByDisplayValue('draft')).toBeVisible();
  view.update({ draftMediaItems: [] });
  expect(screen.getByText('未选择文件')).toBeVisible();
});

test.each(['click', 'Enter', 'Space'])('attachment %s activates its own picker without submitting', async (action) => {
  const view = await setup();
  const user = userEvent.setup();
  const input = screen.getByLabelText('添付');
  const clicked = vi.fn();
  input.addEventListener('click', clicked);
  const button = screen.getByRole('button', { name: 'ファイルを選択' });
  if (action === 'click') await user.click(button);
  else { button.focus(); await user.keyboard(action === 'Enter' ? '{Enter}' : ' '); }
  expect(clicked).toHaveBeenCalledTimes(1);
  expect(view.props.onSubmit).not.toHaveBeenCalled();
  fireEvent(input, new Event('cancel'));
  expect(button).toHaveFocus();
  expect(screen.getByText('ファイル未選択')).toBeVisible();
  expect(view.props.onAttachmentSelection).not.toHaveBeenCalled();
  view.update({ attachmentsDisabled: true });
  expect(button).toBeDisabled();
  expect(input).toBeDisabled();
  await user.click(button);
  expect(clicked).toHaveBeenCalledTimes(1);
});

test('input reset and removal derive counts from accepted drafts and allow the same file again', async () => {
  const view = await setup();
  const user = userEvent.setup();
  const file = new File(['image'], 'photo.png', { type: 'image/png' });
  await user.upload(screen.getByLabelText('添付'), file);
  view.update({ draftMediaItems: [item], attachmentInputKey: 1 });
  expect(screen.getByText('添付 1 件')).toBeVisible();
  expect(screen.getByLabelText('添付')).toHaveValue('');
  // A video's two internal attachments must count as one selected source file.
  await user.upload(screen.getByLabelText('添付'), file);
  expect(view.props.onAttachmentSelection).toHaveBeenCalledTimes(2);
  view.update({ draftMediaItems: [item, { ...item, id: 'second' }], attachmentInputKey: 2 });
  expect(screen.getByText('添付 2 件')).toBeVisible();
  await user.click(screen.getAllByRole('button', { name: '削除' })[0]);
  expect(view.props.onRemoveDraftAttachment).toHaveBeenCalledWith('video');
  view.update({ draftMediaItems: [] });
  expect(screen.getByText('ファイル未選択')).toBeVisible();
});

test('multiple composer instances only open the picker belonging to the activated composer', async () => {
  const view = await setup();
  view.rerender(<I18nextProvider i18n={view.i18n}>
    <section aria-label='first'><ComposerPanel {...view.props} /></section>
    <section aria-label='second'><ComposerPanel {...view.props} mode='message' /></section>
  </I18nextProvider>);
  const clicked = [vi.fn(), vi.fn()];
  ['first', 'second'].forEach((name, index) => {
    within(screen.getByRole('region', { name })).getByLabelText('添付').addEventListener('click', clicked[index]);
  });
  await userEvent.setup().click(within(screen.getByRole('region', { name: 'second' })).getByRole('button', { name: 'ファイルを選択' }));
  expect(clicked[0]).not.toHaveBeenCalled();
  expect(clicked[1]).toHaveBeenCalledTimes(1);
});
