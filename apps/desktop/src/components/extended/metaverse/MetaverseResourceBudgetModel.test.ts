import { describe, expect, it } from 'vitest';

import type { MetaverseAssetRef } from '@/lib/api';
import {
  DEFAULT_CLIENT_RESOURCE_BUDGET,
  createClientResourcePlan,
  readClientResourceBudget,
} from './MetaverseResourceBudgetModel';

function model(name: string, triangles: number): MetaverseAssetRef {
  return {
    kind: 'vrm',
    blob_hash: name.padEnd(64, '0'),
    size_bytes: 1024,
    name,
    budget_metadata: {
      stored_bytes: 1024,
      decoded_texture_bytes: 0,
      model_triangles: triangles,
      model_primitives: 1,
    },
  };
}

describe('createClientResourcePlan', () => {
  it('falls back and then hides remote avatars deterministically at the boundary', () => {
    const budget = {
      ...DEFAULT_CLIENT_RESOURCE_BUDGET,
      max_rendered_avatars: 2,
      max_rendered_triangles: 100,
    };
    const plan = createClientResourcePlan({
      budget,
      currentDomeAssets: [],
      remoteAvatars: [
        { peerId: 'c', asset: model('c', 10) },
        { peerId: 'a', asset: model('a', 80) },
        { peerId: 'b', asset: model('b', 80) },
      ],
      propIds: [],
    });
    expect(plan.fullAvatarPeerIds).toEqual(['a']);
    expect(plan.fallbackAvatarPeerIds).toEqual(['b']);
    expect(plan.hiddenAvatarPeerIds).toEqual(['c']);
    expect(plan.tier).toBe('minimal');
  });

  it('degrades current plus four maximum neighbors without stopping the current Dome', () => {
    const plan = createClientResourcePlan({
      budget: DEFAULT_CLIENT_RESOURCE_BUDGET,
      currentDomeAssets: [model('current', 2_000_000)],
      remoteAvatars: [],
      propIds: ['current-prop'],
      neighborDomes: Array.from({ length: 4 }, () => ({
        textureBytes: DEFAULT_CLIENT_RESOURCE_BUDGET.max_texture_memory_bytes,
        triangles: DEFAULT_CLIENT_RESOURCE_BUDGET.max_rendered_triangles,
      })),
    });
    expect(plan.renderedPropIds).toEqual(['current-prop']);
    expect(plan.neighborQuality).toHaveLength(4);
    expect(plan.neighborQuality.every((quality) => quality !== 'reduced')).toBe(true);
  });

  it('falls back to safe defaults for malformed local configuration', () => {
    expect(readClientResourceBudget({ getItem: () => '{broken' })).toEqual(
      DEFAULT_CLIENT_RESOURCE_BUDGET
    );
    expect(readClientResourceBudget({ getItem: () => JSON.stringify({ max_rendered_avatars: 0 }) }))
      .toEqual(DEFAULT_CLIENT_RESOURCE_BUDGET);
    expect(
      readClientResourceBudget({
        getItem: () => JSON.stringify({ max_rendered_triangles: 20_000_001 }),
      })
    ).toEqual(DEFAULT_CLIENT_RESOURCE_BUDGET);
  });
});
