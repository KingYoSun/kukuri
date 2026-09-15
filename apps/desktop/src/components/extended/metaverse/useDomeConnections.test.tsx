import { act, renderHook, waitFor } from '@testing-library/react';
import { describe, expect, test, vi } from 'vitest';
import type { MetaverseRoomActions } from './MetaverseRoomActions';
import { connectionFixtureRooms as rooms, connectionFixtureTopology } from './DomeConnectionFixtures';
import { useDomeConnections } from './useDomeConnections';

describe('connection reads', () => {
  test('deduplicates concurrent refresh and retains a failed snapshot', async () => {
    const topology = connectionFixtureTopology();
    const listConnections = vi.fn().mockResolvedValueOnce(topology).mockRejectedValue(new Error('offline'));
    const actions = { listConnections } as unknown as MetaverseRoomActions;
    const { result } = renderHook(() => useDomeConnections(actions, rooms[0], 'author'));
    await waitFor(() => expect(result.current.status).toBe('ready'));
    await act(async () => { await Promise.all([result.current.refresh(), result.current.refresh()]); });
    expect(listConnections).toHaveBeenCalledTimes(2);
    expect(result.current.status).toBe('error');
    expect(result.current.topology).toBe(topology);
  });
  test('discards late reads on account or generation changes', async () => {
    const topology = connectionFixtureTopology();
    let resolve!: (value: typeof topology) => void;
    const listConnections = vi.fn().mockImplementationOnce(() => new Promise(r => { resolve = r; })).mockResolvedValue({ ...topology, proposals: [] });
    const actions = { listConnections } as unknown as MetaverseRoomActions;
    const { result, rerender } = renderHook(({ author }) => useDomeConnections(actions, rooms[0], author), { initialProps: { author: 'old' } });
    rerender({ author: 'new' });
    await waitFor(() => expect(result.current.status).toBe('ready'));
    const latest = result.current.topology;
    await act(async () => resolve(topology));
    expect(result.current.topology).toBe(latest);
    expect(result.current.scope).toContain('new');
  });
  test('does not read while no Dome is selected and rejects a mismatched response', async () => {
    const topology = connectionFixtureTopology(); topology.resolution.topology.spatial_context = { kind: 'topic', topic_id: 'foreign' };
    const listConnections = vi.fn().mockResolvedValue(topology);
    const actions = { listConnections } as unknown as MetaverseRoomActions;
    const { result, rerender } = renderHook(({ room }) => useDomeConnections(actions, room, 'a'), { initialProps: { room: null as typeof rooms[0] | null } });
    expect(listConnections).not.toHaveBeenCalled(); rerender({ room: rooms[0] });
    await waitFor(() => expect(result.current.status).toBe('error'));
    expect(result.current.topology).toBeNull();
  });
});
