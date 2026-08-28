import { useEffect, useState } from 'react';

import type { GameRoomView, MetaverseAssetRef } from '@/lib/api';
import type { MetaverseRoomActions } from './MetaverseRoomActions';
import {
  resolveActiveDomeNeighbors,
  type DomeNeighborTransitionView,
} from './DomeTransitionModel';

export function useDomeTransitionNeighbors(
  actions: MetaverseRoomActions,
  selectedRoom: GameRoomView | null,
  rooms: GameRoomView[],
  participantPubkey: string
) {
  const state = useState<DomeNeighborTransitionView[]>([]);
  const [, setNeighbors] = state;

  useEffect(() => {
    if (!selectedRoom?.metaverse) {
      setNeighbors([]);
      return;
    }
    let cancelled = false;
    let intervalId = 0;
    const load = async () => {
      try {
        const topology = await actions.listConnections(selectedRoom.metaverse!.spatial_context);
        const loading = resolveActiveDomeNeighbors(topology, selectedRoom, rooms, {}, {});
        if (cancelled) return;
        setNeighbors(loading);
        const hostingEntries = await Promise.all(loading.map(async (neighbor) => {
          try {
            return [neighbor.room.metaverse!.instance_id, await actions.getHosting(
              neighbor.room.metaverse!.spatial_context,
              neighbor.room.metaverse!.instance_id
            )] as const;
          } catch {
            return [neighbor.room.metaverse!.instance_id, undefined] as const;
          }
        }));
        const hosting = Object.fromEntries(hostingEntries);
        const accessEntries = await Promise.all(loading.map(async (neighbor) => {
          try {
            const decision = await actions.previewTransitionAccess({
              transition_id: `preview-${neighbor.connectionId}`,
              connection_id: neighbor.connectionId,
              topology_digest: neighbor.topologyDigest,
              spatial_context: selectedRoom.metaverse!.spatial_context,
              source_instance_id: selectedRoom.metaverse!.instance_id,
              source_instance_generation: selectedRoom.metaverse!.instance_generation,
              target_instance_id: neighbor.room.metaverse!.instance_id,
              target_instance_generation: neighbor.room.metaverse!.instance_generation,
              participant_pubkey: participantPubkey,
              direction: neighbor.direction,
              requested_at: Date.now(),
            });
            return [neighbor.connectionId, decision.status === 'allowed'] as const;
          } catch {
            return [neighbor.connectionId, false] as const;
          }
        }));
        const accessByConnection = Object.fromEntries(accessEntries);
        const assetStates: Record<string, 'loading' | 'ready' | 'error'> = {};
        const textureUrls: Record<string, { wall: string | null; floor: string | null }> = {};
        await Promise.all(loading.map(async (neighbor) => {
          const metaverse = neighbor.room.metaverse!;
          const refs = [
            ...metaverse.asset_refs,
            metaverse.dome.customization.surface.wall_texture,
            metaverse.dome.customization.surface.floor_texture,
            ...metaverse.dome.customization.persistent_props.map((prop) => prop.asset_ref),
          ].filter((asset): asset is MetaverseAssetRef => Boolean(asset));
          const unique = [...new Map(refs.map((asset) => [asset.blob_hash, asset])).values()];
          try {
            const resolved = new Map<string, string | null>();
            await Promise.all(unique.map(async (asset) => {
              resolved.set(asset.blob_hash, await actions.getBlobPreviewUrl(
                asset.blob_hash,
                asset.mime_type ?? 'application/octet-stream',
                asset.kind
              ));
            }));
            if (unique.some((asset) => !resolved.get(asset.blob_hash))) {
              throw new Error('neighbor Dome asset is unavailable');
            }
            textureUrls[metaverse.instance_id] = {
              wall: metaverse.dome.customization.surface.wall_texture
                ? resolved.get(metaverse.dome.customization.surface.wall_texture.blob_hash) ?? null
                : null,
              floor: metaverse.dome.customization.surface.floor_texture
                ? resolved.get(metaverse.dome.customization.surface.floor_texture.blob_hash) ?? null
                : null,
            };
            assetStates[metaverse.instance_id] = 'ready';
          } catch {
            assetStates[metaverse.instance_id] = 'error';
          }
        }));
        if (!cancelled) {
          setNeighbors(resolveActiveDomeNeighbors(
            topology,
            selectedRoom,
            rooms,
            hosting,
            assetStates,
            textureUrls
          ).map((neighbor) => accessByConnection[neighbor.connectionId]
            ? neighbor
            : { ...neighbor, boundaryState: 'closed' }));
        }
      } catch {
        if (!cancelled) {
          setNeighbors((current) => current.map((neighbor) => ({
            ...neighbor,
            boundaryState: 'error',
          })));
        }
      }
    };
    void load();
    intervalId = window.setInterval(() => void load(), 5_000);
    return () => {
      cancelled = true;
      window.clearInterval(intervalId);
    };
  }, [actions, participantPubkey, rooms, selectedRoom, setNeighbors]);

  return state;
}
