import { beforeEach, afterEach, describe, expect, it, vi } from 'vitest';
import { QueryClient, QueryClientProvider, type InfiniteData } from '@tanstack/react-query';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import type { PropsWithChildren } from 'react';
import { ProfilePage, Route } from '@/routes/profile.$userId';
import { TauriApi } from '@/lib/api/tauri';
import type { UserProfile as UserProfileDto } from '@/lib/api/tauri';
import { subscribeToUser } from '@/lib/api/nostr';
import { toast } from 'sonner';
import { errorHandler } from '@/lib/errorHandler';

vi.mock('@/lib/errorHandler', () => ({
  errorHandler: {
    log: vi.fn(),
    info: vi.fn(),
  },
}));

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

let followersMock: UserProfileDto[] = [];
let followingMock: UserProfileDto[] = [];

const renderWithClient = (client: QueryClient) =>
  render(
    <QueryClientProvider client={client}>
      <ProfilePage />
    </QueryClientProvider>,
  );

type ObserverRecord = { callback: IntersectionObserverCallback; element: Element | null };
const observerRecords: ObserverRecord[] = [];

class MockIntersectionObserver implements IntersectionObserver {
  readonly root: Element | null = null;
  readonly rootMargin = '';
  readonly thresholds = [];
  private callback: IntersectionObserverCallback;

  constructor(callback: IntersectionObserverCallback) {
    this.callback = callback;
  }

  observe = (element: Element) => {
    observerRecords.push({ callback: this.callback, element });
  };

  unobserve = (element: Element) => {
    const index = observerRecords.findIndex(
      (record) => record.callback === this.callback && record.element === element,
    );
    if (index >= 0) {
      observerRecords.splice(index, 1);
    }
  };

  disconnect = () => {
    for (let index = observerRecords.length - 1; index >= 0; index -= 1) {
      if (observerRecords[index].callback === this.callback) {
        observerRecords.splice(index, 1);
      }
    }
  };

  takeRecords = () => [];
}

const triggerIntersection = () => {
  observerRecords.forEach((record) => {
    const entry = {
      isIntersecting: true,
      target: record.element ?? document.createElement('div'),
      intersectionRatio: 1,
      time: Date.now(),
      boundingClientRect: {} as DOMRectReadOnly,
      intersectionRect: {} as DOMRectReadOnly,
      rootBounds: null,
    } as IntersectionObserverEntry;
    record.callback([entry], {} as IntersectionObserver);
  });
};

beforeAll(() => {
  vi.stubGlobal('IntersectionObserver', MockIntersectionObserver);
});

afterEach(() => {
  observerRecords.splice(0, observerRecords.length);
});

afterAll(() => {
  vi.unstubAllGlobals();
});

