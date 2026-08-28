import type { SpatialContextV1 } from '../types';
import type {
  CommitDomeLayoutRequest,
  ResyncDomeSnapshotsRequest,
} from '../types.generated';

export const commitDomeLayoutRequest = (
  spatialContext: SpatialContextV1,
  instanceId: string,
  operationId: string
): CommitDomeLayoutRequest => ({
  spatial_context: spatialContext,
  instance_id: instanceId,
  operation_id: operationId,
});

export const resyncDomeSnapshotsRequest = (
  spatialContext: SpatialContextV1,
  instanceId: string,
  afterSequence: number
): ResyncDomeSnapshotsRequest => ({
  spatial_context: spatialContext,
  instance_id: instanceId,
  after_sequence: afterSequence,
});
