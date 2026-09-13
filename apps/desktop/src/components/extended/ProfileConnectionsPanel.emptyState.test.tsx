// #994: フォロー中 / フォロワー / ミュート中 / ブロック中の 0 件で、実ボタンを示す chip と次の行動を同じ場所に置く。
// chip は操作不能(button role を持たない)で、CTA は既存導線の呼出しだけを行う(INVAR-1)。
import { act, render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, expect, test, vi } from 'vitest';

import type { ProfileConnectionsView } from '@/components/shell/types';
import { MIN_LIST_LOADING_MS } from '@/lib/useMinimumLoading';

import { ProfileConnectionsPanel } from './ProfileConnectionsPanel';

const LOCAL_ID = 'f'.repeat(64);

type PanelProps = Partial<React.ComponentProps<typeof ProfileConnectionsPanel>>;

function renderEmpty(activeView: ProfileConnectionsView, overrides: PanelProps = {}) {
  const props = {
    activeView,
    items: [],
    localAuthorPubkey: LOCAL_ID,
    status: 'ready' as const,
    error: null,
    onSelectView: vi.fn(),
    onToggleRelationship: vi.fn(),
    onToggleMute: vi.fn(),
    onToggleBlock: vi.fn(),
    onBack: vi.fn(),
    onOpenTimeline: vi.fn(),
    onOpenExplore: vi.fn(),
    ...overrides,
  };
  const view = render(<ProfileConnectionsPanel {...props} />);
  return { ...view, props };
}

function chips(root: HTMLElement): string[] {
  return Array.from(root.querySelectorAll('[data-testid="action-ref"]')).map((chip) =>
    chip.textContent?.trim() ?? ''
  );
}

afterEach(() => {
  vi.useRealTimers();
});

test('following: shows how to open a profile, the Follow chip, and timeline / explore actions', async () => {
  const user = userEvent.setup();
  const { props } = renderEmpty('following');
  const guidance = screen.getByRole('status', { name: 'You are not following anyone yet.' });
  expect(guidance).toHaveAttribute('data-testid', 'profile-connections-empty-state');
  expect(within(guidance).getByText(/name or avatar/)).toBeInTheDocument();
  expect(chips(guidance)).toEqual(['Follow']);
  expect(within(guidance).queryByRole('button', { name: 'Follow' })).not.toBeInTheDocument();
  expect(guidance.querySelector('[data-testid="action-ref"]')).not.toHaveAttribute('tabindex');

  await user.click(within(guidance).getByRole('button', { name: 'Show timeline' }));
  expect(props.onOpenTimeline).toHaveBeenCalledTimes(1);
  await user.click(within(guidance).getByRole('button', { name: 'Search in Explore' }));
  expect(props.onOpenExplore).toHaveBeenCalledTimes(1);
  expect(props.onToggleRelationship).not.toHaveBeenCalled();
});

test('followers: explains that follows arrive from other devices and offers copying the own user ID', async () => {
  const user = userEvent.setup();
  const clipboardWriteText = vi.fn().mockResolvedValue(undefined);
  Object.defineProperty(navigator, 'clipboard', {
    configurable: true,
    value: { writeText: clipboardWriteText },
  });
  const { props } = renderEmpty('followed');
  const guidance = screen.getByRole('status', { name: 'No followers known to this device yet.' });
  expect(within(guidance).getByText(/reaches this device/)).toBeInTheDocument();
  expect(chips(guidance)).toEqual([]);

  await user.click(within(guidance).getByRole('button', { name: 'Copy user ID' }));
  expect(clipboardWriteText).toHaveBeenLastCalledWith(LOCAL_ID);
  await user.click(within(guidance).getByRole('button', { name: 'Show timeline' }));
  expect(props.onOpenTimeline).toHaveBeenCalledTimes(1);
});

test('muted: shows the Mute chip, the Report icon route, the device-only note, and the timeline action', async () => {
  const user = userEvent.setup();
  const { props } = renderEmpty('muted');
  const guidance = screen.getByRole('status', { name: 'No muted users yet.' });
  expect(chips(guidance)).toEqual(['Mute', 'Report']);
  expect(within(guidance).getByText(/on this device only/)).toBeInTheDocument();
  expect(within(guidance).queryByRole('button', { name: 'Mute' })).not.toBeInTheDocument();
  await user.click(within(guidance).getByRole('button', { name: 'Show timeline' }));
  expect(props.onOpenTimeline).toHaveBeenCalledTimes(1);
  expect(props.onToggleMute).not.toHaveBeenCalled();
});

test('blocking: shows the Block chip and the signed / synced note', () => {
  const { props } = renderEmpty('blocking');
  const guidance = screen.getByRole('status', { name: 'No blocked users yet.' });
  expect(chips(guidance)).toEqual(['Block']);
  expect(within(guidance).getByText(/signed with your account/)).toBeInTheDocument();
  expect(within(guidance).getByRole('button', { name: 'Show timeline' })).toBeInTheDocument();
  expect(props.onToggleBlock).not.toHaveBeenCalled();
});

test('the empty guidance is not shown while loading or on error', () => {
  const { unmount } = renderEmpty('muted', { status: 'loading' });
  expect(screen.getByText('Loading follows, mutes, and blocks…')).toBeInTheDocument();
  expect(screen.queryByTestId('profile-connections-empty-state')).not.toBeInTheDocument();
  unmount();
  // error で mount した panel は最低 loading を経ずに error を示す(値の無い error を loading に偽装しない)。
  renderEmpty('muted', { status: 'error', error: 'boom' });
  expect(screen.getByText('boom')).toBeInTheDocument();
  expect(screen.queryByTestId('profile-connections-empty-state')).not.toBeInTheDocument();
  expect(screen.queryByText('Loading follows, mutes, and blocks…')).not.toBeInTheDocument();
});

test('a fast initial fetch keeps the loading notice for the minimum duration before the guidance appears', () => {
  vi.useFakeTimers();
  const { rerender } = renderEmpty('following', { status: 'loading' });
  expect(screen.getByText('Loading follows, mutes, and blocks…')).toBeInTheDocument();
  act(() => {
    vi.advanceTimersByTime(50);
  });
  rerender(
    <ProfileConnectionsPanel
      activeView='following'
      items={[]}
      localAuthorPubkey={LOCAL_ID}
      status='ready'
      error={null}
      onSelectView={vi.fn()}
      onToggleRelationship={vi.fn()}
      onToggleMute={vi.fn()}
      onToggleBlock={vi.fn()}
      onBack={vi.fn()}
      onOpenTimeline={vi.fn()}
    />
  );
  expect(screen.getByText('Loading follows, mutes, and blocks…')).toBeInTheDocument();
  expect(screen.queryByTestId('profile-connections-empty-state')).not.toBeInTheDocument();
  act(() => {
    vi.advanceTimersByTime(MIN_LIST_LOADING_MS - 50);
  });
  expect(screen.queryByText('Loading follows, mutes, and blocks…')).not.toBeInTheDocument();
  expect(screen.getByTestId('profile-connections-empty-state')).toBeInTheDocument();
});
