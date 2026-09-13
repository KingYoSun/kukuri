import { expect, test } from 'vitest';

import { profileConnectionsEmptyGuidance } from './profileConnectionsEmptyGuidance';

test.each([
  ['following', ['openProfile', 'follow'], [], ['openTimeline', 'openExplore']],
  ['followed', ['shareOwnId'], ['followedInfo'], ['copyOwnId', 'openTimeline']],
  ['muted', ['openProfile', 'mute'], ['muteDeviceOnly'], ['openTimeline']],
  ['blocking', ['openProfile', 'block'], ['blockSigned'], ['openTimeline']],
] as const)('%s lists its steps, notes, and actions', (view, steps, notes, actions) => {
  expect(profileConnectionsEmptyGuidance(view)).toEqual({ steps, notes, actions });
});

test('every view names at least one concrete next action', () => {
  for (const view of ['following', 'followed', 'muted', 'blocking'] as const) {
    expect(profileConnectionsEmptyGuidance(view).actions.length).toBeGreaterThan(0);
  }
});
