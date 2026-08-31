import { render, screen } from '@testing-library/react';
import { describe, expect, test, vi } from 'vitest';

import i18n, { type SupportedLocale } from '@/i18n';
import type { GameRoomView } from '@/lib/api';

import { GameRoomPanel } from './GameRoomPanel';
import { ProfileEditorPanel } from './ProfileEditorPanel';
import { ProfileOverviewPanel } from './ProfileOverviewPanel';

const room: GameRoomView = {
  room_id: 'game-room-1',
  host_pubkey: 'f'.repeat(64),
  title: 'Final',
  description: 'Championship',
  status: 'Running',
  phase_label: 'Round 1',
  scores: [{ participant_id: 'alice', label: 'Alice', score: 3 }],
  room_kind: 'score_game',
  metaverse: null,
  dome_hosting: null,
  manifest_blob_hash: 'game-room-manifest',
  updated_at: 1,
  channel_id: null,
  audience_label: 'Public',
};

const expectations: Array<{
  locale: SupportedLocale;
  overviewAlt: string;
  previewAlt: string;
  score: string;
  status: string;
  phase: string;
}> = [
  {
    locale: 'en',
    overviewAlt: "Alex's profile picture",
    previewAlt: "Alex's profile picture preview",
    score: "Alice's score in Final",
    status: 'Status for Final',
    phase: 'Phase for Final',
  },
  {
    locale: 'ja',
    overviewAlt: 'Alexのプロフィール画像',
    previewAlt: 'Alexのプロフィール画像プレビュー',
    score: 'FinalでのAliceのスコア',
    status: 'Finalの状態',
    phase: 'Finalのフェーズ',
  },
  {
    locale: 'zh-CN',
    overviewAlt: 'Alex的头像',
    previewAlt: 'Alex的头像预览',
    score: 'Alice在Final中的分数',
    status: 'Final的状态',
    phase: 'Final的阶段',
  },
];

describe.each(expectations)('accessible names in $locale', (expected) => {
  test('uses localized names for profile images and game controls', async () => {
    await i18n.changeLanguage(expected.locale);
    const noop = vi.fn();

    render(
      <>
        <ProfileOverviewPanel
          authorLabel='Alex'
          about={null}
          picture='data:image/png;base64,AA=='
          status='ready'
          error={null}
          postCount={0}
          followingCount={0}
          followedCount={0}
          mutedCount={0}
          onEdit={noop}
          onOpenFollowing={noop}
          onOpenFollowed={noop}
          onOpenMuted={noop}
        />
        <ProfileEditorPanel
          authorLabel='Alex'
          status='ready'
          saving={false}
          dirty={false}
          error={null}
          fields={{ displayName: '', name: '', about: '' }}
          picturePreviewSrc='data:image/png;base64,AA=='
          hasPicture
          pictureInputKey={0}
          onFieldChange={noop}
          onPictureSelect={noop}
          onPictureClear={noop}
          onSave={noop}
          onReset={noop}
        />
        <GameRoomPanel
          status='ready'
          error={null}
          audienceLabel='Public'
          title=''
          description=''
          participantsInput=''
          createPending={false}
          rooms={[room]}
          drafts={{
            [room.room_id]: { status: 'Running', phaseLabel: 'Round 1', scores: { alice: '3' } },
          }}
          savingByRoomId={{}}
          localAuthorPubkey={room.host_pubkey}
          onTitleChange={noop}
          onDescriptionChange={noop}
          onParticipantsChange={noop}
          onSubmit={noop}
          onDraftStatusChange={noop}
          onDraftPhaseChange={noop}
          onDraftScoreChange={noop}
          onSaveRoom={noop}
        />
      </>
    );

    expect(screen.getByAltText(expected.overviewAlt)).toBeInTheDocument();
    expect(screen.getByAltText(expected.previewAlt)).toBeInTheDocument();
    expect(screen.getByRole('textbox', { name: expected.score })).toBeInTheDocument();
    expect(screen.getByRole('combobox', { name: expected.status })).toBeInTheDocument();
    expect(screen.getByRole('textbox', { name: expected.phase })).toBeInTheDocument();
  });
});
