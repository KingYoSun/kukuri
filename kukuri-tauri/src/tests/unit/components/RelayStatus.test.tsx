import { render, screen, cleanup, act, fireEvent } from '@testing-library/react';
import type { Mock } from 'vitest';
import { beforeEach, afterEach, describe, expect, it, vi } from 'vitest';
import { RelayStatus, MAINLINE_RUNBOOK_URL } from '@/components/RelayStatus';
import { useAuthStore } from '@/stores/authStore';
import { p2pApi } from '@/lib/api/p2p';
import { errorHandler } from '@/lib/errorHandler';

vi.mock('@/stores/authStore', () => ({
  useAuthStore: vi.fn(),
}));

vi.mock('@/lib/api/p2p', () => ({
  p2pApi: {
    getBootstrapConfig: vi.fn(),
    applyCliBootstrapNodes: vi.fn(),
  },
}));

vi.mock('@/lib/errorHandler', () => ({
  errorHandler: {
    log: vi.fn(),
  },
}));

type RelayInfo = {
  url: string;
  status: string;
};

type MockStoreState = {
  relayStatus: RelayInfo[];
  updateRelayStatus: Mock;
  relayStatusError: string | null;
  relayStatusBackoffMs: number;
  lastRelayStatusFetchedAt: number | null;
  isFetchingRelayStatus: boolean;
};

const mockedUseAuthStore = useAuthStore as unknown as Mock;
const mockGetBootstrapConfig = p2pApi.getBootstrapConfig as unknown as Mock;
const mockApplyCliBootstrapNodes = p2pApi.applyCliBootstrapNodes as unknown as Mock;
const mockErrorHandlerLog = errorHandler.log as unknown as Mock;

const bootstrapConfigResponse = {
  mode: 'default',
  nodes: [],
  effective_nodes: [],
  source: 'none',
  env_locked: false,
  cli_nodes: [],
  cli_updated_at_ms: null,
};

const defaultState = (): MockStoreState => ({
  relayStatus: [],
  updateRelayStatus: vi.fn().mockResolvedValue(undefined),
  relayStatusError: null,
  relayStatusBackoffMs: 30_000,
  lastRelayStatusFetchedAt: Date.now(),
  isFetchingRelayStatus: false,
});

const flushAsync = async () => {
  await act(async () => {
    await Promise.resolve();
  });
};

const renderRelayStatus = async (overrides: Partial<MockStoreState> = {}) => {
  const state = { ...defaultState(), ...overrides };
  mockedUseAuthStore.mockReturnValue(state);
  let utils: ReturnType<typeof render>;
  await act(async () => {
    utils = render(<RelayStatus />);
  });
  await flushAsync();
  return { ...utils!, state };
};

