import { useState } from 'react';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { expect, test, vi } from 'vitest';

import { ComposerPanel } from './ComposerPanel';
import { type MentionCandidate } from './types';

const ALICE = 'a'.repeat(64);
const BOB = 'b'.repeat(64);
const MENTION_CANDIDATES: MentionCandidate[] = [
  { pubkey: ALICE, label: 'Alice', displayName: 'Alice', name: 'alice', about: 'Bio', picture: null },
  { pubkey: BOB, label: 'Bob', displayName: 'Bob', name: 'bob', about: null, picture: null },
];

function MentionHarness({ candidates = MENTION_CANDIDATES }: { candidates?: MentionCandidate[] }) {
  const [value, setValue] = useState('');
  return (
    <ComposerPanel
      value={value}
      onChange={(event) => setValue(event.target.value)}
      onValueChange={setValue}
      mentionCandidates={candidates}
      onSubmit={(event) => event.preventDefault()}
      attachmentInputKey={0}
      onAttachmentSelection={() => undefined}
      draftMediaItems={[]}
      onRemoveDraftAttachment={() => undefined}
      audienceLabel='Public'
      onClearReply={() => undefined}
    />
  );
}

test('typing @ with a query shows matching mention candidates', async () => {
  const user = userEvent.setup();
  render(<MentionHarness />);

  await user.click(screen.getByRole('textbox'));
  await user.keyboard('@al');

  expect(screen.getByRole('listbox', { name: 'Mention suggestions' })).toBeInTheDocument();
  expect(screen.getByRole('option', { name: /Alice/ })).toBeInTheDocument();
  expect(screen.queryByRole('option', { name: /Bob/ })).not.toBeInTheDocument();
});

test('selecting a candidate with the keyboard inserts the mention token', async () => {
  const user = userEvent.setup();
  render(<MentionHarness />);

  const textarea = screen.getByRole('textbox') as HTMLTextAreaElement;
  await user.click(textarea);
  await user.keyboard('hi @al');
  await user.keyboard('{Enter}');

  expect(textarea.value).toBe(`hi @[Alice](${ALICE}) `);
  expect(screen.queryByRole('listbox')).not.toBeInTheDocument();
});

test('Escape closes the suggestion list', async () => {
  const user = userEvent.setup();
  render(<MentionHarness />);

  await user.click(screen.getByRole('textbox'));
  await user.keyboard('@al');
  expect(screen.getByRole('listbox')).toBeInTheDocument();

  await user.keyboard('{Escape}');
  expect(screen.queryByRole('listbox')).not.toBeInTheDocument();
});

test('a bare @ lists all candidates', async () => {
  const user = userEvent.setup();
  render(<MentionHarness />);

  await user.click(screen.getByRole('textbox'));
  await user.keyboard('@');

  expect(screen.getByRole('option', { name: /Alice/ })).toBeInTheDocument();
  expect(screen.getByRole('option', { name: /Bob/ })).toBeInTheDocument();
});

test('mention autocomplete stays inert without onValueChange', async () => {
  const user = userEvent.setup();
  render(
    <ComposerPanel
      value='@al'
      onChange={() => undefined}
      mentionCandidates={MENTION_CANDIDATES}
      onSubmit={(event) => event.preventDefault()}
      attachmentInputKey={0}
      onAttachmentSelection={() => undefined}
      draftMediaItems={[]}
      onRemoveDraftAttachment={() => undefined}
      audienceLabel='Public'
      onClearReply={() => undefined}
    />
  );

  await user.click(screen.getByRole('textbox'));
  expect(screen.queryByRole('listbox')).not.toBeInTheDocument();
});

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
