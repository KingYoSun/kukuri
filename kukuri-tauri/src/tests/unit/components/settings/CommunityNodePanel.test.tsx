import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor, fireEvent } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';

import { CommunityNodePanel } from '@/components/settings/CommunityNodePanel';
import { communityNodeApi } from '@/lib/api/communityNode';
import { accessControlApi } from '@/lib/api/accessControl';
import { errorHandler } from '@/lib/errorHandler';
import { toast } from 'sonner';
import { useCommunityNodeStore } from '@/stores/communityNodeStore';
import { useAuthStore } from '@/stores/authStore';
import { useP2PStore } from '@/stores/p2pStore';

vi.mock('@/lib/api/communityNode', () => ({
  defaultCommunityNodeRoles: {
    labels: true,
    trust: true,
    search: false,
    bootstrap: true,
  },
  communityNodeApi: {
    getConfig: vi.fn(),
    getTrustProvider: vi.fn(),
    listGroupKeys: vi.fn(),
    getConsentStatus: vi.fn(),
    setConfig: vi.fn(),
    clearConfig: vi.fn(),
    authenticate: vi.fn(),
    clearToken: vi.fn(),
    acceptConsents: vi.fn(),
    setTrustProvider: vi.fn(),
    clearTrustProvider: vi.fn(),
  },
}));

vi.mock('@/lib/api/accessControl', () => ({
  accessControlApi: {
    listJoinRequests: vi.fn(),
    requestJoin: vi.fn(),
    approveJoinRequest: vi.fn(),
    rejectJoinRequest: vi.fn(),
  },
}));

vi.mock('@/lib/errorHandler', () => ({
  errorHandler: {
    log: vi.fn(),
    info: vi.fn(),
  },
}));

vi.mock('sonner', () => ({
  toast: {
    success: vi.fn(),
    error: vi.fn(),
  },
}));

vi.mock('@/stores/authStore', () => ({
  useAuthStore: {
    getState: vi.fn(),
  },
}));

vi.mock('@/stores/p2pStore', () => ({
  useP2PStore: {
    getState: vi.fn(),
  },
}));

const mockCommunityNodeApi = communityNodeApi as unknown as Record<string, vi.Mock>;
const mockAccessControlApi = accessControlApi as unknown as Record<string, vi.Mock>;
const mockErrorHandler = errorHandler as unknown as { log: vi.Mock };
const mockToast = toast as unknown as { success: vi.Mock; error: vi.Mock };
const mockUseAuthStore = useAuthStore as unknown as { getState: vi.Mock };
const mockUseP2PStore = useP2PStore as unknown as { getState: vi.Mock };
const mockUpdateRelayStatus = vi.fn().mockResolvedValue(undefined);
const mockRefreshP2PStatus = vi.fn().mockResolvedValue(undefined);

const createNode = (overrides: Record<string, unknown> = {}) => ({
  base_url: 'https://community.example',
  roles: {
    labels: true,
    trust: true,
    search: false,
    bootstrap: true,
  },
  has_token: true,
  token_expires_at: null,
  pubkey: 'abcdef0123456789abcdef0123456789',
  ...overrides,
});

const createQueryClient = () =>
  new QueryClient({
    defaultOptions: {
      queries: {
        retry: false,
      },
    },
  });

const renderPanel = () => {
  const client = createQueryClient();
  return render(
    <QueryClientProvider client={client}>
      <CommunityNodePanel />
    </QueryClientProvider>,
  );
};

beforeEach(() => {
  vi.clearAllMocks();
  useCommunityNodeStore.getState().reset();
  mockUseAuthStore.getState.mockReturnValue({
    updateRelayStatus: mockUpdateRelayStatus,
  });
  mockUseP2PStore.getState.mockReturnValue({
    refreshStatus: mockRefreshP2PStatus,
  });

  mockCommunityNodeApi.getConfig.mockResolvedValue({ nodes: [createNode()] });
  mockCommunityNodeApi.getTrustProvider.mockResolvedValue(null);
  mockCommunityNodeApi.listGroupKeys.mockResolvedValue([]);
  mockCommunityNodeApi.getConsentStatus.mockResolvedValue({ pending: [] });
  mockCommunityNodeApi.setConfig.mockResolvedValue({ nodes: [createNode()] });
  mockCommunityNodeApi.clearConfig.mockResolvedValue(undefined);
  mockCommunityNodeApi.authenticate.mockResolvedValue({});
  mockCommunityNodeApi.clearToken.mockResolvedValue(undefined);
  mockCommunityNodeApi.acceptConsents.mockResolvedValue({});
  mockCommunityNodeApi.setTrustProvider.mockResolvedValue({});
  mockCommunityNodeApi.clearTrustProvider.mockResolvedValue(undefined);

  mockAccessControlApi.listJoinRequests.mockResolvedValue({ items: [] });
  mockAccessControlApi.requestJoin.mockResolvedValue({});
  mockAccessControlApi.approveJoinRequest.mockResolvedValue({});
  mockAccessControlApi.rejectJoinRequest.mockResolvedValue(undefined);
});

