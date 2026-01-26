import { render } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import React from 'react';
import { vi } from 'vitest';
import type { Post } from '@/stores';

const hoisted = vi.hoisted(() => {
  const bookmarkStore = {
    fetchBookmarks: vi.fn(),
    toggleBookmark: vi.fn(),
    isBookmarked: vi.fn(() => false),
  };

  const offlineState = {
    isOnline: true,
    pendingActions: [] as Array<{ actionType: string; localId?: string }>,
    saveOfflineAction: vi.fn(),
  };
  const useOfflineStore = vi.fn(() => offlineState);
  (useOfflineStore as unknown as { getState: () => typeof offlineState }).getState = () =>
    offlineState;

  const deletePostMutation = {
    mutate: vi.fn(),
    mutateAsync: vi.fn(),
    isPending: false,
    manualRetryDelete: vi.fn(),
  };

  const createPost = vi.fn();
  const usePostStore = vi.fn(() => ({
    createPost,
  }));

  const communityNodeState = {
    enableAccessControl: true,
    enableLabels: true,
    enableTrust: true,
    enableSearch: false,
    setEnableAccessControl: vi.fn(),
    setEnableLabels: vi.fn(),
    setEnableTrust: vi.fn(),
    setEnableSearch: vi.fn(),
    reset: vi.fn(),
  };
  const useCommunityNodeStore = vi.fn(() => communityNodeState);

  const authState = {
    currentUser: {
      pubkey: 'user-pubkey',
      npub: 'npub1user',
      name: 'Current User',
      displayName: 'Current User Display',
      picture: 'https://example.com/current-user.jpg',
    },
  };

  const useAuthStore = vi.fn((selector?: (state: typeof authState) => unknown) => {
    return typeof selector === 'function' ? selector(authState) : authState;
  });

  return {
    bookmarkStoreMock: bookmarkStore,
    offlineStoreState: offlineState,
    likePostMock: vi.fn(),
    boostPostMock: vi.fn(),
    createPostMock: createPost,
    deletePostMutationMock: deletePostMutation,
    toastMock: {
      error: vi.fn(),
      success: vi.fn(),
    },
    useAuthStoreMock: useAuthStore,
    useDeletePostMock: vi.fn(() => deletePostMutation),
    useOfflineStoreMock: useOfflineStore,
    usePostStoreMock: usePostStore,
    useCommunityNodeStoreMock: useCommunityNodeStore,
  };
});

export const {
  bookmarkStoreMock,
  offlineStoreState,
  likePostMock,
  boostPostMock,
  createPostMock,
  deletePostMutationMock,
  toastMock,
  useAuthStoreMock,
  useDeletePostMock,
  useOfflineStoreMock,
  usePostStoreMock,
  useCommunityNodeStoreMock,
} = hoisted;

export const mockPost: Post = {
  id: '1',
  content: 'テスト投稿です',
  author: {
    id: 'user1',
    pubkey: 'pubkey1',
    npub: 'npub1test...',
    name: 'テストユーザー',
    displayName: 'Test User',
    picture: '',
    about: '',
    nip05: '',
  },
  topicId: 'topic1',
  created_at: Math.floor(Date.now() / 1000) - 3600,
  tags: [],
  likes: 10,
  boosts: 0,
  replies: [],
  isSynced: true,
};

export const renderWithQueryClient = (ui: React.ReactElement) => {
  const queryClient = new QueryClient({
    defaultOptions: {
      queries: { retry: false },
      mutations: { retry: false },
    },
  });

  return render(<QueryClientProvider client={queryClient}>{ui}</QueryClientProvider>);
};

vi.mock('@/lib/api/tauri', () => ({
  TauriApi: {
    likePost: likePostMock,
    boostPost: boostPostMock,
    createPost: createPostMock,
  },
}));

vi.mock('@/lib/api/communityNode', () => ({
  communityNodeApi: {
    getConfig: vi.fn().mockResolvedValue(null),
    listLabels: vi.fn(),
    trustReportBased: vi.fn(),
    trustCommunicationDensity: vi.fn(),
  },
}));

vi.mock('sonner', () => ({
  toast: toastMock,
}));

vi.mock('@/hooks/usePosts', () => ({
  useDeletePost: useDeletePostMock,
}));

vi.mock('@/components/ui/collapsible', () => ({
  Collapsible: ({ children, open }: { children: React.ReactNode; open: boolean }) => (
    <div data-state={open ? 'open' : 'closed'}>{open ? children : null}</div>
  ),
  CollapsibleContent: ({ children }: { children: React.ReactNode }) => <div>{children}</div>,
}));

vi.mock('@/components/ui/dropdown-menu', () => ({
  DropdownMenu: ({ children }: { children: React.ReactNode }) => <div>{children}</div>,
  DropdownMenuTrigger: ({ children }: { children: React.ReactNode }) => <>{children}</>,
  DropdownMenuContent: ({ children }: { children: React.ReactNode }) => <div>{children}</div>,
  DropdownMenuItem: ({
    children,
    onClick,
    ...rest
  }: {
    children: React.ReactNode;
    onClick?: () => void;
    [key: string]: unknown;
  }) => (
    <button type="button" onClick={onClick} {...rest}>
      {children}
    </button>
  ),
}));

vi.mock('@/components/ui/alert-dialog', () => ({
  AlertDialog: ({ children, open = true }: { children: React.ReactNode; open?: boolean }) => (
    <div>{open ? children : null}</div>
  ),
  AlertDialogContent: ({ children }: { children: React.ReactNode }) => <div>{children}</div>,
  AlertDialogHeader: ({ children }: { children: React.ReactNode }) => <div>{children}</div>,
  AlertDialogTitle: ({
    children,
    ...rest
  }: {
    children: React.ReactNode;
    [key: string]: unknown;
  }) => <h3 {...rest}>{children}</h3>,
  AlertDialogDescription: ({ children }: { children: React.ReactNode }) => <p>{children}</p>,
  AlertDialogFooter: ({ children }: { children: React.ReactNode }) => <div>{children}</div>,
  AlertDialogCancel: ({
    children,
    disabled,
    onClick,
  }: {
    children: React.ReactNode;
    disabled?: boolean;
    onClick?: () => void;
  }) => (
    <button type="button" disabled={disabled} onClick={onClick}>
      {children}
    </button>
  ),
  AlertDialogAction: ({
    children,
    disabled,
    onClick,
    ...rest
  }: {
    children: React.ReactNode;
    disabled?: boolean;
    onClick?: () => void;
  }) => (
    <button type="button" disabled={disabled} onClick={onClick} {...rest}>
      {children}
    </button>
  ),
}));

vi.mock('@/stores', () => ({
  useAuthStore: useAuthStoreMock,
  useBookmarkStore: vi.fn(() => bookmarkStoreMock),
  usePostStore: usePostStoreMock,
}));

vi.mock('@/stores/communityNodeStore', () => ({
  useCommunityNodeStore: useCommunityNodeStoreMock,
}));

vi.mock('@/stores/offlineStore', () => ({
  useOfflineStore: useOfflineStoreMock,
}));
