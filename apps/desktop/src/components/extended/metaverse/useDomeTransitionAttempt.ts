import { useCallback, useRef, useState, type Dispatch, type SetStateAction } from 'react';

import type { DomeDirection, DomeSessionInputKindV1, DomeTransitionAdmissionTicketV1, GameRoomView } from '@/lib/api';
import type { AvatarTransform } from '../MetaverseSceneModel';
import type { MetaverseRoomActions } from './MetaverseRoomActions';
import {
  domeTransitionProgress,
  transitionNeighborAtPosition,
  transitionNeighborInZone,
  transformAvatarBetweenDomes,
  type DomeNeighborTransitionView,
} from './DomeTransitionModel';
import { recoverDomeTransitionCommit } from './DomeTransitionCommitRecovery';
import { writeLastVisitedDome } from './DomeEntryModel';

type TransitionAttempt = {
  id: string;
  sourceRoom: GameRoomView;
  neighbor: DomeNeighborTransitionView;
  phase: 'preparing' | 'provisional' | 'committing' | 'target_committed';
  ticket: DomeTransitionAdmissionTicketV1 | null;
  cancelled: boolean;
};

type UseDomeTransitionAttemptArgs = {
  actions: MetaverseRoomActions;
  admittedRoom: GameRoomView | null;
  localAuthorPubkey: string;
  localPeerId: string;
  transitionNeighbors: DomeNeighborTransitionView[];
  setTransitionNeighbors: Dispatch<SetStateAction<DomeNeighborTransitionView[]>>;
  submitInputForRoom: (
    room: GameRoomView,
    input: DomeSessionInputKindV1,
    suggestedSequence?: number
  ) => ReturnType<MetaverseRoomActions['submitSessionInput']>;
  onHandoff: (sourceRoom: GameRoomView, targetRoom: GameRoomView, transform: AvatarTransform) => void;
  onError: (message: string | null) => void;
};