describe('CommunityNodePanel', () => {
  it('adds a node after normalizing trailing slash', async () => {
    const user = userEvent.setup();
    renderPanel();

    await screen.findByTestId('community-node-node-0');

    await user.type(screen.getByTestId('community-node-base-url'), 'https://new.example///');
    await user.click(screen.getByTestId('community-node-save-config'));

    await waitFor(() => {
      expect(mockCommunityNodeApi.setConfig).toHaveBeenCalledWith([
        {
          base_url: 'https://community.example',
          roles: { labels: true, trust: true, search: false, bootstrap: true },
        },
        {
          base_url: 'https://new.example',
          roles: { labels: true, trust: true, search: false, bootstrap: true },
        },
      ]);
    });
    expect(mockToast.success).toHaveBeenCalledWith('Community Node を追加しました');
  });

  it('shows validation error when adding duplicate node', async () => {
    const user = userEvent.setup();
    renderPanel();

    await screen.findByTestId('community-node-node-0');

    await user.type(screen.getByTestId('community-node-base-url'), 'https://community.example');
    await user.click(screen.getByTestId('community-node-save-config'));

    expect(mockCommunityNodeApi.setConfig).not.toHaveBeenCalled();
    expect(mockToast.error).toHaveBeenCalledWith('同じBase URLのノードが既に登録されています');
  });

  it('updates role toggle state via setConfig', async () => {
    const user = userEvent.setup();
    renderPanel();

    const roleSwitch = await screen.findByTestId('community-node-role-search-0');
    await user.click(roleSwitch);

    await waitFor(() => {
      expect(mockCommunityNodeApi.setConfig).toHaveBeenCalledWith([
        {
          base_url: 'https://community.example',
          roles: { labels: true, trust: true, search: true, bootstrap: true },
        },
      ]);
    });
  });

  it('refreshes relay and p2p status after authenticate', async () => {
    const user = userEvent.setup();
    renderPanel();

    await screen.findByTestId('community-node-node-0');
    await user.click(screen.getByTestId('community-node-authenticate-0'));

    await waitFor(() => {
      expect(mockCommunityNodeApi.authenticate).toHaveBeenCalledWith('https://community.example');
      expect(mockUpdateRelayStatus).toHaveBeenCalledTimes(1);
      expect(mockRefreshP2PStatus).toHaveBeenCalledTimes(1);
    });
  });

  it('requests join from invite JSON', async () => {
    const user = userEvent.setup();
    renderPanel();

    await screen.findByTestId('community-node-node-0');
    fireEvent.change(screen.getByTestId('community-node-invite-json'), {
      target: { value: '{"kind":39000,"id":"evt"}' },
    });
    await user.click(screen.getByTestId('community-node-request-join'));

    await waitFor(() => {
      expect(mockAccessControlApi.requestJoin).toHaveBeenCalledWith({
        invite_event_json: { kind: 39000, id: 'evt' },
      });
    });
    expect(mockToast.success).toHaveBeenCalledWith('P2P 参加リクエストを送信しました');
  });

  it('logs error when invite JSON is invalid', async () => {
    const user = userEvent.setup();
    renderPanel();

    await screen.findByTestId('community-node-node-0');
    fireEvent.change(screen.getByTestId('community-node-invite-json'), {
      target: { value: '{broken json' },
    });
    await user.click(screen.getByTestId('community-node-request-join'));

    await waitFor(() => {
      expect(mockErrorHandler.log).toHaveBeenCalledWith(
        'Community node join request failed',
        expect.any(SyntaxError),
        expect.objectContaining({ context: 'CommunityNodePanel.requestJoin' }),
      );
    });
  });

  it('approves and rejects pending join requests', async () => {
    const user = userEvent.setup();
    mockAccessControlApi.listJoinRequests.mockResolvedValue({
      items: [
        {
          event_id: 'evt-1',
          topic_id: 'topic-1',
          scope: 'followers',
          requester_pubkey: 'abcdef0123456789abcdef0123456789',
          requested_at: 1,
          received_at: 2,
        },
      ],
    });

    renderPanel();

    await screen.findByTestId('community-node-join-requests');

    await user.click(screen.getByTestId('community-node-join-approve-evt-1'));
    await waitFor(() => {
      expect(mockAccessControlApi.approveJoinRequest).toHaveBeenCalledWith({ event_id: 'evt-1' });
    });

    await user.click(screen.getByTestId('community-node-join-reject-evt-1'));
    await waitFor(() => {
      expect(mockAccessControlApi.rejectJoinRequest).toHaveBeenCalledWith({ event_id: 'evt-1' });
    });
  });

  it('accepts consents for selected node', async () => {
    const user = userEvent.setup();
    renderPanel();

    const acceptButton = await screen.findByTestId('community-node-accept-consents');
    await waitFor(() => {
      expect(acceptButton).toBeEnabled();
    });

    await user.click(acceptButton);

    await waitFor(() => {
      expect(mockCommunityNodeApi.acceptConsents).toHaveBeenCalledWith({
        base_url: 'https://community.example',
        accept_all_current: true,
      });
    });
    expect(mockToast.success).toHaveBeenCalledWith('同意状況を更新しました');
  });

  it('logs query error when trust provider fetch fails', async () => {
    mockCommunityNodeApi.getTrustProvider.mockRejectedValue(new Error('fetch failed'));

    renderPanel();

    await waitFor(() => {
      expect(mockErrorHandler.log).toHaveBeenCalledWith(
        'Failed to load community node trust provider',
        expect.any(Error),
        expect.objectContaining({ context: 'CommunityNodePanel.trustProvider' }),
      );
    });
  });
});
