// Content provenance and responsible-capability metadata (#358).
//
// kukuri では profile / social graph の canonical source は author docs + signed
// envelope であり、community node は truth source ではない。一方で検索結果・index・
// moderation label・trust signal・media cache・recommendation は特定 community node の
// capability に由来し得る。この差分を DTO に載せることで、#310 の分散通報ルーティングや
// dependency 表示が「どの source を正本とし、どの node capability を経由して観測したか」を
// 正しく扱える。
//
// 重要な不変条件:
// - `canonicalSource` と `observedVia` は分離する。community node 経由で観測したことは、
//   その node が content の truth source であることを意味しない。
// - provenance が不明な場合は `unknown` として表現し、通報先を default node /
//   kukuri project へ自動 fallback しない（`resolveReportTargets` は空配列を返す）。

/// content の正本（canonical source）。
///
/// `community_docs` は community node が保持する docs を指すが、これは observation 経路で
/// あって profile / social graph の truth source ではない点に注意する。
export type CanonicalSource =
  | 'author_docs'
  | 'community_docs'
  | 'blob'
  | 'local_cache'
  | 'external_bridge'
  | 'unknown';

/// content を現在の client へ観測・表示させた community node capability。
export type ObservedViaCapability =
  | 'bootstrap_assist'
  | 'relay_assist'
  | 'community_index'
  | 'moderation'
  | 'trust_signal'
  | 'media_cache'
  | 'recommendation'
  | 'bridge';

/// content の観測経路となった community node。truth source ではなく observation 経路。
export type ObservedViaNode = {
  nodeBaseUrl: string;
  capability: ObservedViaCapability;
  nodeRole?: string;
  manifestVersion?: string;
};

/// 通報先となり得る community node capability（#310 と整合）。
export type ReportTargetCapability =
  | 'community_index'
  | 'moderation'
  | 'trust_signal'
  | 'media_cache'
  | 'recommendation'
  | 'relay_assist'
  | 'bootstrap_assist';

/// 通報先候補。node manifest（#355/#356）から解決される。
export type ReportTarget = {
  nodeBaseUrl: string;
  nodeId?: string;
  capability: ReportTargetCapability;
  reportEndpoint?: string;
  abuseContact?: string;
  policyUrl?: string;
  authorityScope?: string[];
};

/// content / profile / media / search result / moderation label / recommendation などに
/// 付与する provenance metadata。
export type ContentProvenance = {
  /// 正本の種類。truth source を表す。
  canonicalSource: CanonicalSource;
  /// 観測・表示に関与した community node（複数可）。truth source ではない。
  observedVia: ObservedViaNode[];
  /// この content に関する責任ある通報先（実際に関与した node の capability に基づく）。
  responsibleReportTargets: ReportTarget[];
};

/// provenance を持ち得る DTO を表すユーティリティ型。
export type WithProvenance<T> = T & { provenance?: ContentProvenance };

/// provenance 不明を表す値。`observedVia` / `responsibleReportTargets` は空。
export function unknownProvenance(): ContentProvenance {
  return {
    canonicalSource: 'unknown',
    observedVia: [],
    responsibleReportTargets: [],
  };
}

/// provenance が不明かどうか。canonicalSource が unknown かつ観測経路が無い場合に true。
export function isProvenanceUnknown(provenance: ContentProvenance | null | undefined): boolean {
  if (!provenance) {
    return true;
  }
  return provenance.canonicalSource === 'unknown' && provenance.observedVia.length === 0;
}

/// 通報先を解決する。
///
/// 返すのは provenance が保持する `responsibleReportTargets` のみ。provenance が不明、または
/// 通報先が解決できない場合は **空配列** を返し、default node / kukuri project へ fallback
/// しない。これにより通報の中央集約を構造的に防ぐ（#310）。
export function resolveReportTargets(
  provenance: ContentProvenance | null | undefined,
): ReportTarget[] {
  if (!provenance) {
    return [];
  }
  return provenance.responsibleReportTargets;
}

/// 通報先を 1 つでも解決できるか。false の場合、client は local action（block / mute /
/// local hide）のみを案内し、通報先を default node へ向けない。
export function hasResolvableReportTarget(
  provenance: ContentProvenance | null | undefined,
): boolean {
  return resolveReportTargets(provenance).length > 0;
}

/// 通報先への連絡手段。reportEndpoint があれば POST、なければ abuseContact を mailto /
/// copyable contact として案内する（#310 の初期実装方針）。
export type ReportContact =
  | { kind: 'endpoint'; value: string }
  | { kind: 'contact'; value: string }
  | { kind: 'none' };

export function reportTargetContact(target: ReportTarget): ReportContact {
  if (target.reportEndpoint && target.reportEndpoint.trim().length > 0) {
    return { kind: 'endpoint', value: target.reportEndpoint };
  }
  if (target.abuseContact && target.abuseContact.trim().length > 0) {
    return { kind: 'contact', value: target.abuseContact };
  }
  return { kind: 'none' };
}

/// content details / diagnostics 表示用の provenance サマリ（非ローカライズの構造データ）。
/// UI 側で i18n する前提で、source / capability は key のまま返す。
export type ProvenanceSummary = {
  canonicalSource: CanonicalSource;
  unknown: boolean;
  observedVia: ObservedViaNode[];
  reportTargets: ReportTarget[];
  canReport: boolean;
};

export function summarizeProvenance(
  provenance: ContentProvenance | null | undefined,
): ProvenanceSummary {
  const resolved = provenance ?? unknownProvenance();
  return {
    canonicalSource: resolved.canonicalSource,
    unknown: isProvenanceUnknown(provenance),
    observedVia: resolved.observedVia,
    reportTargets: resolveReportTargets(provenance),
    canReport: hasResolvableReportTarget(provenance),
  };
}
