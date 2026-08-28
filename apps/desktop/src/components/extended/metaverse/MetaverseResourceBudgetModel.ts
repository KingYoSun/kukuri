import type {
  ClientResourceBudget,
  MetaverseAssetRef,
} from '@/lib/api';

const MIB = 1024 * 1024;
const GIB = 1024 * MIB;

export const DEFAULT_CLIENT_RESOURCE_BUDGET: ClientResourceBudget = {
  max_rendered_avatars: 32,
  max_texture_memory_bytes: 512 * MIB,
  max_rendered_triangles: 3_000_000,
  max_interpolated_bodies: 256,
  max_neighbor_domes: 4,
  cache_capacity_bytes: GIB,
};

export const METAVERSE_CLIENT_BUDGET_STORAGE_KEY = 'kukuri.metaverse.client-budget.v1';

const CLIENT_SAFETY_CEILING: ClientResourceBudget = {
  max_rendered_avatars: 512,
  max_texture_memory_bytes: 2 * GIB,
  max_rendered_triangles: 20_000_000,
  max_interpolated_bodies: 4_096,
  max_neighbor_domes: 4,
  cache_capacity_bytes: 64 * GIB,
};

export type ClientRenderTier = 'full' | 'reduced' | 'fallback' | 'minimal';

export type ClientResourcePlan = {
  tier: ClientRenderTier;
  fullAvatarPeerIds: string[];
  fallbackAvatarPeerIds: string[];
  hiddenAvatarPeerIds: string[];
  renderedPropIds: string[];
  texturesEnabled: boolean;
  interpolationEnabled: boolean;
  dpr: [number, number];
  neighborQuality: Array<'reduced' | 'fallback' | 'hidden'>;
};

export type ClientResourcePlanInput = {
  budget: ClientResourceBudget;
  currentDomeAssets: MetaverseAssetRef[];
  remoteAvatars: Array<{ peerId: string; asset: MetaverseAssetRef | null }>;
  propIds: string[];
  neighborDomes?: Array<{ textureBytes: number; triangles: number }>;
};

function assetTriangles(asset: MetaverseAssetRef | null): number {
  return asset?.budget_metadata?.model_triangles ?? 0;
}

function decodedTextureBytes(assets: MetaverseAssetRef[]): number {
  return assets.reduce(
    (total, asset) => total + (asset.budget_metadata?.decoded_texture_bytes ?? 0),
    0
  );
}

export function createClientResourcePlan(input: ClientResourcePlanInput): ClientResourcePlan {
  const avatars = [...input.remoteAvatars].sort((left, right) => left.peerId.localeCompare(right.peerId));
  const fullAvatarPeerIds: string[] = [];
  const fallbackAvatarPeerIds: string[] = [];
  const hiddenAvatarPeerIds: string[] = [];
  let triangles = input.currentDomeAssets.reduce(
    (total, asset) => total + (asset.budget_metadata?.model_triangles ?? 0),
    0
  );

  avatars.forEach((avatar, index) => {
    if (index >= input.budget.max_rendered_avatars) {
      hiddenAvatarPeerIds.push(avatar.peerId);
      return;
    }
    const requested = assetTriangles(avatar.asset);
    if (requested > 0 && triangles + requested <= input.budget.max_rendered_triangles) {
      triangles += requested;
      fullAvatarPeerIds.push(avatar.peerId);
    } else {
      fallbackAvatarPeerIds.push(avatar.peerId);
    }
  });

  const texturesEnabled =
    decodedTextureBytes(input.currentDomeAssets) <= input.budget.max_texture_memory_bytes;
  const renderedPropIds = input.propIds
    .slice()
    .sort()
    .slice(0, input.budget.max_interpolated_bodies);
  const interpolationEnabled = input.propIds.length <= input.budget.max_interpolated_bodies;
  const neighborDomes = (input.neighborDomes ?? []).slice(0, input.budget.max_neighbor_domes);
  let neighborTextureBytes = decodedTextureBytes(input.currentDomeAssets);
  let neighborTriangles = triangles;
  const neighborQuality = neighborDomes.map((neighbor) => {
    if (
      neighborTextureBytes + neighbor.textureBytes <= input.budget.max_texture_memory_bytes &&
      neighborTriangles + neighbor.triangles <= input.budget.max_rendered_triangles
    ) {
      neighborTextureBytes += neighbor.textureBytes;
      neighborTriangles += neighbor.triangles;
      return 'reduced' as const;
    }
    if (neighborTriangles < input.budget.max_rendered_triangles) {
      return 'fallback' as const;
    }
    return 'hidden' as const;
  });
  const degraded =
    !texturesEnabled ||
    fallbackAvatarPeerIds.length > 0 ||
    hiddenAvatarPeerIds.length > 0 ||
    !interpolationEnabled ||
    neighborQuality.some((quality) => quality !== 'reduced');
  const minimal = hiddenAvatarPeerIds.length > 0 || neighborQuality.includes('hidden');
  const fallback = fallbackAvatarPeerIds.length > 0 || neighborQuality.includes('fallback');
  return {
    tier: minimal ? 'minimal' : fallback ? 'fallback' : degraded ? 'reduced' : 'full',
    fullAvatarPeerIds,
    fallbackAvatarPeerIds,
    hiddenAvatarPeerIds,
    renderedPropIds,
    texturesEnabled,
    interpolationEnabled,
    dpr: degraded ? [1, 1] : [1, 2],
    neighborQuality,
  };
}

export function readClientResourceBudget(storage: Pick<Storage, 'getItem'> | null): ClientResourceBudget {
  const raw = storage?.getItem(METAVERSE_CLIENT_BUDGET_STORAGE_KEY);
  if (!raw) return DEFAULT_CLIENT_RESOURCE_BUDGET;
  try {
    const parsed = JSON.parse(raw) as Partial<ClientResourceBudget>;
    const budget = { ...DEFAULT_CLIENT_RESOURCE_BUDGET, ...parsed };
    const values = Object.values(budget);
    if (
      values.some((value) => !Number.isSafeInteger(value) || value <= 0) ||
      Object.entries(budget).some(
        ([key, value]) => value > CLIENT_SAFETY_CEILING[key as keyof ClientResourceBudget]
      )
    ) {
      return DEFAULT_CLIENT_RESOURCE_BUDGET;
    }
    return budget;
  } catch {
    return DEFAULT_CLIENT_RESOURCE_BUDGET;
  }
}
