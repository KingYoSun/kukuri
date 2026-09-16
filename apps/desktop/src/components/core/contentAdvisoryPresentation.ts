import type { TFunction } from 'i18next';

import type { Basis, SafetyCategory } from '@/lib/api';

import type { ContentAdvisoryView } from './types';

/// #1055: Community Node の content advisory を利用者へ説明するための表示変換。
/// 断定表現にせず、必ず発行元を伴って示す(ADR 0046 §6.3 / ADR 0027 §2.8)。

/// node_id は 64 桁の hex で、そのままではカード内に収まらない。先頭と末尾だけを出し、
/// 完全な値は通報 / 申し立て画面が扱う。
export function shortenNodeId(nodeId: string): string {
  const trimmed = nodeId.trim();
  if (trimmed.length <= 16) {
    return trimmed;
  }
  return `${trimmed.slice(0, 8)}…${trimmed.slice(-8)}`;
}

/// 発行元の表示名。manifest を取得できていればその名前を、できていなければ base URL の host を使う。
/// どちらも取れない場合だけ短縮 node_id へ落とす。
export function advisoryIssuerLabel(advisory: ContentAdvisoryView): string {
  const nodeName = advisory.nodeName?.trim();
  if (nodeName) {
    return nodeName;
  }
  const baseUrl = advisory.nodeBaseUrl.trim();
  if (baseUrl) {
    try {
      return new URL(baseUrl).host;
    } catch {
      return baseUrl.replace(/^https?:\/\//, '');
    }
  }
  return shortenNodeId(advisory.issuerNodeId);
}

/// 分類の表示語。未知のカテゴリは翻訳せず生の値を出す(node が語彙を増やしても壊れない)。
export function advisoryCategoryLabel(t: TFunction, category: SafetyCategory): string {
  const key = `advisory.categoryValue.${category}`;
  const translated = t(key, { defaultValue: '' });
  return translated || category;
}

/// 根拠の表示語。`classifier_score` 以外は現状 advisory に載らないが、未知値でも生の値を出す。
export function advisoryBasisLabel(t: TFunction, basis: Basis): string {
  const key = `advisory.basisValue.${basis}`;
  const translated = t(key, { defaultValue: '' });
  return translated || basis;
}
