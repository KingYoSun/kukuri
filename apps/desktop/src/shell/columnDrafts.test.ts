import { describe, expect, it } from 'vitest';

import {
  columnDraftKey,
  createColumnDraft,
  isColumnDraftDirty,
  removeColumnDraft,
  setColumnDraft,
  type ColumnDraftTarget,
} from '@/shell/slices/columnDrafts';

const publicPost: ColumnDraftTarget = {
  columnId: 'timeline-public',
  action: 'post',
  scope: { topicId: 'topic-a', channelId: null },
};

describe('column Draft state', () => {
  it('separates Draft keys by Column, action, scope, thread, and peer', () => {
    expect(columnDraftKey(publicPost)).not.toBe(
      columnDraftKey({
        ...publicPost,
        columnId: 'timeline-private',
        scope: { topicId: 'topic-a', channelId: 'friends' },
      })
    );
    expect(
      columnDraftKey({
        columnId: 'thread-a',
        action: 'reply',
        scope: { topicId: 'topic-a', channelId: 'friends' },
        threadId: 'thread-1',
      })
    ).not.toBe(
      columnDraftKey({
        columnId: 'conversation-a',
        action: 'message',
        peerPubkey: 'peer-a',
      })
    );
  });

  it('updates and clears only the addressed Draft', () => {
    const privatePost: ColumnDraftTarget = {
      columnId: 'timeline-private',
      action: 'post',
      scope: { topicId: 'topic-a', channelId: 'friends' },
    };
    let drafts = setColumnDraft({}, publicPost, (draft) => ({
      ...draft,
      content: 'public Draft',
      expanded: true,
    }));
    drafts = setColumnDraft(drafts, privatePost, (draft) => ({
      ...draft,
      content: 'private Draft',
    }));

    expect(drafts[columnDraftKey(publicPost)]?.content).toBe('public Draft');
    expect(drafts[columnDraftKey(privatePost)]?.content).toBe('private Draft');

    drafts = removeColumnDraft(drafts, publicPost);
    expect(drafts[columnDraftKey(publicPost)]).toBeUndefined();
    expect(drafts[columnDraftKey(privatePost)]?.content).toBe('private Draft');
  });

  it('treats content and attachments as dirty while an empty expanded Draft is clean', () => {
    expect(isColumnDraftDirty(createColumnDraft(publicPost))).toBe(false);
    expect(
      isColumnDraftDirty({
        ...createColumnDraft(publicPost),
        expanded: true,
      })
    ).toBe(false);
    expect(
      isColumnDraftDirty({
        ...createColumnDraft(publicPost),
        content: 'unsent',
      })
    ).toBe(true);
    expect(
      isColumnDraftDirty({
        ...createColumnDraft(publicPost),
        mediaItems: [
          {
            id: 'draft-image',
            source_name: 'image.png',
            preview_url: 'blob:image',
            attachments: [],
          },
        ],
      })
    ).toBe(true);
  });
});
