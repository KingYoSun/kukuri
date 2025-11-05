import { beforeEach, afterEach, describe, expect, it, vi } from 'vitest';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import type { PropsWithChildren } from 'react';
import { ProfilePage, Route } from '@/routes/profile.$userId';
import { TauriApi } from '@/lib/api/tauri';
import { subscribeToUser } from '@/lib/api/nostr';
import { toast } from 'sonner';

vi.mock('@tanstack/react-router', async () => {
  const actual =
    await vi.importActual<typeof import('@tanstack/react-router')>('@tanstack/react-router');
  const Link = ({
    children,
    params: _params,
    ...rest
  }: PropsWithChildren<Record<string, unknown>>) => <a {...rest}>{children}</a>;

  return {
    ...actual,
    Link,
  };
});

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

var mockAuthState: AuthStoreState = {
  currentUser: null,
};
var useAuthStoreMock: ReturnType<typeof vi.fn>;

const mockDirectMessageStoreState = {
  openDialog: vi.fn(),
};
var useDirectMessageStoreMock: ReturnType<typeof vi.fn>;

vi.mock('@/stores', async () => {
  const actual = await vi.importActual<typeof import('@/stores')>('@/stores');
  useAuthStoreMock = vi.fn((selector?: (state: AuthStoreState) => unknown) =>
    selector ? selector(mockAuthState) : mockAuthState,
  );
  return {
    ...actual,
    useAuthStore: useAuthStoreMock,
  };
});

vi.mock('@/stores/directMessageStore', () => {
  useDirectMessageStoreMock = vi.fn(
    (selector?: (state: typeof mockDirectMessageStoreState) => unknown) =>
      selector ? selector(mockDirectMessageStoreState) : mockDirectMessageStoreState,
  );
  return {
    useDirectMessageStore: useDirectMessageStoreMock,
  };
});

vi.mock('@/lib/api/tauri', () => ({
  TauriApi: {
    getUserProfile: vi.fn(),
    getUserProfileByPubkey: vi.fn(),
    getPosts: vi.fn(),
    getFollowers: vi.fn(),
    getFollowing: vi.fn(),
    followUser: vi.fn(),
    unfollowUser: vi.fn(),
    sendDirectMessage: vi.fn(),
  },
}));

vi.mock('@/lib/api/nostr', () => ({
  subscribeToUser: vi.fn(),
}));

var toastMock: { success: ReturnType<typeof vi.fn>; error: ReturnType<typeof vi.fn> };

vi.mock('sonner', () => {
  toastMock = {
    success: vi.fn(),
    error: vi.fn(),
  };
  return { toast: toastMock };
});

const targetUserProfile = {
  npub: 'npub1target',
  pubkey: 'pubkey-target',
  name: 'target-name',
  display_name: 'ターゲットユーザー',
  about: '自己紹介',
  picture: null,
  banner: null,
  website: null,
  nip05: null,
};

const currentUserProfile = {
  id: 'current-user',
  npub: 'npub1current',
  pubkey: 'pubkey-current',
  name: 'current-name',
  displayName: '現ユーザー',
  about: '',
  picture: '',
  nip05: '',
};

const renderWithClient = (client: QueryClient) =>
  render(
    <QueryClientProvider client={client}>
      <ProfilePage />
    </QueryClientProvider>,
  );

describe('ProfilePage route', () => {
  let queryClient: QueryClient;
  let paramsSpy: ReturnType<typeof vi.spyOn>;

  beforeEach(() => {
    queryClient = new QueryClient({
      defaultOptions: {
        queries: { retry: false, gcTime: 0 },
      },
    });

    mockAuthState.currentUser = { ...currentUserProfile };
    mockDirectMessageStoreState.openDialog.mockClear();
    if (useAuthStoreMock) {
      useAuthStoreMock.mockClear();
    }
    if (useDirectMessageStoreMock) {
      useDirectMessageStoreMock.mockClear();
    }
    toast.success.mockClear();
    toast.error.mockClear();

    vi.mocked(TauriApi.getUserProfile).mockResolvedValue(targetUserProfile);
    vi.mocked(TauriApi.getUserProfileByPubkey).mockResolvedValue(null);
    vi.mocked(TauriApi.getPosts).mockResolvedValue([]);
    vi.mocked(TauriApi.getFollowers).mockResolvedValue({
      items: [],
      nextCursor: null,
      hasMore: false,
    });
    vi.mocked(TauriApi.getFollowing).mockResolvedValue({
      items: [],
      nextCursor: null,
      hasMore: false,
    });
    vi.mocked(TauriApi.followUser).mockResolvedValue(undefined);
    vi.mocked(TauriApi.unfollowUser).mockResolvedValue(undefined);
    vi.mocked(subscribeToUser).mockResolvedValue(undefined);

    paramsSpy = vi.spyOn(Route, 'useParams').mockReturnValue({ userId: targetUserProfile.npub });
  });

  afterEach(() => {
    paramsSpy.mockRestore();
    queryClient.clear();
  });

  it('フォローボタンで follow/unfollow がトリガーされる', async () => {
    const user = userEvent.setup();
    renderWithClient(queryClient);

    await waitFor(() =>
      expect(
        screen.getByText(targetUserProfile.display_name ?? targetUserProfile.npub),
      ).toBeInTheDocument(),
    );

    const followButton = await screen.findByRole('button', { name: 'フォロー' });
    await user.click(followButton);

    await waitFor(() =>
      expect(TauriApi.followUser).toHaveBeenCalledWith(
        currentUserProfile.npub,
        targetUserProfile.npub,
      ),
    );
    await waitFor(() =>
      expect(screen.getByRole('button', { name: 'フォロー中' })).toBeInTheDocument(),
    );
    expect(subscribeToUser).toHaveBeenCalledWith(targetUserProfile.pubkey);

    const followActiveButton = await screen.findByRole('button', { name: 'フォロー中' });
    await user.click(followActiveButton);

    await waitFor(() =>
      expect(TauriApi.unfollowUser).toHaveBeenCalledWith(
        currentUserProfile.npub,
        targetUserProfile.npub,
      ),
    );
    await waitFor(() =>
      expect(screen.getByRole('button', { name: 'フォロー' })).toBeInTheDocument(),
    );
  });

  it('メッセージボタンで openDialog が呼び出される', async () => {
    const user = userEvent.setup();
    renderWithClient(queryClient);

    await waitFor(() =>
      expect(
        screen.getByText(targetUserProfile.display_name ?? targetUserProfile.npub),
      ).toBeInTheDocument(),
    );

    const messageButton = await screen.findByRole('button', { name: 'メッセージ' });
    expect(messageButton).toBeEnabled();

    await user.click(messageButton);
    expect(mockDirectMessageStoreState.openDialog).toHaveBeenCalledWith(targetUserProfile.npub);
    expect(toast.error).not.toHaveBeenCalled();
  });

  it('未ログイン時はメッセージボタンが無効化される', async () => {
    mockAuthState.currentUser = null;
    renderWithClient(queryClient);

    await waitFor(() =>
      expect(
        screen.getByText(targetUserProfile.display_name ?? targetUserProfile.npub),
      ).toBeInTheDocument(),
    );

    const messageButton = await screen.findByRole('button', { name: 'メッセージ' });
    expect(messageButton).toBeDisabled();
  });
});
