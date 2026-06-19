import { type CommunityNodeManifestEntry } from '@/shell/store';

import { type CommunityNodeDependencyView } from './types';

type Translate = (key: string, options?: Record<string, unknown>) => string;

const DEFAULT_ONBOARDING_NODE_ROLE = 'default-onboarding-node';

function joinedOrNone(values: string[], t: Translate): string {
  return values.length > 0 ? values.join(', ') : t('common:fallbacks.none');
}

/// manifest fetch 状態 (#356) から、settings 表示用の依存度 view を組み立てる。
///
/// - capability scope / authority scope / node role を行として表示する。
/// - identity / profile / social graph が node-owned ではないこと、default node が
///   network-wide authority ではないことを常に説明する（manifest が無くても表示）。
/// - manifest fetch 失敗時はエラーを表示し、default node へ fallback しない。
export function buildCommunityNodeDependencyView(
  entry: CommunityNodeManifestEntry | undefined,
  t: Translate
): CommunityNodeDependencyView {
  const status = entry?.status ?? 'loading';
  const diagnostics: CommunityNodeDependencyView['diagnostics'] = [
    {
      label: t('settings:communityNode.dependency.diagnostics.manifestStatus'),
      value: t(`settings:communityNode.dependency.status.${status}`),
      tone: status === 'error' ? 'danger' : 'default',
    },
  ];

  // identity / profile / social graph は node-owned ではない。常に説明する。
  const boundaryNotes = [t('settings:communityNode.dependency.boundary.identityNotOwned')];

  if (entry?.status === 'ok') {
    const manifest = entry.manifest;
    const isDefaultNode = manifest.node_role === DEFAULT_ONBOARDING_NODE_ROLE;

    diagnostics.push(
      {
        label: t('settings:communityNode.dependency.diagnostics.origin'),
        value: isDefaultNode
          ? t('settings:communityNode.dependency.origin.default')
          : t('settings:communityNode.dependency.origin.userAdded'),
      },
      {
        label: t('settings:communityNode.dependency.diagnostics.role'),
        value: manifest.node_role || t('common:fallbacks.none'),
        monospace: true,
      },
      {
        label: t('settings:communityNode.dependency.diagnostics.manifestVersion'),
        value: manifest.manifest_version || t('common:fallbacks.none'),
        monospace: true,
      },
      {
        label: t('settings:communityNode.dependency.diagnostics.capabilityAvailable'),
        value: joinedOrNone(manifest.capability_scope.available_enabled, t),
        monospace: true,
      },
      {
        label: t('settings:communityNode.dependency.diagnostics.capabilityPlanned'),
        value: joinedOrNone(manifest.capability_scope.planned_enabled, t),
        monospace: true,
      },
      {
        label: t('settings:communityNode.dependency.diagnostics.authorityAppliesTo'),
        value: joinedOrNone(manifest.authority_scope.applies_to, t),
        monospace: true,
      },
      {
        label: t('settings:communityNode.dependency.diagnostics.authorityDoesNotApplyTo'),
        value: joinedOrNone(manifest.authority_scope.does_not_apply_to, t),
        monospace: true,
      }
    );

    // default node であっても network-wide authority ではないことを明記する。
    if (isDefaultNode) {
      boundaryNotes.push(t('settings:communityNode.dependency.boundary.defaultNotAuthority'));
    }
  }

  return {
    diagnostics,
    boundaryNotes,
    manifestError: entry?.status === 'error' ? entry.error : null,
  };
}
