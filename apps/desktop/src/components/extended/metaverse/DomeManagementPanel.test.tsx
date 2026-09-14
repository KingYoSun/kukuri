import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { expect, test, vi } from 'vitest';
import type { GameRoomView } from '@/lib/api';
import { createDesktopMockApi } from '@/mocks/desktopApiMock';
import { createMetaverseRoomActions } from '@/shell/actions/metaverse';
import { createDefaultMetaverseRoomState } from './DomeSceneModel';
import { DomeManagementPanel } from './DomeManagementPanel';

const room: GameRoomView = {
  room_id: 'dome-test', title: 'Owner Dome', description: '', host_pubkey: 'f'.repeat(64), status: 'Waiting',
  scores: [], phase_label: null, room_kind: 'metaverse_room', metaverse: createDefaultMetaverseRoomState(8),
  dome_hosting: { kind: 'closed' }, manifest_blob_hash: 'manifest', updated_at: 1, channel_id: null, audience_label: 'Public',
};
function setup() {
  const api = createDesktopMockApi();
  const actions = createMetaverseRoomActions({ api, activeTopic: 'kukuri:topic:demo', activeComposeChannel: { kind: 'public' }, onRefresh: vi.fn() });
  actions.startOwnerHosting = vi.fn(actions.startOwnerHosting);
  actions.deleteRoom = vi.fn().mockResolvedValue({ deleted: true, cleanup_pending: false });
  const onEnter = vi.fn().mockResolvedValue(false);
  const onDeleted = vi.fn();
  const view = render(<DomeManagementPanel room={room} actions={actions} endpointId='endpoint' locale='en'
    admitted={false} onEnter={onEnter} onDeleted={onDeleted} onStopped={vi.fn()} onClose={vi.fn()} />);
  return { ...view, actions, onEnter, onDeleted };
}
test('management reads without starting, and retrying failed entry does not restart hosting', async () => {
  const user = userEvent.setup();
  const { actions, onEnter } = setup();
  await waitFor(() => expect(screen.getByRole('button', { name: 'Start hosting and enter' })).toBeEnabled());
  expect(actions.startOwnerHosting).not.toHaveBeenCalled();
  expect(onEnter).not.toHaveBeenCalled();
  await user.click(screen.getByRole('button', { name: 'Start hosting and enter' }));
  await screen.findByText('Hosting is running, but entry failed. Retry entering.');
  await user.click(screen.getByRole('button', { name: 'Join Room' }));
  expect(actions.startOwnerHosting).toHaveBeenCalledTimes(1);
  expect(onEnter).toHaveBeenCalledTimes(2);
});
test('cancel is inert; confirmation binds deletion to the displayed generation', async () => {
  const user = userEvent.setup();
  const { actions, onDeleted } = setup();
  await waitFor(() => expect(screen.getByRole('button', { name: 'Delete Dome' })).toBeEnabled());
  await user.click(screen.getByRole('button', { name: 'Delete Dome' }));
  await user.click(screen.getByRole('button', { name: 'Cancel' }));
  expect(actions.deleteRoom).not.toHaveBeenCalled();
  await user.click(screen.getByRole('button', { name: 'Delete Dome' }));
  await user.click(screen.getByRole('button', { name: 'Confirm deletion' }));
  expect(actions.deleteRoom).toHaveBeenCalledWith(room.metaverse!.spatial_context, room.metaverse!.instance_id,
    room.metaverse!.instance_generation, `delete-${room.metaverse!.instance_id}-${room.metaverse!.instance_generation}`);
  expect(onDeleted).toHaveBeenCalledOnce();
});
test('failed deletion retains target and reuses the same operation on retry', async () => {
  const user = userEvent.setup();
  const { actions } = setup();
  vi.mocked(actions.deleteRoom).mockRejectedValueOnce(new Error('storage unavailable'));
  await waitFor(() => expect(screen.getByRole('button', { name: 'Delete Dome' })).toBeEnabled());
  await user.click(screen.getByRole('button', { name: 'Delete Dome' }));
  await user.click(screen.getByRole('button', { name: 'Confirm deletion' }));
  expect(screen.getByRole('dialog')).toHaveTextContent('storage unavailable');
  await user.click(screen.getByRole('button', { name: 'Confirm deletion' }));
  const calls = vi.mocked(actions.deleteRoom).mock.calls;
  expect(calls[1]).toEqual(calls[0]);
});
