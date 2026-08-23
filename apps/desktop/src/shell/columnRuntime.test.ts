import { describe, expect, it } from 'vitest';

import {
  projectColumnRuntime,
  releaseColumnAudioFocus,
  requestColumnAudioFocus,
} from '@/shell/columnRuntime';

describe('Column runtime projection', () => {
  it('keeps active, visible, audio focus, and suspension independent', () => {
    expect(projectColumnRuntime({
      kind: 'metaverse',
      columnId: 'room',
      activeColumnId: 'timeline',
      visibleColumnIds: new Set(['timeline']),
      audioFocusedColumnId: null,
    })).toEqual({ visible: false, active: false, audioFocused: false, suspended: true });

    expect(projectColumnRuntime({
      kind: 'timeline',
      columnId: 'timeline',
      activeColumnId: 'timeline',
      visibleColumnIds: new Set(['timeline', 'thread']),
      audioFocusedColumnId: null,
    })).toEqual({ visible: true, active: true, audioFocused: false, suspended: false });
  });

  it('keeps an explicitly audio-focused Stream alive offscreen and makes focus exclusive', () => {
    let audioFocus: string | null = null;
    audioFocus = requestColumnAudioFocus(audioFocus, 'stream-a');
    audioFocus = requestColumnAudioFocus(audioFocus, 'stream-b');
    expect(audioFocus).toBe('stream-b');
    expect(projectColumnRuntime({
      kind: 'stream',
      columnId: 'stream-b',
      activeColumnId: 'timeline',
      visibleColumnIds: new Set(),
      audioFocusedColumnId: audioFocus,
    }).suspended).toBe(false);
    expect(releaseColumnAudioFocus(audioFocus, 'stream-a')).toBe('stream-b');
    expect(releaseColumnAudioFocus(audioFocus, 'stream-b')).toBeNull();
  });
});
