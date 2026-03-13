import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import type { InfiniteData } from '@tanstack/react-query';

import { FollowingSummaryPanel } from '@/components/following/FollowingSummaryPanel';
import type { FollowingFeedPageResult } from '@/hooks/useTrendingFeeds';

vi.mock('date-fns', async () => {
  const actual = await vi.importActual<typeof import('date-fns')>('date-fns');
  return {
    ...actual,
    formatDistanceToNow: vi.fn(() => '1分前'),
  };
});

const badgeMock = vi.hoisted(() => ({
  unreadTotal: 5,
  latestMessage: { createdAt: Date.now() },
  latestConversationNpub: 'npub1following',
}));

vi.mock('@/hooks/useDirectMessageBadge', () => ({
  useDirectMessageBadge: () => badgeMock,
}));

const openInboxMock = vi.fn();

vi.mock('@/stores/directMessageStore', () => ({
  useDirectMessageStore: (selector: (state: { openInbox: () => void }) => unknown) =>
    selector({ openInbox: openInboxMock }),
}));

describe('FollowingSummaryPanel', () => {
  beforeEach(() => {
    openInboxMock.mockClear();
  });

  it('DMカードの未読情報とCTAを表示する', async () => {
    const user = userEvent.setup();
    const data: InfiniteData<FollowingFeedPageResult> = {
      pages: [
        {
          items: [],
          hasMore: false,
          nextCursor: null,
          serverTime: Date.now(),
        },
      ],
      pageParams: [],
    };

    render(<FollowingSummaryPanel data={data} />);

    expect(screen.getByTestId('following-summary-direct-messages')).toHaveTextContent('5件');
    expect(screen.getByTestId('following-summary-direct-messages-helper')).toHaveTextContent(
      '1分前 / 会話: npub1following',
    );

    await user.click(screen.getByTestId('following-summary-direct-messages-cta'));
    expect(openInboxMock).toHaveBeenCalledTimes(1);
  });
});
