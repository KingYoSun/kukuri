import { beforeEach, afterEach, describe, expect, it, vi } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import type { ReactElement } from 'react';
import userEvent from '@testing-library/user-event';
import { DirectMessageDialog } from '@/components/directMessages/DirectMessageDialog';
import { useDirectMessageStore, getDirectMessageInitialState } from '@/stores/directMessageStore';
import { TauriApi } from '@/lib/api/tauri';
import { toast } from 'sonner';
import { errorHandler } from '@/lib/errorHandler';
import type { ZustandStoreMock } from '@/tests/utils/zustandTestUtils';

vi.mock('@/lib/errorHandler', () => ({
  errorHandler: {
    log: vi.fn(),
  },
}));

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

var defaultCurrentUser = {
  id: 'current-user',
  npub: 'npub1current',
  pubkey: 'pubkey-current',
  name: 'current-name',
  displayName: 'Current User',
  about: '',
  picture: '',
  nip05: '',
};

function createAuthStoreState(): AuthStoreState {
  return {
    currentUser: { ...defaultCurrentUser },
  };
}

var authStoreMock: ZustandStoreMock<AuthStoreState>;

const requireCurrentUser = () => {
  const user = authStoreMock.getState().currentUser;
  if (!user) {
    throw new Error('Auth store user is not initialized');
  }
  return user;
};

vi.mock('@/stores/authStore', async () => {
  const { createZustandStoreMock } = await vi.importActual<
    typeof import('@/tests/utils/zustandTestUtils')
  >('@/tests/utils/zustandTestUtils');

  authStoreMock = createZustandStoreMock<AuthStoreState>(createAuthStoreState);

  return {
    useAuthStore: authStoreMock.hook,
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

vi.mock('sonner', async () => {
  const { createToastMock } =
    await vi.importActual<typeof import('@/tests/utils/toastMock')>('@/tests/utils/toastMock');
  return { toast: createToastMock() };
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
    authStoreMock.apply();
    authStoreMock.hook.mockClear();
    toast.success.mockClear();
    toast.error.mockClear();
    errorHandler.log.mockClear();
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
          recipientNpub: requireCurrentUser().npub,
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

    expect(authStoreMock.hook).toHaveBeenCalled();
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

  it('送信失敗時に失敗状態とエラーを記録する', async () => {
    openDialog('failure case');
    const sendError = new Error('offline');
    vi.mocked(TauriApi.sendDirectMessage).mockRejectedValueOnce(sendError);

    const user = userEvent.setup();
    renderWithQueryClient(<DirectMessageDialog />);

    await waitFor(() => expect(TauriApi.listDirectMessages).toHaveBeenCalled());

    await user.click(screen.getByRole('button', { name: '送信' }));

    await waitFor(() =>
      expect(errorHandler.log).toHaveBeenCalledWith(
        'DirectMessageDialog.sendFailed',
        sendError,
        expect.objectContaining({
          context: 'DirectMessageDialog.handleSend',
          metadata: expect.objectContaining({
            recipient: targetNpub,
          }),
        }),
      ),
    );
    expect(toast.error).toHaveBeenCalledWith('メッセージの送信に失敗しました。');
    await waitFor(() =>
      expect(useDirectMessageStore.getState().optimisticMessages[targetNpub]).toHaveLength(1),
    );
    expect(useDirectMessageStore.getState().optimisticMessages[targetNpub]?.[0]?.status).toBe(
      'failed',
    );
  });

  it('失敗メッセージの再送操作で API を再び呼び出す', async () => {
    const user = userEvent.setup();
    const failedMessage = {
      eventId: null,
      clientMessageId: 'client-failed',
      senderNpub: defaultCurrentUser.npub,
      recipientNpub: targetNpub,
      content: 'retry me',
      createdAt: 1_730_000_000_100,
      status: 'failed' as const,
    };
    useDirectMessageStore.setState((state) => ({
      ...state,
      isDialogOpen: true,
      activeConversationNpub: targetNpub,
      optimisticMessages: {
        ...state.optimisticMessages,
        [targetNpub]: [failedMessage],
      },
    }));

    renderWithQueryClient(<DirectMessageDialog />);

    const retryButton = await screen.findByTestId('direct-message-retry-button');
    vi.mocked(TauriApi.sendDirectMessage).mockResolvedValueOnce({
      eventId: 'evt-retry',
      queued: false,
    });

    await user.click(retryButton);

    await waitFor(() =>
      expect(TauriApi.sendDirectMessage).toHaveBeenLastCalledWith(
        expect.objectContaining({
          recipientNpub: targetNpub,
          content: 'retry me',
        }),
      ),
    );
    expect(toast.success).toHaveBeenCalledWith('メッセージを送信しました。');
  });

  it('既存メッセージがある場合は既読同期を行う', async () => {
    const existingMessage = {
      eventId: 'evt-1',
      clientMessageId: 'client-1',
      senderNpub: targetNpub,
      recipientNpub: requireCurrentUser().npub,
      content: 'hello there',
      createdAt: 1_730_000_000_500,
      status: 'sent' as const,
    };
    openDialog();
    useDirectMessageStore.setState((state) => ({
      ...state,
      conversations: {
        ...state.conversations,
        [targetNpub]: [existingMessage],
      },
    }));

    renderWithQueryClient(<DirectMessageDialog />);

    await waitFor(() =>
      expect(TauriApi.markDirectMessageConversationRead).toHaveBeenCalledWith({
        conversationNpub: targetNpub,
        lastReadAt: existingMessage.createdAt,
      }),
    );
    await waitFor(() =>
      expect(useDirectMessageStore.getState().conversationReadTimestamps[targetNpub]).toBe(
        existingMessage.createdAt,
      ),
    );
  });
});
