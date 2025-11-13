import { beforeAll, afterEach, describe, expect, it, vi } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';

import { DirectMessageInbox } from '@/components/directMessages/DirectMessageInbox';
import { useDirectMessageStore, type DirectMessageModel } from '@/stores/directMessageStore';
import { useAuthStore } from '@/stores/authStore';
import { TauriApi } from '@/lib/api/tauri';

vi.mock('@tanstack/react-virtual', () => ({
  useVirtualizer: (options: { count: number }) => {
    return {
      getVirtualItems: () =>
        Array.from({ length: options.count }).map((_, index) => ({
          index,
          key: index,
          size: 88,
          start: index * 88,
        })),
      getTotalSize: () => options.count * 88,
      scrollToIndex: vi.fn(),
      measureElement: vi.fn(),
    };
  },
}));

vi.mock('@/lib/api/tauri', () => ({
  TauriApi: {
    searchUsers: vi.fn(),
    markDirectMessageConversationRead: vi.fn(),
  },
}));

type Mocked<T> = T extends (...args: infer P) => infer R ? vi.Mock<R, P> : never;
type SetupOptions = {
  conversations?: Record<string, DirectMessageModel[]>;
  unreadCounts?: Record<string, number>;
  readTimestamps?: Record<string, number>;
};

const mockSearchUsers = TauriApi.searchUsers as Mocked<typeof TauriApi.searchUsers>;
const mockMarkConversationRead = TauriApi.markDirectMessageConversationRead as Mocked<
  typeof TauriApi.markDirectMessageConversationRead
>;

const baseMessage = {
  eventId: 'evt-1',
  clientMessageId: 'client-1',
  senderNpub: 'npub1alice',
  recipientNpub: 'npub1tester',
  content: 'こんにちは',
  createdAt: Date.now(),
  status: 'sent' as const,
};

const originalGetBoundingClientRect = HTMLElement.prototype.getBoundingClientRect;

beforeAll(() => {
  (global as any).ResizeObserver = class {
    observe() {}
    unobserve() {}
    disconnect() {}
  };

  HTMLElement.prototype.getBoundingClientRect = function getBoundingClientRect() {
    return {
      width: 320,
      height: 80,
      top: 0,
      left: 0,
      bottom: 80,
      right: 320,
      x: 0,
      y: 0,
      toJSON() {
        return {};
      },
    };
  };
});

afterAll(() => {
  HTMLElement.prototype.getBoundingClientRect = originalGetBoundingClientRect;
});

const setupStore = (options: SetupOptions = {}) => {
  const closeInbox = vi.fn();
  const openDialog = vi.fn();
  const markConversationAsRead = vi.fn();

  useDirectMessageStore.setState((state) => ({
    ...state,
    isInboxOpen: true,
    closeInbox,
    openDialog,
    markConversationAsRead,
    conversations: options.conversations ?? {
      npub1alice: [baseMessage],
    },
    unreadCounts: options.unreadCounts ?? {
      npub1alice: 2,
    },
    conversationReadTimestamps: {
      ...state.conversationReadTimestamps,
      ...(options.readTimestamps ?? {}),
    },
  }));

  useAuthStore.setState({
    currentUser: {
      npub: 'npub1tester',
      pubkey: 'pubkeytester',
      id: 'tester',
      displayName: 'Tester',
      name: 'Tester',
      about: '',
      picture: '',
      nip05: '',
    } as any,
  });

  return { closeInbox, openDialog, markConversationAsRead };
};

describe('DirectMessageInbox', () => {
  const user = userEvent.setup();

  afterEach(() => {
    vi.clearAllMocks();
    useDirectMessageStore.getState().reset();
    useAuthStore.setState({ currentUser: null } as any);
  });

  it('renders conversation entries using virtualization', async () => {
    setupStore();
    render(<DirectMessageInbox />);

    expect(await screen.findByTestId('dm-inbox-list')).toBeInTheDocument();
    await waitFor(() =>
      expect(screen.getByTestId('dm-inbox-conversation-npub1alice')).toBeInTheDocument(),
    );
  });

  it('shows search suggestions and opens suggestion when clicked', async () => {
    const { openDialog, closeInbox } = setupStore();
    mockSearchUsers.mockResolvedValue({
      items: [
        {
          npub: 'npub1bob',
          pubkey: 'pubkeybob',
          name: 'Bob',
          display_name: 'Bob',
          about: '',
          picture: '',
          nip05: '',
          banner: null,
          website: null,
          is_profile_public: true,
          show_online_status: false,
        },
      ],
      nextCursor: null,
      hasMore: false,
      totalCount: 1,
      tookMs: 4,
    });

    render(<DirectMessageInbox />);

    const input = screen.getByTestId('dm-inbox-target-input');
    await user.type(input, 'bo');

    expect(await screen.findByTestId('dm-inbox-suggestions')).toBeInTheDocument();
    await user.click(screen.getByTestId('dm-inbox-suggestion-npub1bob'));

    expect(closeInbox).toHaveBeenCalledTimes(1);
    expect(openDialog).toHaveBeenCalledWith('npub1bob');
  });

  it('marks a conversation as read without opening it', async () => {
    const { markConversationAsRead } = setupStore();
    render(<DirectMessageInbox />);

    const markButton = await screen.findByTestId('dm-inbox-mark-read-npub1alice');
    await user.click(markButton);

    expect(markConversationAsRead).toHaveBeenCalledWith('npub1alice', baseMessage.createdAt);
    expect(mockMarkConversationRead).toHaveBeenCalledWith({
      conversationNpub: 'npub1alice',
      lastReadAt: baseMessage.createdAt,
    });
  });

  it('filters conversations via search input and opens the highlighted match with Enter', async () => {
    const secondMessage: DirectMessageModel = {
      ...baseMessage,
      eventId: 'evt-2',
      clientMessageId: 'client-2',
      senderNpub: 'npub1bob',
      recipientNpub: 'npub1tester',
      content: 'Bob からのメッセージ',
      createdAt: baseMessage.createdAt + 5_000,
    };
    const { openDialog, closeInbox } = setupStore({
      conversations: {
        npub1alice: [baseMessage],
        npub1bob: [secondMessage],
      },
      unreadCounts: {
        npub1alice: 0,
        npub1bob: 1,
      },
    });

    render(<DirectMessageInbox />);
    const searchInput = await screen.findByTestId('dm-inbox-conversation-search');
    await user.type(searchInput, 'bob');

    expect(screen.queryByTestId('dm-inbox-conversation-npub1alice')).not.toBeInTheDocument();
    expect(screen.getByTestId('dm-inbox-conversation-npub1bob')).toBeInTheDocument();

    await user.keyboard('{Enter}');

    expect(closeInbox).toHaveBeenCalledTimes(1);
    expect(openDialog).toHaveBeenCalledWith('npub1bob');
  });

  it('shows multi-device read indicators when lastReadAt is synced', async () => {
    setupStore({
      unreadCounts: { npub1alice: 0 },
      readTimestamps: { npub1alice: baseMessage.createdAt },
    });

    render(<DirectMessageInbox />);

    expect(await screen.findByTestId('dm-inbox-read-sync-npub1alice')).toBeInTheDocument();
    expect(screen.getByTestId('dm-inbox-read-receipt-npub1alice')).toHaveTextContent('既読同期');
  });
});
