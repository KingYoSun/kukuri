import React from 'react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import type { SyncConflict } from '@/lib/sync/syncEngine';
import { ConflictResolutionDialog } from '@/components/sync/ConflictResolutionDialog';

const mockDialogFactory = vi.hoisted(() => {
  return () => {
    const passthrough = ({ children }: { children?: React.ReactNode }) => <div>{children}</div>;
    return {
      Dialog: passthrough,
      DialogContent: passthrough,
      DialogHeader: passthrough,
      DialogTitle: passthrough,
      DialogDescription: passthrough,
      DialogFooter: passthrough,
    };
  };
});

vi.mock('@/components/ui/dialog', () => {
  const mocked = mockDialogFactory();
  return {
    ...mocked,
  };
});

describe('ConflictResolutionDialog', () => {
  const baseConflict: SyncConflict = {
    localAction: {
      id: 1,
      localId: 'local_1',
      userPubkey: 'npub1',
      actionType: 'create_post',
      actionData: JSON.stringify({ topicId: 'topic1' }),
      createdAt: '2024-01-01T00:00:00Z',
      isSynced: false,
    },
    conflictType: 'timestamp',
  };

  let onResolve: ReturnType<typeof vi.fn>;
  let onClose: ReturnType<typeof vi.fn>;

  beforeEach(() => {
    onResolve = vi.fn().mockResolvedValue(undefined);
    onClose = vi.fn();
  });

  it('ローカル/リモートの詳細を表示し、ローカルを適用できる', async () => {
    render(
      <ConflictResolutionDialog
        conflicts={[baseConflict]}
        isOpen
        onResolve={onResolve}
        onClose={onClose}
      />,
    );

    expect(screen.getByText('ローカルの変更')).toBeInTheDocument();
    expect(screen.getByText('create_post')).toBeInTheDocument();

    await userEvent.click(screen.getByText('適用'));

    await waitFor(() => {
      expect(onResolve).toHaveBeenCalledWith(baseConflict, 'local');
    });
  });

  it('リモートを選択して適用できる', async () => {
    const conflict: SyncConflict = {
      ...baseConflict,
      remoteAction: {
        id: 2,
        localId: 'remote_1',
        userPubkey: 'npub2',
        actionType: 'create_post',
        actionData: JSON.stringify({ topicId: 'topic1' }),
        createdAt: '2024-01-02T00:00:00Z',
        isSynced: true,
      },
    };

    render(
      <ConflictResolutionDialog
        conflicts={[conflict]}
        isOpen
        onResolve={onResolve}
        onClose={onClose}
      />,
    );

    await userEvent.click(screen.getByLabelText('リモートの変更を優先する'));
    await userEvent.click(screen.getByText('適用'));

    await waitFor(() => {
      expect(onResolve).toHaveBeenCalledWith(conflict, 'remote');
    });
  });

  it('Doc/Blob タブで比較を表示する', async () => {
    const conflict: SyncConflict = {
      ...baseConflict,
      localAction: {
        ...baseConflict.localAction,
        actionData: JSON.stringify({
          docVersion: 2,
          blobHash: 'local-hash',
          payloadBytes: 2048,
        }),
      },
      remoteAction: {
        id: 2,
        localId: 'remote_doc',
        userPubkey: 'npubX',
        actionType: 'profile_update',
        actionData: JSON.stringify({
          docVersion: 3,
          blobHash: 'remote-hash',
          payloadBytes: 4096,
        }),
        createdAt: '2024-01-02T00:00:00Z',
        isSynced: true,
      },
      conflictType: 'version',
    };

    render(
      <ConflictResolutionDialog
        conflicts={[conflict]}
        isOpen
        onResolve={onResolve}
        onClose={onClose}
      />,
    );

    await userEvent.click(screen.getByRole('tab', { name: 'Doc/Blob' }));

    expect(screen.getAllByText('ローカル').length).toBeGreaterThan(0);
    expect(screen.getAllByText('リモート').length).toBeGreaterThan(0);
    expect(screen.getByText('Doc Version')).toBeInTheDocument();
    expect(screen.getByText('2')).toBeInTheDocument();
    expect(screen.getByText('3')).toBeInTheDocument();
  });
});