describe('ProfilePage route', () => {
  let queryClient: QueryClient;
  let paramsSpy: ReturnType<typeof vi.spyOn>;

  beforeEach(() => {
    queryClient = new QueryClient({
      defaultOptions: {
        queries: { retry: false, gcTime: Infinity },
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
    errorHandler.log.mockClear();

    vi.mocked(TauriApi.getUserProfile).mockResolvedValue(targetUserProfile);
    vi.mocked(TauriApi.getUserProfileByPubkey).mockResolvedValue(null);
    vi.mocked(TauriApi.getPosts).mockResolvedValue([]);
    followersMock = [];
    followingMock = [];
    vi.mocked(TauriApi.getFollowers).mockImplementation(async () => ({
      items: followersMock,
      nextCursor: null,
      hasMore: false,
      totalCount: followersMock.length,
    }));
    vi.mocked(TauriApi.getFollowing).mockImplementation(async () => ({
      items: followingMock,
      nextCursor: null,
      hasMore: false,
      totalCount: followingMock.length,
    }));
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
    followersMock.push({
      npub: currentUserProfile.npub,
      pubkey: currentUserProfile.pubkey,
      name: currentUserProfile.name,
      display_name: currentUserProfile.displayName,
      about: currentUserProfile.about,
      picture: currentUserProfile.picture,
      banner: null,
      website: null,
      nip05: currentUserProfile.nip05 || null,
    });
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
    followersMock = [];
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

  it('フォロワー一覧のソート変更で API を再取得する', async () => {
    const user = userEvent.setup();
    renderWithClient(queryClient);

    await waitFor(() => expect(TauriApi.getFollowers).toHaveBeenCalled());
    expect(TauriApi.getFollowers).toHaveBeenLastCalledWith(
      expect.objectContaining({
        sort: 'recent',
        search: undefined,
      }),
    );

    const sortTriggers = await screen.findAllByRole('combobox');
    await user.click(sortTriggers[0]);
    const nameAscOption = await screen.findByRole('option', { name: '名前順 (A→Z)' });
    await user.click(nameAscOption);

    await waitFor(() =>
      expect(TauriApi.getFollowers).toHaveBeenLastCalledWith(
        expect.objectContaining({
          sort: 'name_asc',
          search: undefined,
        }),
      ),
    );
  });

  it('フォロー成功時にキャッシュとトーストが更新される', async () => {
    const user = userEvent.setup();
    renderWithClient(queryClient);

    const followButton = await screen.findByRole('button', { name: 'フォロー' });
    followersMock.push({
      npub: currentUserProfile.npub,
      pubkey: currentUserProfile.pubkey,
      name: currentUserProfile.name,
      display_name: currentUserProfile.displayName,
      about: currentUserProfile.about,
      picture: currentUserProfile.picture,
      banner: null,
      website: null,
      nip05: currentUserProfile.nip05 || null,
    });
    await user.click(followButton);

    await waitFor(() =>
      expect(toast.success).toHaveBeenCalledWith(
        `${targetUserProfile.display_name ?? targetUserProfile.npub} をフォローしました`,
      ),
    );

    const followersKey = [
      'profile',
      targetUserProfile.npub,
      'followers',
      'recent',
      '',
      currentUserProfile.npub,
    ] as const;

    await waitFor(() =>
      expect(
        queryClient.getQueryData<InfiniteData<{ items: UserProfileDto[] }>>(followersKey)?.pages[0]
          .items,
      ).toEqual(expect.arrayContaining([expect.objectContaining({ npub: currentUserProfile.npub })])),
    );

    await waitFor(() =>
      expect(
        queryClient.getQueryData<UserProfileDto[] | undefined>([
          'social',
          'following',
          currentUserProfile.npub,
        ]),
      ).toEqual(
        expect.arrayContaining([expect.objectContaining({ npub: targetUserProfile.npub })]),
      ),
    );
  });

  it('フォロー失敗時はログとトーストを表示し状態をロールバックする', async () => {
    const error = new Error('network');
    vi.mocked(TauriApi.followUser).mockRejectedValueOnce(error);
    const user = userEvent.setup();
    renderWithClient(queryClient);

    const followButton = await screen.findByRole('button', { name: 'フォロー' });
    await user.click(followButton);

    await waitFor(() => {
      expect(errorHandler.log).toHaveBeenCalledWith(
        'ProfilePage.followFailed',
        error,
        expect.objectContaining({
          context: 'ProfilePage.followMutation',
          metadata: { targetNpub: targetUserProfile.npub },
        }),
      );
    });
    expect(toast.error).toHaveBeenCalledWith('フォローに失敗しました');
    expect(screen.getByRole('button', { name: 'フォロー' })).toBeInTheDocument();
  });

  it('未ログイン時はフォローボタンが無効化される', async () => {
    mockAuthState.currentUser = null;
    renderWithClient(queryClient);

    const followButton = await screen.findByRole('button', { name: 'ログインが必要' });
    expect(followButton).toBeDisabled();
  });

  it('フォロワー一覧で無限スクロール時にカーソルと進捗が更新される', async () => {
    const responses = [
      {
        items: [
          { ...targetUserProfile, npub: 'npub1follower1', pubkey: 'pk1' },
          { ...targetUserProfile, npub: 'npub1follower2', pubkey: 'pk2' },
        ],
        nextCursor: 'cursor-1',
        hasMore: true,
        totalCount: 3,
      },
      {
        items: [{ ...targetUserProfile, npub: 'npub1follower3', pubkey: 'pk3' }],
        nextCursor: null,
        hasMore: false,
        totalCount: 3,
      },
    ];
    vi.mocked(TauriApi.getFollowers).mockImplementation(async (params) => {
      if (params.cursor) {
        return responses[1];
      }
      return responses[0];
    });

    renderWithClient(queryClient);

    await waitFor(() =>
      expect(TauriApi.getFollowers).toHaveBeenCalledWith(
        expect.objectContaining({
          viewerNpub: currentUserProfile.npub,
        }),
      ),
    );
    expect(await screen.findByText('表示中 2 / 3 件')).toBeInTheDocument();

    triggerIntersection();

    await waitFor(() =>
      expect(TauriApi.getFollowers).toHaveBeenCalledWith(
        expect.objectContaining({
          cursor: 'cursor-1',
        }),
      ),
    );
    await waitFor(() => expect(screen.getByText('表示中 3 / 3 件')).toBeInTheDocument());
  });
});
