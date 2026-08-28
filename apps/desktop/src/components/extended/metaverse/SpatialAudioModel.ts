import type { DomeDirection } from '@/lib/api';
import { DOME_CONNECTION_BOUNDARY_OFFSET_CM } from './DomeSceneModel';

export const METAVERSE_AUDIO_SAMPLE_RATE_HZ = 16_000;
export const METAVERSE_AUDIO_MAX_SAMPLES_PER_FRAME = 320;

export function spatialAudioGain(distanceCm: number): number {
  if (!Number.isFinite(distanceCm) || distanceCm < 0) return 0;
  return Math.min(1, 100 / Math.max(100, distanceCm));
}

export function distanceCm(left: [number, number, number], right: [number, number, number]): number {
  return Math.hypot(left[0] - right[0], left[1] - right[1], left[2] - right[2]);
}

export function domeOpeningPosition(direction: DomeDirection): [number, number, number] {
  if (direction === 'north') return [0, 250, -DOME_CONNECTION_BOUNDARY_OFFSET_CM];
  if (direction === 'east') return [DOME_CONNECTION_BOUNDARY_OFFSET_CM, 250, 0];
  if (direction === 'south') return [0, 250, DOME_CONNECTION_BOUNDARY_OFFSET_CM];
  return [-DOME_CONNECTION_BOUNDARY_OFFSET_CM, 250, 0];
}

export function connectionOpeningAudioDistanceCm(
  speaker: [number, number, number],
  speakerDirection: DomeDirection,
  listenerDirection: DomeDirection,
  listener: [number, number, number]
): number {
  return distanceCm(speaker, domeOpeningPosition(speakerDirection))
    + distanceCm(domeOpeningPosition(listenerDirection), listener);
}

export function resamplePcm16(input: Float32Array, inputRate: number): number[] {
  if (inputRate <= 0 || input.length === 0) return [];
  const outputLength = Math.min(
    METAVERSE_AUDIO_MAX_SAMPLES_PER_FRAME,
    Math.floor(input.length * METAVERSE_AUDIO_SAMPLE_RATE_HZ / inputRate)
  );
  const output = new Array<number>(outputLength);
  for (let index = 0; index < outputLength; index += 1) {
    const sourceIndex = Math.min(input.length - 1, Math.floor(index * inputRate / METAVERSE_AUDIO_SAMPLE_RATE_HZ));
    output[index] = Math.round(Math.max(-1, Math.min(1, input[sourceIndex])) * 32_767);
  }
  return output;
}
