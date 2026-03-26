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
      onClearReply={vi.fn()}
    />
  );

  expect(screen.getByText('Replying')).toBeInTheDocument();
  expect(screen.getByRole('button', { name: 'Clear reply' })).toHaveClass('shell-icon-button');
  expect(screen.queryByRole('button', { name: 'Clear' })).not.toBeInTheDocument();
  expect(screen.queryByText('reply target body')).not.toBeInTheDocument();
  expect(screen.queryByText('Audience: Imported')).not.toBeInTheDocument();
});
