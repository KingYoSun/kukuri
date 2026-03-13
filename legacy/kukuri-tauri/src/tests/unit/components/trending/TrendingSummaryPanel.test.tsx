import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';

import { TrendingSummaryPanel } from '@/components/trending/TrendingSummaryPanel';

vi.mock('date-fns', async () => {
  const actual = await vi.importActual<typeof import('date-fns')>('date-fns');
  return {
    ...actual,
    formatDistanceToNow: vi.fn(() => '1分前'),
  };
});

const badgeMock = vi.hoisted(() => ({
  unreadTotal: 12,
  latestMessage: { createdAt: Date.now() },
  latestConversationNpub: 'npub1demo123',
}));

vi.mock('@/hooks/useDirectMessageBadge', () => ({
  useDirectMessageBadge: () => badgeMock,
}));

const openInboxMock = vi.fn();

vi.mock('@/stores/directMessageStore', () => ({
  useDirectMessageStore: (selector: (state: { openInbox: () => void }) => unknown) =>
    selector({ openInbox: openInboxMock }),
}));

describe('TrendingSummaryPanel', () => {
  beforeEach(() => {
    openInboxMock.mockClear();
  });

  it('DMカードで未読件数と最新時刻を表示し、CTAでInboxを開く', async () => {
    const user = userEvent.setup();

    render(
      <TrendingSummaryPanel
        topics={{ generatedAt: Date.now(), topics: [] }}
        posts={{ topics: [], generatedAt: Date.now() }}
      />,
    );

    expect(screen.getByTestId('trending-summary-direct-messages')).toHaveTextContent('12件');
    expect(screen.getByTestId('trending-summary-direct-messages-helper')).toHaveTextContent(
      '1分前 / 会話: npub1demo123',
    );

    await user.click(screen.getByTestId('trending-summary-direct-messages-cta'));
    expect(openInboxMock).toHaveBeenCalledTimes(1);
  });
});