// Owns each asynchronous attempt; admission and the input sequence remain in the session.
export function useDomeTransitionAttempt({
  actions, admittedRoom, localAuthorPubkey, localPeerId, transitionNeighbors,
  setTransitionNeighbors, submitInputForRoom, onHandoff, onError,
}: UseDomeTransitionAttemptArgs) {
  const [transitionPreparingDirections, setTransitionPreparingDirections] = useState<Set<DomeDirection>>(
    () => new Set()
  );
  const transitionAttemptRef = useRef<TransitionAttempt | null>(null);

  function setTransitionPreparing(direction: DomeDirection, preparing: boolean) {
    setTransitionPreparingDirections((current) => {
      const next = new Set(current);
      if (preparing) next.add(direction);
      else next.delete(direction);
      return next;
    });
  }

  const abortTransitionAttempt = useCallback(async (attempt: TransitionAttempt) => {
    if (attempt.phase === 'committing' || attempt.phase === 'target_committed') return;
    attempt.cancelled = true;
    if (attempt.ticket) {
      await actions.abortTransition(attempt.ticket).catch(() => undefined);
    }
    await submitInputForRoom(attempt.sourceRoom, {
      type: 'abort_transition',
      transition_id: attempt.id,
    }).catch(() => undefined);
    if (transitionAttemptRef.current === attempt) {
      transitionAttemptRef.current = null;
    }
    setTransitionPreparingDirections((current) => {
      const next = new Set(current);
      next.delete(attempt.neighbor.direction);
      return next;
    });
  }, [actions, submitInputForRoom]);

  function beginTransitionAttempt(neighbor: DomeNeighborTransitionView) {
    if (!admittedRoom?.metaverse || transitionAttemptRef.current) return;
    const transitionId = `dome-transition-${globalThis.crypto?.randomUUID?.() ?? Date.now()}`;
    const attempt: TransitionAttempt = {
      id: transitionId,
      sourceRoom: admittedRoom,
      neighbor,
      phase: 'preparing',
      ticket: null,
      cancelled: false,
    };
    transitionAttemptRef.current = attempt;
    setTransitionPreparing(neighbor.direction, true);
    void (async () => {
      try {
        await submitInputForRoom(attempt.sourceRoom, {
          type: 'prepare_transition',
          transition_id: attempt.id,
          direction: neighbor.direction,
        });
        if (attempt.cancelled) return;
        attempt.ticket = await actions.prepareTransition({
          transition_id: attempt.id,
          connection_id: neighbor.connectionId,
          topology_digest: neighbor.topologyDigest,
          spatial_context: attempt.sourceRoom.metaverse!.spatial_context,
          source_instance_id: attempt.sourceRoom.metaverse!.instance_id,
          source_instance_generation: attempt.sourceRoom.metaverse!.instance_generation,
          target_instance_id: neighbor.room.metaverse!.instance_id,
          target_instance_generation: neighbor.room.metaverse!.instance_generation,
          participant_pubkey: localAuthorPubkey,
          direction: neighbor.direction,
          requested_at: Date.now(),
        });
        if (attempt.cancelled) {
          await abortTransitionAttempt(attempt);
          return;
        }
        attempt.phase = 'provisional';
        setTransitionPreparing(neighbor.direction, false);
      } catch (transitionError) {
        await abortTransitionAttempt(attempt);
        setTransitionNeighbors((current) => current.map((candidate) =>
          candidate.connectionId === neighbor.connectionId
            ? { ...candidate, boundaryState: 'error' }
            : candidate
        ));
        onError(
          transitionError instanceof Error
            ? transitionError.message
            : 'Dome transition preparation failed'
        );
      }
    })();
  }

  function commitTransitionAttempt(attempt: TransitionAttempt, transform: AvatarTransform) {
    if (attempt.phase !== 'provisional' || attempt.cancelled || !attempt.ticket) return;
    attempt.phase = 'committing';
    const targetPosition = transformAvatarBetweenDomes(transform.position, attempt.neighbor.relativeCoordinateCm);
    const targetTransform: AvatarTransform = {
      ...transform, roomId: attempt.neighbor.room.room_id, seq: 0,
      position: targetPosition, sentAt: Date.now(),
    };
    void (async () => {
      const recovery = await recoverDomeTransitionCommit({
        ticket: attempt.ticket!,
        commit: () => actions.commitTransition(attempt.ticket!, targetPosition, transform.rotation),
        getHosting: () => actions.getHosting(attempt.ticket!.request.spatial_context, attempt.ticket!.request.target_instance_id),
        isCurrent: () => transitionAttemptRef.current === attempt && !attempt.cancelled,
      });
      if (recovery.status === 'cancelled') return;
      if (recovery.status === 'rollback') {
        attempt.phase = 'provisional';
        await abortTransitionAttempt(attempt);
        setTransitionNeighbors((current) => current.map((candidate) =>
          candidate.connectionId === attempt.neighbor.connectionId
            ? { ...candidate, boundaryState: 'error' }
            : candidate
        ));
        onError(recovery.error instanceof Error
          ? recovery.error.message
          : 'Dome transition commit failed');
        return;
      }
      attempt.phase = 'target_committed';
      transitionAttemptRef.current = null;
      setTransitionPreparing(attempt.neighbor.direction, false);
      onHandoff(attempt.sourceRoom, attempt.neighbor.room, targetTransform);
      if (attempt.neighbor.room.metaverse) {
        writeLastVisitedDome(
          localAuthorPubkey,
          attempt.neighbor.room.metaverse.spatial_context,
          attempt.neighbor.room.metaverse.instance_id
        );
      }
      onError(null);
      let sourceCompleted = false;
      for (const retryDelay of [0, 250, 1_000]) {
        if (retryDelay > 0) {
          await new Promise((resolve) => window.setTimeout(resolve, retryDelay));
        }
        try {
          await submitInputForRoom(attempt.sourceRoom, {
            type: 'complete_transition',
            transition_id: attempt.id,
          });
          sourceCompleted = true;
          break;
        } catch {
          // The destination remains authoritative; retry only source cleanup.
        }
      }
      if (sourceCompleted) {
        const leftAt = Date.now();
        await actions.publishRoomEvent(attempt.sourceRoom.room_id, localPeerId, leftAt, {
          type: 'presence_leave',
          room_id: attempt.sourceRoom.room_id,
          peer_id: localPeerId,
          left_at: leftAt,
        }).catch(() => undefined);
      } else {
        onError('Destination committed; source Dome cleanup will require resynchronization');
      }
    })();
  }

  const requestTransitionAbort = useCallback(() => {
    const attempt = transitionAttemptRef.current;
    if (attempt) void abortTransitionAttempt(attempt);
  }, [abortTransitionAttempt]);

  function handleTransitionTransform(transform: AvatarTransform, previous: AvatarTransform | null) {
    const attempt = transitionAttemptRef.current;
    const inZone = transitionNeighborInZone(transform.position, transitionNeighbors);
    if (!attempt && inZone) {
      beginTransitionAttempt(inZone);
      return;
    }
    if (!attempt) return;
    if (
      attempt.phase !== 'committing' &&
      domeTransitionProgress(transform.position, attempt.neighbor.direction) <= 0
    ) {
      void abortTransitionAttempt(attempt);
      return;
    }
    const crossed = transitionNeighborAtPosition(previous?.position ?? null, transform.position, [
      attempt.neighbor,
    ]);
    if (crossed && attempt.phase === 'provisional') {
      commitTransitionAttempt(attempt, transform);
    }
  }
  return { transitionPreparingDirections, requestTransitionAbort, handleTransitionTransform };
}