describe('RelayStatus', () => {
  beforeEach(() => {
    vi.useFakeTimers();
    vi.clearAllMocks();
    mockGetBootstrapConfig.mockResolvedValue(bootstrapConfigResponse);
    mockApplyCliBootstrapNodes.mockResolvedValue(bootstrapConfigResponse);
    mockErrorHandlerLog.mockReset();
  });

  afterEach(() => {
    cleanup();
    vi.runOnlyPendingTimers();
    vi.useRealTimers();
    vi.clearAllMocks();
  });

  it('renders placeholder when no relay status is available', async () => {
    await renderRelayStatus({ relayStatus: [] });

    expect(screen.getByText('リレー接続状態')).toBeInTheDocument();
    expect(screen.getByText('接続中のリレーはありません。')).toBeInTheDocument();
  });

  it('renders relay entries with status badges', async () => {
    await renderRelayStatus({
      relayStatus: [
        { url: 'wss://relay1.example', status: 'connected' },
        { url: 'wss://relay2.example', status: 'connecting' },
        { url: 'wss://relay3.example', status: 'disconnected' },
        { url: 'wss://relay4.example', status: 'error: unreachable' },
      ],
    });

    expect(screen.getByText('wss://relay1.example')).toBeInTheDocument();
    expect(screen.getByText('wss://relay2.example')).toBeInTheDocument();
    expect(screen.getByText('wss://relay3.example')).toBeInTheDocument();
    expect(screen.getByText('wss://relay4.example')).toBeInTheDocument();

    expect(screen.getByText('接続済み')).toHaveClass('bg-green-100');
    expect(screen.getByText('接続中')).toHaveClass('bg-yellow-100');
    expect(screen.getByText('切断')).toHaveClass('bg-gray-100');
    expect(screen.getByText('エラー')).toHaveClass('bg-red-100');
  });

  it('triggers immediate fetch when no previous timestamp exists', async () => {
    const { state } = await renderRelayStatus({ lastRelayStatusFetchedAt: null });
    expect(state.updateRelayStatus).toHaveBeenCalledTimes(1);
  });

  it('schedules automatic refresh using backoff interval', async () => {
    const { state } = await renderRelayStatus({ relayStatusBackoffMs: 60_000 });

    expect(state.updateRelayStatus).not.toHaveBeenCalled();

    await act(async () => {
      vi.advanceTimersByTime(60_000);
    });
    await flushAsync();

    expect(state.updateRelayStatus).toHaveBeenCalledTimes(1);
  });

  it('shows error message when relayStatusError is present', async () => {
    await renderRelayStatus({ relayStatusError: 'timeout' });

    expect(screen.getByText('リレー状態の取得に失敗しました。')).toBeInTheDocument();
    expect(screen.getByText(/詳細: timeout/)).toBeInTheDocument();
  });

  it('manual retry button triggers updateRelayStatus', async () => {
    const { state } = await renderRelayStatus();

    const retryButton = screen.getByRole('button', { name: '再試行' });
    await act(async () => {
      fireEvent.click(retryButton);
    });

    expect(state.updateRelayStatus).toHaveBeenCalledTimes(1);
  });

  it('refreshes bootstrap config when manual retry is triggered', async () => {
    await renderRelayStatus();
    mockGetBootstrapConfig.mockClear();

    const retryButton = screen.getByRole('button', { name: '再試行' });
    await act(async () => {
      fireEvent.click(retryButton);
    });
    await flushAsync();

    expect(mockGetBootstrapConfig).toHaveBeenCalledTimes(1);
  });

  it('automatic refresh also refetches bootstrap config', async () => {
    await renderRelayStatus({ relayStatusBackoffMs: 60_000 });
    mockGetBootstrapConfig.mockClear();

    await act(async () => {
      vi.advanceTimersByTime(60_000);
    });
    await flushAsync();

    expect(mockGetBootstrapConfig).toHaveBeenCalledTimes(1);
  });

  it('renders runbook link with external target', async () => {
    await renderRelayStatus();

    const runbookLink = screen.getByRole('link', { name: 'Runbook' });
    expect(runbookLink).toHaveAttribute('href', MAINLINE_RUNBOOK_URL);
    expect(runbookLink).toHaveAttribute('target', '_blank');
    expect(runbookLink).toHaveAttribute('rel', 'noreferrer');
  });

  it('enables CLI apply button when CLI nodes exist', async () => {
    mockGetBootstrapConfig.mockResolvedValueOnce({
      ...bootstrapConfigResponse,
      cli_nodes: ['node1@example:1234'],
      cli_updated_at_ms: Date.now(),
      effective_nodes: [],
    });
    const { state } = await renderRelayStatus();
    const applyButton = screen.getByRole('button', { name: '最新リストを適用' });
    expect(applyButton).not.toBeDisabled();
    await act(async () => {
      fireEvent.click(applyButton);
    });
    await flushAsync();
    expect(mockApplyCliBootstrapNodes).toHaveBeenCalledTimes(1);
    expect(state.updateRelayStatus).toHaveBeenCalledTimes(1);
  });
});
