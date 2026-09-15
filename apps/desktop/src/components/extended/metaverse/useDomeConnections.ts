import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import type { DomeConnectionTopologyView, GameRoomView, SpatialContextV1 } from '@/lib/api';
import type { MetaverseRoomActions } from './MetaverseRoomActions';

export function connectionContextKey(context: SpatialContextV1 | null | undefined) {
  return context ? JSON.stringify([context.kind, context.topic_id, context.kind === 'channel' ? context.channel_id : null]) : '';
}

export function connectionScope(room: GameRoomView | null, author: string) {
  const dome = room?.metaverse;
  return dome ? JSON.stringify([author, connectionContextKey(dome.spatial_context), dome.instance_id, dome.instance_generation]) : '';
}

type Snapshot = {
  scope: string;
  topology: DomeConnectionTopologyView | null;
  status: 'loading' | 'ready' | 'refreshing' | 'error';
  error: string | null;
};
export type DomeConnectionsState = Snapshot & { refresh: () => Promise<void> };

/** One owner per active surface; the admitted session passes this same snapshot to its map. */
export function useDomeConnections(actions: MetaverseRoomActions, room: GameRoomView | null, author: string): DomeConnectionsState {
  const scope = connectionScope(room, author);
  const contextKey = connectionContextKey(room?.metaverse?.spatial_context);
  const context = useMemo<SpatialContextV1 | null>(() => {
    if (!contextKey) return null;
    const [kind, topic_id, channel_id] = JSON.parse(contextKey);
    return kind === 'channel' ? { kind, topic_id, channel_id } : { kind, topic_id };
  }, [contextKey]);
  const [snapshot, setSnapshot] = useState<Snapshot>({ scope: '', topology: null, status: 'loading', error: null });
  const epoch = useRef(0);
  const flight = useRef<{ scope: string; promise: Promise<void> } | null>(null);
  const refresh = useCallback(() => {
    if (!context || !scope) return Promise.resolve();
    if (flight.current?.scope === scope) return flight.current.promise;
    const generation = epoch.current;
    setSnapshot(current => ({ scope, topology: current.scope === scope ? current.topology : null,
      status: current.scope === scope && current.topology ? 'refreshing' : 'loading', error: null }));
    const promise = (async () => {
      try {
        const topology = await actions.listConnections(context);
        if (connectionContextKey(topology.resolution.topology.spatial_context) !== contextKey) throw new Error('Connection context changed');
        if (epoch.current === generation) setSnapshot({ scope, topology, status: 'ready', error: null });
      } catch (error) {
        if (epoch.current === generation) setSnapshot(current => ({ scope,
          topology: current.scope === scope ? current.topology : null, status: 'error',
          error: error instanceof Error ? error.message : String(error) }));
      } finally {
        if (epoch.current === generation) flight.current = null;
      }
    })();
    flight.current = { scope, promise };
    return promise;
  }, [actions, context, contextKey, scope]);
  useEffect(() => {
    epoch.current += 1;
    flight.current = null;
    void refresh();
    if (!scope) return;
    const timer = window.setInterval(() => void refresh(), 5_000);
    return () => { epoch.current += 1; flight.current = null; window.clearInterval(timer); };
  }, [refresh, scope]);
  return { ...(snapshot.scope === scope ? snapshot : { scope, topology: null, status: 'loading' as const, error: null }), refresh };
}
