import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { expect, test, vi } from 'vitest';

import { SmartReferenceText } from './SmartReferenceText';
import { type MentionAuthorView } from './types';

const PUBKEY = 'a'.repeat(64);

const MENTION_AUTHORS: Record<string, MentionAuthorView> = {
  [PUBKEY]: {
    pubkey: PUBKEY,
    label: 'Alice',
    displayName: 'Alice',
    name: 'alice',
    aboutPreview: 'Building things',
    picture: null,
  },
};

test('renders a resolved mention as a clickable chip', async () => {
  const user = userEvent.setup();
  const onOpenMention = vi.fn();
  render(
    <SmartReferenceText
      text={`hello @[Alice](${PUBKEY})`}
      mentionAuthors={MENTION_AUTHORS}
      onOpenMention={onOpenMention}
    />
  );

  const chip = screen.getByRole('button', { name: '@Alice' });
  await user.click(chip);
  expect(onOpenMention).toHaveBeenCalledWith(PUBKEY);
});

test('renders an unresolved mention as plain text', () => {
  render(<SmartReferenceText text={`hi @[Ghost](${PUBKEY})`} mentionAuthors={{}} />);

  expect(screen.queryByRole('button', { name: '@Ghost' })).not.toBeInTheDocument();
  expect(screen.getByText('@Ghost')).toBeInTheDocument();
});
