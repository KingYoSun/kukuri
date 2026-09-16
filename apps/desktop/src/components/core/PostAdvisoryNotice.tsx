import type { ReactNode } from 'react';
import { useTranslation } from 'react-i18next';

import {
  advisoryBasisLabel,
  advisoryCategoryLabel,
  advisoryIssuerLabel,
  shortenNodeId,
} from './contentAdvisoryPresentation';
import type { ContentAdvisoryView } from './types';

/// #1055: Community Node の content advisory による代替表示の説明。
///
/// 判定は issuer node の node-local な推定であり、投稿者の申告でも kukuri ネットワーク全体の
/// 判断でもない(ADR 0046 §6.3 / ADR 0027 §2.8)。断定表現を避け、発行元・分類・確信度・根拠を
/// 必ず一緒に示し、異議申し立てへの導線を添える。
export type PostAdvisoryNoticeProps = {
  advisory: ContentAdvisoryView;
  objectId: string;
  /// 異議申し立ての導線。通報操作が出せない文脈では未指定にして導線を出さない。
  appealAction?: ReactNode;
};

/// #858 / #1055: 表示設定 OFF の代替表示。本文の文言は判定元(投稿者の自己申告 / ノードの推定)で
/// 分け、canonical 解決前は呼出元が渡した待機文言を保つ。
export type PostGatedContentProps = {
  objectId: string;
  gatedBy?: 'self_label' | 'advisory';
  bodyText?: string | null;
  advisory?: ContentAdvisoryView | null;
  appealAction?: ReactNode;
};

export function PostGatedContent({
  objectId,
  gatedBy,
  bodyText,
  advisory,
  appealAction,
}: PostGatedContentProps) {
  const { t } = useTranslation(['common']);

  return (
    <>
      <p
        className='topic-diagnostic topic-diagnostic-secondary'
        role='status'
        data-testid={`post-adult-gated-${objectId}`}
      >
        {bodyText ??
          (gatedBy === 'advisory'
            ? t('feed.advisoryContentHidden')
            : t('feed.adultContentHidden'))}
      </p>
      {advisory ? (
        <PostAdvisoryNotice
          advisory={advisory}
          objectId={objectId}
          appealAction={appealAction}
        />
      ) : null}
    </>
  );
}

export function PostAdvisoryNotice({ advisory, objectId, appealAction }: PostAdvisoryNoticeProps) {
  const { t } = useTranslation(['common']);
  const issuerLabel = advisoryIssuerLabel(advisory);

  return (
    <div className='post-advisory-note' data-testid={`post-advisory-gated-${objectId}`}>
      <p className='post-advisory-title'>{t('advisory.title')}</p>
      <p className='topic-diagnostic topic-diagnostic-secondary'>
        {t('advisory.description', { node: issuerLabel })}
      </p>
      <dl className='post-advisory-facts'>
        <div>
          <dt>{t('advisory.issuer')}</dt>
          <dd data-testid={`post-advisory-issuer-${objectId}`}>
            {issuerLabel}
            <span className='post-advisory-issuer-id'>
              {shortenNodeId(advisory.issuerNodeId)}
            </span>
          </dd>
        </div>
        <div>
          <dt>{t('advisory.category')}</dt>
          <dd>{advisoryCategoryLabel(t, advisory.category)}</dd>
        </div>
        {typeof advisory.confidence === 'number' ? (
          <div>
            <dt>{t('advisory.confidence')}</dt>
            <dd>{t('advisory.confidenceValue', { value: advisory.confidence })}</dd>
          </div>
        ) : null}
        <div>
          <dt>{t('advisory.basis')}</dt>
          <dd>{advisoryBasisLabel(t, advisory.basis)}</dd>
        </div>
      </dl>
      {appealAction}
    </div>
  );
}
