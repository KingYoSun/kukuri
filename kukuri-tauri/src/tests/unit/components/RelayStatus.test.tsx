import { render, screen, cleanup, act, fireEvent } from '@testing-library/react';
import type { Mock } from 'vitest';
import { beforeEach, afterEach, describe, expect, it, vi } from 'vitest';
import { RelayStatus, MAINLINE_RUNBOOK_URL } from '@/components/RelayStatus';
import { useAuthStore } from '@/stores/authStore';

vi.mock('@/stores/authStore', () => ({
  useAuthStore: vi.fn(),
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

const defaultState = (): MockStoreState => ({
  relayStatus: [],
  updateRelayStatus: vi.fn().mockResolvedValue(undefined),
  relayStatusError: null,
  relayStatusBackoffMs: 30_000,
  lastRelayStatusFetchedAt: Date.now(),
  isFetchingRelayStatus: false,
});

const renderRelayStatus = (overrides: Partial<MockStoreState> = {}) => {
  const state = { ...defaultState(), ...overrides };
  mockedUseAuthStore.mockReturnValue(state);
  const utils = render(<RelayStatus />);
  return { ...utils, state };
};

describe('RelayStatus', () => {
  beforeEach(() => {
    vi.useFakeTimers();
    vi.clearAllMocks();
  });

  afterEach(() => {
    cleanup();
    vi.runOnlyPendingTimers();
    vi.useRealTimers();
    vi.clearAllMocks();
  });

  it('renders placeholder when no relay status is available', () => {
    renderRelayStatus({ relayStatus: [] });

    expect(screen.getByText('リレー接続状態')).toBeInTheDocument();
    expect(screen.getByText('接続中のリレーはありません。')).toBeInTheDocument();
  });

  it('renders relay entries with status badges', () => {
    renderRelayStatus({
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

  it('triggers immediate fetch when no previous timestamp exists', () => {
    const { state } = renderRelayStatus({ lastRelayStatusFetchedAt: null });
    expect(state.updateRelayStatus).toHaveBeenCalledTimes(1);
  });

  it('schedules automatic refresh using backoff interval', () => {
    const { state } = renderRelayStatus({ relayStatusBackoffMs: 60_000 });

    expect(state.updateRelayStatus).not.toHaveBeenCalled();

    act(() => {
      vi.advanceTimersByTime(60_000);
    });

    expect(state.updateRelayStatus).toHaveBeenCalledTimes(1);
  });

  it('shows error message when relayStatusError is present', () => {
    renderRelayStatus({ relayStatusError: 'timeout' });

    expect(screen.getByText('リレー状態の取得に失敗しました。')).toBeInTheDocument();
    expect(screen.getByText(/詳細: timeout/)).toBeInTheDocument();
  });

  it('manual retry button triggers updateRelayStatus', async () => {
    const { state } = renderRelayStatus();

    const retryButton = screen.getByRole('button', { name: '再試行' });
    fireEvent.click(retryButton);

    expect(state.updateRelayStatus).toHaveBeenCalledTimes(1);
  });

  it('renders runbook link with external target', () => {
    renderRelayStatus();

    const runbookLink = screen.getByRole('link', { name: 'Runbook' });
    expect(runbookLink).toHaveAttribute('href', MAINLINE_RUNBOOK_URL);
    expect(runbookLink).toHaveAttribute('target', '_blank');
    expect(runbookLink).toHaveAttribute('rel', 'noreferrer');
  });
});
