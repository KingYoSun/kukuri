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
