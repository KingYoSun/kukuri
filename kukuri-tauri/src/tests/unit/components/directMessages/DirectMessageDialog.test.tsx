import { beforeEach, afterEach, describe, expect, it, vi } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import type { ReactElement } from 'react';
import userEvent from '@testing-library/user-event';
import { DirectMessageDialog } from '@/components/directMessages/DirectMessageDialog';
import { useDirectMessageStore, getDirectMessageInitialState } from '@/stores/directMessageStore';
import { TauriApi } from '@/lib/api/tauri';
import { toast } from 'sonner';

type AuthStoreState = {
  currentUser: {
    id: string;
    npub: string;
    pubkey: string;
    name: string;
    displayName: string;
    about: string;
    picture: string;
    nip05: string;
  } | null;
};

const defaultCurrentUser = {
  id: 'current-user',
  npub: 'npub1current',
  pubkey: 'pubkey-current',
  name: 'current-name',
  displayName: 'Current User',
  about: '',
  picture: '',
  nip05: '',
};

var mockAuthState: AuthStoreState = {
  currentUser: { ...defaultCurrentUser },
};
var useAuthStoreMock: ReturnType<typeof vi.fn>;

vi.mock('@/stores/authStore', () => {
  useAuthStoreMock = vi.fn((selector?: (state: AuthStoreState) => unknown) =>
    selector ? selector(mockAuthState) : mockAuthState,
  );
  return {
    useAuthStore: useAuthStoreMock,
  };
});

vi.mock('@/lib/api/tauri', () => ({
  TauriApi: {
    sendDirectMessage: vi.fn(),
    listDirectMessages: vi.fn(),
    listDirectMessageConversations: vi.fn(),
    markDirectMessageConversationRead: vi.fn(),
  },
}));

var toastMock: { success: ReturnType<typeof vi.fn>; error: ReturnType<typeof vi.fn> };

vi.mock('sonner', () => {
  toastMock = {
    success: vi.fn(),
    error: vi.fn(),
  };
  return { toast: toastMock };
});

const targetNpub = 'npub1target';

const createQueryClient = () =>
  new QueryClient({
    defaultOptions: {
      queries: {
        retry: false,
        gcTime: Infinity,
      },
    },
  });

const renderWithQueryClient = (ui: ReactElement, client = createQueryClient()) => {
  return {
    client,
    ...render(<QueryClientProvider client={client}>{ui}</QueryClientProvider>),
  };
};

const openDialog = (draft = '') => {
  const baseState = getDirectMessageInitialState();
  useDirectMessageStore.setState({
    ...baseState,
    isDialogOpen: true,
    activeConversationNpub: targetNpub,
    messageDraft: draft,
  });
};

describe('DirectMessageDialog', () => {
  beforeEach(() => {
    mockAuthState.currentUser = { ...defaultCurrentUser };
    if (useAuthStoreMock) {
      useAuthStoreMock.mockClear();
    }
    toast.success.mockClear();
    toast.error.mockClear();
    vi.mocked(TauriApi.sendDirectMessage).mockReset();
    vi.mocked(TauriApi.listDirectMessages).mockReset();
    vi.mocked(TauriApi.listDirectMessageConversations).mockReset();
    vi.mocked(TauriApi.markDirectMessageConversationRead).mockReset();
    vi.mocked(TauriApi.listDirectMessages).mockResolvedValue({
      items: [],
      nextCursor: null,
      hasMore: false,
    });
    vi.mocked(TauriApi.listDirectMessageConversations).mockResolvedValue({ items: [] });
    vi.mocked(TauriApi.markDirectMessageConversationRead).mockResolvedValue();
    useDirectMessageStore.setState(getDirectMessageInitialState());
    if (typeof useDirectMessageStore.getState().setDraft !== 'function') {
      throw new Error('setDraft is not initialized');
    }
  });

  afterEach(() => {
    useDirectMessageStore.setState(getDirectMessageInitialState());
  });

  it('閉じているときは描画されない', () => {
    const { container } = renderWithQueryClient(<DirectMessageDialog />);
    expect(container).toBeEmptyDOMElement();
    expect(screen.queryByText('ダイレクトメッセージ')).not.toBeInTheDocument();
  });

  it('ダイアログ表示時に入力へフォーカスする', async () => {
    openDialog();
    renderWithQueryClient(<DirectMessageDialog />);

    await waitFor(() => expect(TauriApi.listDirectMessages).toHaveBeenCalled());

    const input = await screen.findByTestId('direct-message-input');
    await waitFor(() => expect(input).toHaveFocus());
  });

  it('既存メッセージが表示される', async () => {
    vi.mocked(TauriApi.listDirectMessages).mockResolvedValue({
      items: [
        {
          eventId: 'evt-1',
          clientMessageId: 'client-1',
          senderNpub: targetNpub,
          recipientNpub: mockAuthState.currentUser!.npub,
          content: 'こんにちは',
          createdAt: 1_730_000_000_000,
          delivered: true,
        },
      ],
      nextCursor: null,
      hasMore: false,
    });

    openDialog();
    renderWithQueryClient(<DirectMessageDialog />);

    await waitFor(() => expect(TauriApi.listDirectMessages).toHaveBeenCalled());

    expect(await screen.findByText('ダイレクトメッセージ')).toBeInTheDocument();
    await waitFor(() =>
      expect(useDirectMessageStore.getState().conversations[targetNpub]).toHaveLength(1),
    );
    await waitFor(() =>
      expect(
        screen.queryByText('まだメッセージはありません。最初のメッセージを送信してみましょう。'),
      ).not.toBeInTheDocument(),
    );
    expect(
      screen.getByText((content) => typeof content === 'string' && content.includes('こんにちは')),
    ).toBeInTheDocument();
    expect(screen.getByText(`宛先: ${targetNpub}`)).toBeInTheDocument();
  });

  it('メッセージ送信で Tauri API を呼び出しストアが更新される', async () => {
    openDialog('test message');
    vi.mocked(TauriApi.sendDirectMessage).mockResolvedValue({
      eventId: 'evt-123',
      queued: false,
    });

    const user = userEvent.setup();
    renderWithQueryClient(<DirectMessageDialog />);

    await waitFor(() => expect(TauriApi.listDirectMessages).toHaveBeenCalled());

    expect(useAuthStoreMock).toHaveBeenCalled();
    expect(useDirectMessageStore.getState().messageDraft).toBe('test message');

    const sendButton = screen.getByRole('button', { name: '送信' });
    expect(sendButton).toBeEnabled();
    await user.click(sendButton);

    await waitFor(() =>
      expect(TauriApi.sendDirectMessage).toHaveBeenCalledWith(
        expect.objectContaining({
          recipientNpub: targetNpub,
          content: 'test message',
        }),
      ),
    );

    const conversation = useDirectMessageStore.getState().conversations[targetNpub] ?? [];
    expect(conversation).toHaveLength(1);
    expect(conversation[0].status).toBe('sent');
    expect(conversation[0].content).toBe('test message');
    expect(conversation[0].eventId).toBe('evt-123');

    expect(toast.success).toHaveBeenCalledWith('メッセージを送信しました。');
    expect(toast.error).not.toHaveBeenCalled();
    expect(screen.getByText('test message')).toBeInTheDocument();
  });
});
