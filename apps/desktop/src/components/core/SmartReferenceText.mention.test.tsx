import { fireEvent, render, screen } from '@testing-library/react';
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

test('resolved mention copies its hidden author ID without replacing the click action', async () => {
  const user = userEvent.setup();
  const onOpenMention = vi.fn();
  const clipboardWriteText = vi.fn().mockResolvedValue(undefined);
  Object.defineProperty(navigator, 'clipboard', {
    configurable: true,
    value: { writeText: clipboardWriteText },
  });
  render(
    <SmartReferenceText
      text={`hello @[Alice](${PUBKEY})`}
      mentionAuthors={MENTION_AUTHORS}
      onOpenMention={onOpenMention}
    />
  );

  const chip = screen.getByRole('button', { name: '@Alice' });
  fireEvent.contextMenu(chip, { clientX: 24, clientY: 36 });
  await user.click(screen.getByRole('menuitem', { name: 'Copy author ID' }));
  expect(clipboardWriteText).toHaveBeenCalledWith(PUBKEY);
  expect(onOpenMention).not.toHaveBeenCalled();

  chip.focus();
  fireEvent.keyDown(chip, { key: 'ContextMenu' });
  fireEvent.keyDown(screen.getByRole('menuitem', { name: 'Copy author ID' }), { key: 'Escape' });
  expect(chip).toHaveFocus();

  await user.click(chip);
  expect(onOpenMention).toHaveBeenCalledWith(PUBKEY);
});

test('renders an unresolved mention as plain text', () => {
  render(<SmartReferenceText text={`hi @[Ghost](${PUBKEY})`} mentionAuthors={{}} />);

  expect(screen.queryByRole('button', { name: '@Ghost' })).not.toBeInTheDocument();
  expect(screen.getByText('@Ghost')).toBeInTheDocument();
});
