import { describe, expect, it } from 'vitest';
import type { MetaverseColliderV1 } from '@/lib/api';

import {
  DOME_ADJACENT_CENTER_DISTANCE_CM,
  DOME_DIRECTIONS,
  clampAvatarToDome,
  createDefaultMetaverseRoomState,
  createDomeInteractionInput,
  domeDirectionOffset,
  interpolateDomeEnvironment,
  isDomeCustomizationValid,
  openingContains,
  resolveDomeCollider,
} from './DomeSceneModel';

describe('DomeSceneModel', () => {
  it('固定Domeと4方向の57m offsetを定義する', () => {
    const state = createDefaultMetaverseRoomState(8);
    expect(state.world_version).toBe(6);
    expect(state.preset_ref.revision).toBe(1);
    expect(state.dome.spec_id).toBe('fixed_dome_v1');
    expect(DOME_DIRECTIONS.map(domeDirectionOffset)).toEqual([
      [0, 0, -DOME_ADJACENT_CENTER_DISTANCE_CM],
      [DOME_ADJACENT_CENTER_DISTANCE_CM, 0, 0],
      [0, 0, DOME_ADJACENT_CENTER_DISTANCE_CM],
      [-DOME_ADJACENT_CENTER_DISTANCE_CM, 0, 0],
    ]);
  });

  it('幅5mかつ上部半径2.5mの開口部を判定する', () => {
    expect(openingContains(250, 750)).toBe(true);
    expect(openingContains(0, 1_000)).toBe(true);
    expect(openingContains(251, 750)).toBe(false);
    expect(openingContains(100, 1_000)).toBe(false);
  });

  it('明示colliderを優先し、欠落時はbounding boxを包含するcapsuleを返す', () => {
    const explicit: MetaverseColliderV1 = {
      shape: 'cuboid',
      center: [0, 50, 0],
      half_extents: [50, 50, 50],
    };
    expect(resolveDomeCollider(explicit, { min: [-100, 0, -50], max: [100, 400, 50] })).toBe(explicit);
    expect(resolveDomeCollider(null, { min: [-100, 0, -50], max: [100, 400, 50] })).toEqual({
      shape: 'capsule',
      center: [0, 200, 0],
      radius: 100,
      half_height: 100,
    });
  });

  it('数値environmentを補間し、通常壁とconnection zone中央で移動を制限する', () => {
    const state = createDefaultMetaverseRoomState();
    expect(interpolateDomeEnvironment(state.dome.customization.environment, {
      key_light_milli: 4_000,
      ambient_light_milli: 1_000,
      fog_density_micros: 20_000,
      gravity_milli: 4_000,
    }, 0.5)).toEqual({
      key_light_milli: 3_200,
      ambient_light_milli: 700,
      fog_density_micros: 14_000,
      gravity_milli: 6_900,
    });
    expect(clampAvatarToDome([2_500, 0, 0])).toEqual([2_500, 0, 0]);
    expect(clampAvatarToDome([3_000, 0, 0])).toEqual([2_850, 0, 0]);
    expect(clampAvatarToDome([2_500, 1_200, 400])).not.toEqual([2_500, 1_200, 400]);
    expect(clampAvatarToDome([1_500, 1_500, 0])).toEqual([1_323, 1_500, 0]);
    expect(clampAvatarToDome([100, 2_000, 0])).toEqual([0, 2_000, 0]);
  });

  it('許可範囲外のcustomizationをclient側でも検出する', () => {
    const customization = createDefaultMetaverseRoomState().dome.customization;
    expect(isDomeCustomizationValid(customization)).toBe(true);
    expect(isDomeCustomizationValid({
      ...customization,
      environment: { ...customization.environment, gravity_milli: 999 },
    })).toBe(false);
    expect(isDomeCustomizationValid({
      ...customization,
      persistent_props: [{
        ...customization.persistent_props[0],
        position: [1_500, 1_500, 0],
      }],
    })).toBe(false);
  });

  it('共通interaction入力をactorとpropに紐づける', () => {
    expect(createDomeInteractionInput('sit', 'dome-prop-1', 'peer-a', 42)).toEqual({
      type: 'sit',
      propId: 'dome-prop-1',
      actorPeerId: 'peer-a',
      issuedAt: 42,
    });
  });
});
