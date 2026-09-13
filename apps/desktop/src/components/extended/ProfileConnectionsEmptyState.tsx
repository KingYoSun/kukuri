import { Trans, useTranslation } from 'react-i18next';
import { Flag } from 'lucide-react';

import { EmptyStateGuidance, type EmptyStateGuidanceAction } from '@/components/core/EmptyStateGuidance';
import { ActionRef } from '@/components/ui/action-ref';
import type { ProfileConnectionsView } from '@/components/shell/types';
import { copyTextToClipboard } from '@/lib/utils';

import {
  type ConnectionsEmptyAction,
  type ConnectionsEmptyNote,
  type ConnectionsEmptyStep,
  profileConnectionsEmptyGuidance,
} from './profileConnectionsEmptyGuidance';

type ProfileConnectionsEmptyStateProps = {
  view: ProfileConnectionsView;
  localAuthorPubkey: string;
  onOpenTimeline?: () => void;
  onOpenExplore?: () => void;
};

/**
 * ソーシャル一覧の取得成功 0 件(#994)。プロフィールの文字ボタン(フォロー / ミュート / ブロック)と
 * 投稿の通報 icon を chip で示し、既存導線(タイムライン / 見つける / 自分の ID コピー)だけを CTA にする。
 */
export function ProfileConnectionsEmptyState({
  view,
  localAuthorPubkey,
  onOpenTimeline,
  onOpenExplore,
}: ProfileConnectionsEmptyStateProps) {
  const { t } = useTranslation(['profile', 'common', 'settings', 'shell']);
  const guidance = profileConnectionsEmptyGuidance(view);

  const renderStep = (step: ConnectionsEmptyStep) => {
    switch (step) {
      case 'openProfile':
        return t('profile:connections.emptyGuidance.openProfile');
      case 'follow':
        return (
          <Trans
            i18nKey='profile:connections.emptyGuidance.followStep'
            components={{ action: <ActionRef label={t('common:actions.follow')} /> }}
          />
        );
      case 'mute':
        return (
          <Trans
            i18nKey='profile:connections.emptyGuidance.muteStep'
            components={{
              action: <ActionRef label={t('common:actions.mute')} />,
              report: <ActionRef icon={Flag} label={t('shell:report.actionLabel')} />,
            }}
          />
        );
      case 'block':
        return (
          <Trans
            i18nKey='profile:connections.emptyGuidance.blockStep'
            components={{ action: <ActionRef label={t('common:actions.block')} /> }}
          />
        );
      case 'shareOwnId':
        return t('profile:connections.emptyGuidance.shareOwnId');
      default:
        return null;
    }
  };

  const renderNote = (note: ConnectionsEmptyNote) => {
    switch (note) {
      case 'followedInfo':
        return t('profile:connections.emptyGuidance.followedInfo');
      case 'muteDeviceOnly':
        return t('settings:safety.social.mute');
      case 'blockSigned':
        return t('settings:safety.social.block');
      default:
        return null;
    }
  };

  const toAction = (action: ConnectionsEmptyAction): EmptyStateGuidanceAction | null => {
    switch (action) {
      case 'openTimeline':
        return onOpenTimeline
          ? { id: action, label: t('profile:connections.emptyActions.openTimeline'), onClick: onOpenTimeline }
          : null;
      case 'openExplore':
        return onOpenExplore
          ? { id: action, label: t('profile:connections.emptyActions.openExplore'), onClick: onOpenExplore }
          : null;
      case 'copyOwnId':
        return {
          id: action,
          label: t('common:actions.copyAuthorId'),
          onClick: () => void copyTextToClipboard(localAuthorPubkey),
        };
      default:
        return null;
    }
  };

  return (
    <EmptyStateGuidance
      testId='profile-connections-empty-state'
      title={t(`profile:connections.empty.${view}`)}
      steps={guidance.steps.map((step) => ({ id: step, content: renderStep(step) }))}
      notes={guidance.notes.map((note) => ({ id: note, content: renderNote(note) }))}
      actions={guidance.actions.flatMap((action) => {
        const resolved = toAction(action);
        return resolved ? [resolved] : [];
      })}
    />
  );
}
