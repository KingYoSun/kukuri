import { useEffect, useMemo, useRef, useState } from 'react';

import type { DesktopApi, IndexEntryView } from '@/lib/api';
import { hasGatingContentAdvisory, isGatingContentAdvisory } from '@/shell/media';

/// #1055: 「見つける」の結果に含まれる content advisory（ADR 0028 §8.6）を表示に使える形へ揃える。
///
/// desktop-runtime が issuer 照合を済ませた advisory だけが届くため、ここでは採用可否を再判定しない。
/// このフックが持つのは表示のための 2 つだけ:
///
/// - 発行元 node の表示名。gating 対象の advisory を含む結果のときにだけ manifest を 1 回引く。
///   取得できなければ null を返し、呼出元は base URL の host へ落とす。
/// - 表示設定 OFF のためゲート中の添付 blob hash。呼出元がプリフェッチの除外集合として公開する。
export type CommunityIndexAdvisoryState = {
  /// manifest から解決した発行元の表示名。未解決・取得失敗は null。
  issuerNodeName: string | null;
  /// ゲート中の添付 blob hash。表示設定 ON では空。
  gatedMediaHashes: string[];
};

export function useCommunityIndexAdvisories({
  api,
  entries,
  nodeBaseUrl,
  adultContentEnabled,
}: {
  api: Pick<DesktopApi, 'fetchCommunityNodeManifest'>;
  entries: IndexEntryView[] | null;
  nodeBaseUrl: string | null;
  adultContentEnabled: boolean;
}): CommunityIndexAdvisoryState {
  const advisoryNodeBaseUrl =
    entries && nodeBaseUrl &&
    entries.some((entry) => hasGatingContentAdvisory(entry.content_advisories))
      ? nodeBaseUrl
      : null;

  const [resolvedName, setResolvedName] = useState<{
    baseUrl: string;
    nodeName: string | null;
  } | null>(null);
  const resolvedBaseUrl = resolvedName?.baseUrl ?? null;

  useEffect(() => {
    if (!advisoryNodeBaseUrl || typeof api.fetchCommunityNodeManifest !== 'function') {
      return;
    }
    if (resolvedBaseUrl === advisoryNodeBaseUrl) {
      return;
    }
    let active = true;
    void api
      .fetchCommunityNodeManifest(advisoryNodeBaseUrl)
      .then((response) => {
        if (!active) return;
        setResolvedName({
          baseUrl: advisoryNodeBaseUrl,
          nodeName: response.status === 'ok' ? response.manifest?.node_name?.trim() || null : null,
        });
      })
      .catch(() => {
        if (!active) return;
        setResolvedName({ baseUrl: advisoryNodeBaseUrl, nodeName: null });
      });
    return () => {
      active = false;
    };
  }, [advisoryNodeBaseUrl, resolvedBaseUrl, api]);

  const gatedMediaHashes = useMemo(() => {
    if (adultContentEnabled || !entries) return [];
    const hashes = new Set<string>();
    for (const entry of entries) {
      for (const advisory of entry.content_advisories ?? []) {
        if (!isGatingContentAdvisory(advisory)) continue;
        if (advisory.subject_kind !== 'blob_cid') continue;
        const hash = advisory.subject_id.trim();
        if (hash) hashes.add(hash);
      }
    }
    return [...hashes];
  }, [adultContentEnabled, entries]);

  return {
    issuerNodeName: resolvedBaseUrl === nodeBaseUrl ? (resolvedName?.nodeName ?? null) : null,
    gatedMediaHashes,
  };
}

/// 同じ hash 集合を繰り返し通知しないための publish helper（`onResolvedPostsChange` と同じ規則）。
export function usePublishedAdvisoryHashes(
  hashes: string[],
  onChange: ((hashes: string[]) => void) | undefined
) {
  const onChangeRef = useRef(onChange);
  useEffect(() => {
    onChangeRef.current = onChange;
  }, [onChange]);
  const published = useRef<string | null>(null);
  useEffect(() => {
    const signature = hashes.join('|');
    if (published.current === signature) return;
    published.current = signature;
    onChange?.(hashes);
  }, [hashes, onChange]);
  useEffect(
    () => () => {
      onChangeRef.current?.([]);
    },
    []
  );
}
