/**
 * #965: 添付選択のハンドラ test。非対応ファイルは読み込まずに理由(ファイル名 + 対応形式)を
 * composer に出し、対応ファイルだけを下書きへ追加する契約を固定する。
 * ハーネスは testSupport/renderShellActions を共有する。
 */
import { act } from '@testing-library/react';
import { beforeEach, describe, expect, test, vi } from 'vitest';

import { columnDraftKey, setColumnDraft } from '@/shell/slices/columnDrafts';
import {
  attachmentChangeEvent,
  recordingTranslate,
  renderActionsHook,
} from '@/shell/testSupport/renderShellActions';
import { resetWindowHash } from '@/shell/testSupport/renderShellHook';

beforeEach(() => {
  resetWindowHash();
});

describe('useDesktopShellActions attachments (#965)', () => {
  test('clipboard images append to the targeted Column draft without publishing', async () => {
    const createPost = vi.fn();
    const target = {
      columnId: 'timeline-public',
      action: 'post' as const,
      scope: { topicId: 'topic-a', channelId: null },
    };
    const view = renderActionsHook({
      api: { createPost },
      preset: (current) => ({
        columnDraftsByKey: setColumnDraft(current.columnDraftsByKey, target, (draft) => ({
          ...draft,
          content: 'keep me',
          expanded: true,
        })),
      }),
    });
    const image = new File(['image'], 'clipboard.png', { type: 'image/png' });

    await act(async () => {
      await view.result.current.handleColumnDraftAttachmentPaste(target, [image]);
    });

    expect(view.store.getState().columnDraftsByKey[columnDraftKey(target)]).toMatchObject({
      content: 'keep me',
      mediaItems: [{ id: 'image-item-clipboard.png' }],
      error: null,
    });
    expect(view.mocks.buildImageDraftItem).toHaveBeenCalledWith(image);
    expect(view.mocks.rememberDraftPreview).toHaveBeenCalledTimes(1);
    expect(createPost).not.toHaveBeenCalled();
  });

  test('clipboard image failure keeps accepted Column drafts and reports an image error', async () => {
    const target = {
      columnId: 'timeline-public',
      action: 'post' as const,
      scope: { topicId: 'topic-a', channelId: null },
    };
    const view = renderActionsHook({ translate: recordingTranslate });
    view.mocks.buildImageDraftItem
      .mockResolvedValueOnce({
        id: 'accepted',
        source_name: 'accepted.png',
        preview_url: 'blob:accepted',
        attachments: [],
      })
      .mockRejectedValueOnce(new Error('read failed'));

    await act(async () => {
      await view.result.current.handleColumnDraftAttachmentPaste(target, [
        new File(['ok'], 'accepted.png', { type: 'image/png' }),
        new File(['bad'], 'broken.png', { type: 'image/png' }),
      ]);
    });

    expect(view.store.getState().columnDraftsByKey[columnDraftKey(target)]).toMatchObject({
      mediaItems: [{ id: 'accepted' }],
      error: 'common:errors.failedToPrepareImageAttachment',
    });
    expect(view.mocks.rememberDraftPreview).toHaveBeenCalledTimes(1);
  });

  test('clipboard image replaces the auxiliary DM draft without sending', async () => {
    const sendDirectMessage = vi.fn();
    const view = renderActionsHook({
      api: { sendDirectMessage },
      preset: () => ({
        directMessageDraftMediaItems: [
          {
            id: 'old-image',
            source_name: 'old.png',
            preview_url: 'blob:old',
            attachments: [],
          },
        ],
      }),
    });
    const image = new File(['new'], 'new.png', { type: 'image/png' });

    await act(async () => {
      await view.result.current.handleDirectMessageAttachmentPaste([image]);
    });

    expect(view.store.getState()).toMatchObject({
      directMessageDraftMediaItems: [{ id: 'image-item-new.png' }],
      directMessageError: null,
    });
    expect(view.mocks.releaseAllDirectMessageDraftPreviews).toHaveBeenCalledTimes(1);
    expect(view.mocks.rememberDirectMessageDraftPreview).toHaveBeenCalledTimes(1);
    expect(sendDirectMessage).not.toHaveBeenCalled();
  });

  test('clipboard image failure keeps the existing auxiliary DM draft', async () => {
    const view = renderActionsHook({
      preset: () => ({
        directMessageDraftMediaItems: [
          {
            id: 'old-image',
            source_name: 'old.png',
            preview_url: 'blob:old',
            attachments: [],
          },
        ],
      }),
    });
    view.mocks.buildImageDraftItem.mockRejectedValueOnce(new Error('read failed'));

    await act(async () => {
      await view.result.current.handleDirectMessageAttachmentPaste([
        new File(['bad'], 'broken.png', { type: 'image/png' }),
      ]);
    });

    expect(view.store.getState()).toMatchObject({
      directMessageDraftMediaItems: [{ id: 'old-image' }],
      directMessageError: 'common:errors.failedToPrepareImageAttachment',
    });
    expect(view.mocks.releaseAllDirectMessageDraftPreviews).not.toHaveBeenCalled();
    expect(view.mocks.rememberDirectMessageDraftPreview).not.toHaveBeenCalled();
  });

  // #965: 非対応ファイルは読み込まずに理由(ファイル名 + 対応形式)を composer に出し、
  // 対応ファイルだけを下書きへ追加する。
  test('unsupported Column Draft attachments show one reason with the rejected count and are never read', async () => {
    const readAsDataURL = vi.spyOn(FileReader.prototype, 'readAsDataURL');
    const target = {
      columnId: 'timeline-public',
      action: 'post' as const,
      scope: { topicId: 'topic-a', channelId: null },
    };
    const view = renderActionsHook({
      translate: recordingTranslate,
      preset: (current) => ({
        columnDraftsByKey: setColumnDraft(current.columnDraftsByKey, target, (draft) => ({
          ...draft,
          content: 'keep me',
          expanded: true,
        })),
      }),
    });

    await act(async () => {
      await view.result.current.handleColumnDraftAttachmentSelection(
        target,
        attachmentChangeEvent([
          new File(['notes'], 'notes.txt', { type: 'text/plain' }),
          new File(['image'], 'photo.png', { type: 'image/png' }),
          new File(['pdf'], 'report.pdf', { type: 'application/pdf' }),
          new File(['?'], 'unknown.bin', { type: '' }),
        ])
      );
    });

    const draft = view.store.getState().columnDraftsByKey[columnDraftKey(target)];
    expect(draft).toMatchObject({
      content: 'keep me',
      expanded: true,
      pending: false,
      attachmentInputKey: 1,
      mediaItems: [{ id: 'image-item-photo.png' }],
      error: 'common:errors.unsupportedAttachmentTypes:{"name":"notes.txt","others":2}',
    });
    expect(view.mocks.buildImageDraftItem).toHaveBeenCalledTimes(1);
    expect(view.mocks.buildVideoDraftItem).not.toHaveBeenCalled();
    expect(readAsDataURL).not.toHaveBeenCalled();

    await act(async () => {
      await view.result.current.handleColumnDraftAttachmentSelection(
        target,
        attachmentChangeEvent([new File(['notes'], 'only.txt', { type: 'text/plain' })])
      );
    });
    expect(view.store.getState().columnDraftsByKey[columnDraftKey(target)]).toMatchObject({
      attachmentInputKey: 2,
      mediaItems: [{ id: 'image-item-photo.png' }],
      error: 'common:errors.unsupportedAttachmentType:{"name":"only.txt"}',
    });
    expect(view.mocks.buildImageDraftItem).toHaveBeenCalledTimes(1);
    readAsDataURL.mockRestore();
  });

  test('unsupported DM attachment shows the same reason and keeps the DM draft untouched', async () => {
    const view = renderActionsHook({ translate: recordingTranslate });

    await act(async () => {
      await view.result.current.handleDirectMessageAttachmentSelection(
        attachmentChangeEvent([new File(['notes'], 'notes.txt', { type: 'text/plain' })])
      );
    });

    expect(view.store.getState()).toMatchObject({
      directMessageError: 'common:errors.unsupportedAttachmentType:{"name":"notes.txt"}',
      directMessageDraftMediaItems: [],
      directMessageAttachmentInputKey: 1,
    });
    expect(view.mocks.buildImageDraftItem).not.toHaveBeenCalled();
    expect(view.mocks.buildVideoDraftItem).not.toHaveBeenCalled();
    expect(view.mocks.rememberDirectMessageDraftPreview).not.toHaveBeenCalled();
  });
});
