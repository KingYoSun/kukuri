import { useEffect, useRef, useState } from 'react';

import type {
  CommunityNodeManifest,
  CommunityNodeManifestFetch,
  ContentProvenance,
} from '@/lib/api';

const EMPTY_MANIFESTS: Readonly<Record<string, CommunityNodeManifest>> = {};

export type ReportManifestsState = {
  /// 今回の通報画面で取得に成功したノード情報だけ。開くたびに空へ戻る。
  manifests: Readonly<Record<string, CommunityNodeManifest>>;
  resolving: boolean;
  resolveError: string | null;
};

/// 通報画面を開くたびに、観測元の基底アドレスから最新の `CommunityNodeManifest` を取得し、
/// 当該オープンで取得に成功した情報だけを候補源として返す(#666 / #696)。
///
/// - 設定画面などで以前取得した store 由来のノード情報は候補源にしない
/// - 取得失敗・情報なしのノードは候補源に入らない(古い候補へ後退しない)
/// - 閉じた後、または対象・観測元が変わった後に届いた応答は捨てる
export function useReportManifests(input: {
  open: boolean;
  provenance: ContentProvenance | undefined;
  fetchManifest?: (baseUrl: string) => Promise<CommunityNodeManifestFetch>;
}): ReportManifestsState {
  const { open, provenance, fetchManifest } = input;
  const [manifests, setManifests] = useState(EMPTY_MANIFESTS);
  const [resolving, setResolving] = useState(false);
  const [resolveError, setResolveError] = useState<string | null>(null);
  // 呼び出し側が毎描画で新しい関数を渡しても再取得を起こさないよう、参照だけを更新する。
  const fetchManifestRef = useRef(fetchManifest);
  useEffect(() => {
    fetchManifestRef.current = fetchManifest;
  });

  useEffect(() => {
    const fetchManifest = fetchManifestRef.current;
    setManifests(EMPTY_MANIFESTS);
    setResolveError(null);
    if (!open || !fetchManifest || !provenance) {
      setResolving(false);
      return;
    }
    const baseUrls = [...new Set(provenance.observedVia.map((item) => item.nodeBaseUrl))];
    if (baseUrls.length === 0) {
      setResolving(false);
      return;
    }
    let active = true;
    setResolving(true);
    Promise.all(
      baseUrls.map(async (baseUrl) => ({ baseUrl, response: await fetchManifest(baseUrl) }))
    )
      .then((responses) => {
        if (!active) return;
        const next: Record<string, CommunityNodeManifest> = {};
        let failed = false;
        for (const { baseUrl, response } of responses) {
          if (response.status === 'ok' && response.manifest) {
            next[baseUrl] = response.manifest;
          } else {
            failed = true;
          }
        }
        setManifests(next);
        if (failed) setResolveError('community node manifest is unavailable');
      })
      .catch((cause) => {
        if (active) setResolveError(cause instanceof Error ? cause.message : String(cause));
      })
      .finally(() => {
        if (active) setResolving(false);
      });
    return () => {
      active = false;
    };
  }, [open, provenance]);

  return { manifests, resolving, resolveError };
}
