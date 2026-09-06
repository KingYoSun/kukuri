import { expect, test, vi } from 'vitest';
import type { DesktopApi, PostView, TimelineView } from '@/lib/api';
import { loadThreadForFocus } from './threadFocus';

const post = (id: string) => ({ object_id: id }) as PostView;
const cursor = { created_at: 10, object_id: 'page-end' };
const first = { items: [post('root')], next_cursor: cursor };

test('finds a later-page target in the same topic/thread and deduplicates overlap', async () => {
  const listThread = vi.fn<DesktopApi['listThread']>()
    .mockResolvedValueOnce(first)
    .mockResolvedValueOnce({ items: [post('root'), post('target')], next_cursor: null });
  const page = await loadThreadForFocus({ listThread }, 'private-topic', 'root', 'target', () => true);
  expect(page?.items.map((item) => item.object_id)).toEqual(['root', 'target']);
  expect(listThread.mock.calls).toEqual([
    ['private-topic', 'root', null, 30], ['private-topic', 'root', cursor, 30],
  ]);
});

test('an exhausted thread and a non-advancing cursor terminate without a made-up target', async () => {
  const listThread = vi.fn<DesktopApi['listThread']>().mockResolvedValue(first);
  const page = await loadThreadForFocus({ listThread }, 'topic', 'root', 'missing', () => true);
  expect(listThread).toHaveBeenCalledTimes(2);
  expect(page?.items.map((item) => item.object_id)).toEqual(['root']);
  expect(page?.next_cursor).toBeNull();
});

test('cancellation before or during a read prevents further page reads or publication', async () => {
  const listThread = vi.fn<DesktopApi['listThread']>().mockResolvedValue(first);
  expect(await loadThreadForFocus({ listThread }, 'topic', 'root', 'target', () => false)).toBeNull();
  expect(listThread).not.toHaveBeenCalled();
  let current = true;
  let finish!: (page: TimelineView) => void;
  listThread.mockReturnValueOnce(new Promise((resolve) => { finish = resolve; }));
  const pending = loadThreadForFocus({ listThread }, 'topic', 'root', 'target', () => current);
  current = false;
  finish(first);
  expect(await pending).toBeNull();
  expect(listThread).toHaveBeenCalledTimes(1);
});

test('a failed additional read propagates to the existing recoverable thread error path', async () => {
  const listThread = vi.fn<DesktopApi['listThread']>().mockResolvedValueOnce(first)
    .mockRejectedValueOnce(new Error('unavailable'));
  await expect(loadThreadForFocus({ listThread }, 'topic', 'root', 'target', () => true))
    .rejects.toThrow('unavailable');
  expect(listThread).toHaveBeenCalledTimes(2);
});
