import { describe, expect, it } from 'vitest';
import {
  connectionOpeningAudioDistanceCm,
  resamplePcm16,
  spatialAudioGain,
} from './SpatialAudioModel';

describe('SpatialAudioModel', () => {
  it('attenuates by the two opening legs for adjacent Domes', () => {
    const near = connectionOpeningAudioDistanceCm([0, 250, 0], 'south', 'north', [0, 250, -1_900]);
    const far = connectionOpeningAudioDistanceCm([0, 250, -1_900], 'south', 'north', [0, 250, 0]);
    expect(spatialAudioGain(near)).toBeGreaterThan(spatialAudioGain(far));
  });

  it('bounds and converts captured samples to PCM16', () => {
    const samples = resamplePcm16(new Float32Array(2_048).fill(0.5), 48_000);
    expect(samples).toHaveLength(320);
    expect(samples[0]).toBe(16_384);
  });
});
