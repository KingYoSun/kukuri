import { render, screen } from '@testing-library/react';
import { expect, test, vi } from 'vitest';

import { ComposerPanel } from './ComposerPanel';

test('reply banner keeps only the replying label and a compact clear icon action', () => {
  render(
    <ComposerPanel
      value=''
      onChange={() => undefined}
      onSubmit={(event) => event.preventDefault()}
      attachmentInputKey={0}
      onAttachmentSelection={() => undefined}
      draftMediaItems={[]}
      onRemoveDraftAttachment={() => undefined}
      composerError={null}
      audienceLabel='Public'
      replyTarget={{ content: 'reply target body', audienceLabel: 'Imported' }}
      sourcePreview={{
        post: {
          object_id: 'post-1',
          envelope_id: 'envelope-post-1',
          author_pubkey: 'a'.repeat(64),
          author_name: 'alice',
          author_display_name: 'Alice',
          following: false,
          followed_by: false,
          mutual: false,
          friend_of_friend: false,
          object_kind: 'post',
          content: 'reply target body',
          content_status: 'Available',
          attachments: [],
          created_at: 1,
          reply_to: null,
          root_id: 'post-1',
          published_topic_id: null,
          origin_topic_id: null,
          repost_of: null,
          repost_commentary: null,
          is_threadable: true,
          channel_id: null,
          audience_label: 'Imported',
          reaction_summary: [],
          my_reactions: [],
        },
        context: 'timeline',
        authorLabel: 'Alice',
        authorPicture: null,
        relationshipLabel: null,
        audienceChipLabel: 'Imported',
        threadTargetId: 'post-1',
        media: {
          objectId: 'post-1',
          kind: null,
          statusLabel: null,
          extraAttachmentCount: 0,
          state: 'ready',
          metaMime: null,
          metaBytesLabel: null,
          imagePreviewSrc: null,
          videoPosterPreviewSrc: null,
          videoPlaybackSrc: null,
          videoUnsupportedOnClient: false,
        },
      }}
      onClearReply={vi.fn()}
    />
  );

  expect(screen.getByText('Replying')).toBeInTheDocument();
  expect(screen.getByRole('button', { name: 'Clear reply' })).toHaveClass('shell-icon-button');
  expect(screen.queryByRole('button', { name: 'Clear' })).not.toBeInTheDocument();
  expect(screen.getByText('Original post')).toBeInTheDocument();
  expect(screen.getByText('reply target body')).toBeInTheDocument();
  expect(screen.getAllByText('Imported').length).toBeGreaterThan(0);
});
