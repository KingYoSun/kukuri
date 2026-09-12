import { act, fireEvent, render, screen } from '@testing-library/react';
import { expect, test, vi } from 'vitest';
import { ProfileRefreshButton } from './ProfileRefreshButton';

test('acknowledges an immediate completion for one second and prevents another click', async () => {
  vi.useFakeTimers();
  const onRefresh = vi.fn().mockResolvedValue(undefined);
  render(<ProfileRefreshButton refreshing={false} saving={false} onRefresh={onRefresh} />);
  const button = screen.getByRole('button');
  fireEvent.click(button);
  fireEvent.click(button);
  expect(onRefresh).toHaveBeenCalledTimes(1);
  await act(async () => vi.advanceTimersByTimeAsync(999));
  expect(button).toHaveAttribute('aria-busy', 'true');
  await act(async () => vi.advanceTimersByTimeAsync(1));
  expect(button).toHaveAttribute('aria-busy', 'false');
});

test('a slow background refresh stays busy until the actual request ends', async () => {
  vi.useFakeTimers();
  const onRefresh = vi.fn();
  const view = render(<ProfileRefreshButton refreshing={true} saving={false} onRefresh={onRefresh} />);
  await act(async () => vi.advanceTimersByTimeAsync(1500));
  expect(screen.getByRole('button')).toHaveAttribute('aria-busy', 'true');
  view.rerender(<ProfileRefreshButton refreshing={false} saving={false} onRefresh={onRefresh} />);
  expect(screen.getByRole('button')).toHaveAttribute('aria-busy', 'false');
});

test('a fast background request keeps feedback until the minimum duration and cleans up on unmount', async () => {
  vi.useFakeTimers();
  const onRefresh = vi.fn();
  const view = render(<ProfileRefreshButton refreshing={true} saving={false} onRefresh={onRefresh} />);
  await act(async () => vi.advanceTimersByTimeAsync(100));
  view.rerender(<ProfileRefreshButton refreshing={false} saving={false} onRefresh={onRefresh} />);
  expect(screen.getByRole('button')).toHaveAttribute('aria-busy', 'true');
  view.unmount();
  expect(vi.getTimerCount()).toBe(0);
});

test('saving prevents refresh without starting feedback', () => {
  const onRefresh = vi.fn();
  render(<ProfileRefreshButton refreshing={false} saving={true} onRefresh={onRefresh} />);
  const button = screen.getByRole('button');
  fireEvent.click(button);
  expect(onRefresh).not.toHaveBeenCalled();
  expect(button).toHaveAttribute('aria-busy', 'false');
  expect(button).toHaveAttribute('aria-disabled', 'true');
});
